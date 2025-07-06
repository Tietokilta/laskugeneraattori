use crate::api::invoices::InvoiceAttachment;
use crate::{api::invoices::Invoice, error::Error};
use bank_barcode::{Barcode, BarcodeBuilder};
use std::sync::LazyLock;
use std::{collections::HashMap, path::PathBuf, sync::OnceLock};
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

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let offset = offset.unwrap_or(0);
        let offset = time::UtcOffset::from_hms(offset.try_into().ok()?, 0, 0).ok()?;
        let time = self.time.checked_to_offset(offset)?;
        Some(Datetime::Date(time.date()))
    }
}

impl IntoValue for Invoice {
    fn into_value(self) -> typst::foundations::Value {
        serde_json::from_str(&serde_json::to_string(&self).unwrap()).unwrap()
    }
}

impl TryInto<PagedDocument> for Invoice {
    type Error = Error;

    fn try_into(self) -> Result<PagedDocument, Error> {
        let mut w = WORLD.clone().with_data(self.clone());
        self.attachments.into_iter().for_each(|a| {
            w.files.insert(
                FileId::new(
                    None,
                    VirtualPath::new("/attachments/".to_owned() + &a.filename),
                ),
                FileEntry::new(a.bytes, None),
            );
        });

        let typst::diag::Warned {
            output,
            warnings: _,
        } = typst::compile(&w);

        match output {
            Ok(template) => Ok(template),
            Err(err) => Err(Error::TypstError(
                err.into_iter()
                    .map(|e| e.message.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            )),
        }
    }
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

pub struct DocumentBuilder {
    invoice: Invoice,
    attachments: Vec<InvoiceAttachment>,
}

impl DocumentBuilder {
    pub fn new(invoice: Invoice, attachments: Vec<InvoiceAttachment>) -> Self {
        Self {
            invoice,
            attachments,
        }
    }

    // FIXME: this is very ugly
    fn data(&self) -> Value {
        let mut value: serde_json::Value = serde_json::from_str(
            serde_json::to_string(&self.invoice)
                .expect("BUG: serializing invoice failed")
                .as_str(),
        )
        .expect("BUG: deserializing invoice failed");

        let barcode = Barcode::try_from(self.invoice.clone());

        value["barcode"] = barcode
            .map(|barcode| barcode.to_string())
            .unwrap_or_default()
            .into();

        serde_json::from_str(&value.to_string())
            .expect("BUG: failed to deserialize into typst::Value")
    }

    #[allow(dead_code)]
    pub fn build(self) -> Result<PagedDocument, Error> {
        self.build_with_pdfs().map(|(doc, _)| doc)
    }

    pub fn build_with_pdfs(self) -> Result<(PagedDocument, Vec<InvoiceAttachment>), Error> {
        let mut w = WORLD.clone().with_data(self.data());

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
                    None
                }
            })
            .collect::<Vec<_>>();

        let typst::diag::Warned {
            output,
            warnings: _,
        } = typst::compile(&w);

        match output {
            Ok(template) => Ok((template, pdfs)),
            Err(err) => Err(Error::TypstError(
                err.into_iter()
                    .map(|e| e.message.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            )),
        }
    }
}
