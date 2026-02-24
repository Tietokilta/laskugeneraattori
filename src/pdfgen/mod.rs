use crate::api::invoices::InvoiceAttachment;
use crate::api::receipts::Receipt;
use crate::{api::invoices::Invoice, error::Error};
use bank_barcode::{Barcode, BarcodeBuilder};
use std::sync::LazyLock;
use std::{collections::HashMap, path::PathBuf, sync::OnceLock};
use time_tz::{timezones, ToTimezone};
use typst::{
    diag::{FileError, FileResult},
    foundations::{Bytes, Datetime, IntoValue, Value},
    layout::PagedDocument,
    syntax::{FileId, Source, VirtualPath},
    text::{Font, FontBook},
    utils::LazyHash,
    Library, World,
};

static WORLD: LazyLock<Sandbox> = LazyLock::new(Sandbox::new);

enum Template {
    Invoice,
    Receipt,
}

#[derive(Clone, Debug)]
pub struct FontSlot {
    path: PathBuf,
    index: u32,
    font: OnceLock<Option<Font>>,
}

impl FontSlot {
    pub fn get(&self) -> Option<Font> {
        self.font
            .get_or_init(|| {
                let data = Bytes::new(std::fs::read(&self.path).ok()?);
                Font::new(data, self.index)
            })
            .clone()
    }
}

fn fonts() -> (FontBook, Vec<FontSlot>) {
    #[cfg(feature = "system_fonts")]
    let mut db = fontdb::Database::new();
    #[cfg(feature = "system_fonts")]
    db.load_system_fonts();

    let mut book = FontBook::new();
    let mut fonts = Vec::new();

    #[cfg(feature = "system_fonts")]
    for face in db.faces() {
        let path = match &face.source {
            fontdb::Source::File(path) | fontdb::Source::SharedFile(path, _) => path,
            _ => continue,
        };

        let info = db
            .with_face_data(face.id, typst::text::FontInfo::new)
            .expect("bug: impossible");

        if let Some(info) = info {
            book.push(info);
            fonts.push(FontSlot {
                path: path.clone(),
                index: face.index,
                font: OnceLock::new(),
            });
        }
    }

    for data in typst_assets::fonts() {
        let buffer = Bytes::new(data);
        for (i, font) in Font::iter(buffer).enumerate() {
            book.push(font.info().clone());
            fonts.push(FontSlot {
                path: PathBuf::new(),
                index: i as u32,
                font: OnceLock::from(Some(font)),
            })
        }
    }
    (book, fonts)
}

#[derive(Clone, Debug)]
struct FileEntry {
    bytes: Bytes,
    source: Option<Source>,
}

impl FileEntry {
    fn new(bytes: Vec<u8>, source: Option<Source>) -> Self {
        Self {
            bytes: Bytes::new(bytes),
            source,
        }
    }

    fn source(&mut self, id: FileId) -> FileResult<Source> {
        let source = if let Some(source) = &self.source {
            source
        } else {
            let contents = std::str::from_utf8(&self.bytes).map_err(|_| FileError::InvalidUtf8)?;
            let contents = contents.trim_start_matches('\u{feff}');
            let source = Source::new(id, contents.into());
            self.source.insert(source)
        };
        Ok(source.clone())
    }
}

#[derive(Debug, Clone)]
struct Sandbox {
    source: Source,
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<FontSlot>,

    files: HashMap<FileId, FileEntry>,
    time: time::OffsetDateTime,
}

impl Sandbox {
    fn new() -> Self {
        let (book, fonts) = fonts();

        let mut new = Self {
            library: LazyHash::new(Library::builder().build()),
            book: LazyHash::new(book),
            fonts,
            source: Source::detached(include_str!("../../templates/invoice.typ")),
            time: time::OffsetDateTime::now_utc(),
            files: HashMap::new(),
        };

        new.files.insert(
            FileId::new(None, VirtualPath::new("/tik.png")),
            FileEntry::new(include_bytes!("../../templates/tik.png").to_vec(), None),
        );

        new
    }
    fn with_template(&self, template: Template) -> Self {
        let mut new = self.clone();
        let src = match template {
            Template::Invoice => include_str!("../../templates/invoice.typ"),
            Template::Receipt => include_str!("../../templates/receipt.typ"),
        };
        new.source = Source::detached(src);
        new
    }

    fn sandbox_file(&self, id: FileId) -> FileResult<&FileEntry> {
        if let Some(entry) = self.files.get(&id) {
            Ok(entry)
        } else {
            Err(FileError::NotFound(
                id.vpath().as_rootless_path().to_path_buf(),
            ))
        }
    }

    fn with_data(&self, data: impl IntoValue) -> Self {
        let mut new = self.clone();
        let scope = new.library.global.scope_mut();
        scope.define("data", data);
        scope.define("COMMIT_HASH", Value::Str(env!("COMMIT_HASH").into()));
        scope.define("VERSION", Value::Str(env!("CARGO_PKG_VERSION").into()));

        new.time = time::OffsetDateTime::now_utc();
        new
    }
}

impl World for Sandbox {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            self.sandbox_file(id)?.clone().source(id)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.sandbox_file(id).map(|file| file.bytes.clone())
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index)?.get()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        let time = self.time.to_timezone(timezones::db::europe::HELSINKI);

        Datetime::from_ymd_hms(
            time.year(),
            time.month() as u8,
            time.day(),
            time.hour(),
            time.minute(),
            time.second(),
        )
    }
}

/// Serialize a value to JSON and then deserialize into a typst `Value`.
fn to_typst_value(value: &impl serde::Serialize) -> Value {
    let json = serde_json::to_string(value).expect("BUG: serialization failed");
    serde_json::from_str(&json).expect("BUG: failed to deserialize into typst::Value")
}

/// Compile a `Sandbox` world into a `PagedDocument`, mapping typst diagnostics to `Error`.
fn compile_world(w: &Sandbox) -> Result<PagedDocument, Error> {
    let typst::diag::Warned {
        output,
        warnings: _,
    } = typst::compile(w);

    output.map_err(|err| {
        Error::TypstError(
            err.into_iter()
                .map(|e| e.message.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    })
}

impl TryFrom<Invoice> for Barcode {
    type Error = bank_barcode::BuilderError;

    fn try_from(invoice: Invoice) -> Result<Self, Self::Error> {
        BarcodeBuilder::v4()
            .account_number(&invoice.bank_account_number)
            .sum(invoice.rows.iter().map(|row| row.unit_price as u32).sum())
            .build()
    }
}

pub struct InvoiceBuilder {
    invoice: Invoice,
    attachments: Vec<InvoiceAttachment>,
}

impl InvoiceBuilder {
    pub fn new(invoice: Invoice, attachments: Vec<InvoiceAttachment>) -> Self {
        Self {
            invoice,
            attachments,
        }
    }

    fn data(&self) -> Value {
        let mut value = to_typst_value(&self.invoice);

        // Inject the barcode field into the typst value
        if let Value::Dict(ref mut dict) = value {
            let barcode = Barcode::try_from(self.invoice.clone())
                .map(|b| b.to_string())
                .unwrap_or_default();
            dict.insert("barcode".into(), Value::Str(barcode.into()));
        }

        value
    }

    #[allow(dead_code)]
    pub fn build(self) -> Result<PagedDocument, Error> {
        self.build_with_pdfs().map(|(doc, _)| doc)
    }

    pub fn build_with_pdfs(self) -> Result<(PagedDocument, Vec<InvoiceAttachment>), Error> {
        let mut w = WORLD
            .clone()
            .with_template(Template::Invoice)
            .with_data(self.data());

        let pdfs = self
            .attachments
            .into_iter()
            .filter_map(|a| {
                if a.filename.to_lowercase().ends_with(".pdf") {
                    Some(a)
                } else {
                    w.files.insert(
                        FileId::new(
                            None,
                            VirtualPath::new(format!("/attachments/{}", a.filename)),
                        ),
                        FileEntry::new(a.bytes, None),
                    );
                    return None;
                }
            })
            .collect::<Vec<_>>();

        let document = compile_world(&w)?;
        Ok((document, pdfs))
    }
}

pub struct ReceiptBuilder {
    receipt: Receipt,
}

impl ReceiptBuilder {
    pub fn new(receipt: Receipt) -> Self {
        Self { receipt }
    }

    pub fn build(self) -> Result<PagedDocument, Error> {
        let w = WORLD
            .clone()
            .with_template(Template::Receipt)
            .with_data(to_typst_value(&self.receipt));

        compile_world(&w)
    }
}
