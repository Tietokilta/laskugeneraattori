use crate::{api::invoices::Invoice, error::Error};
use comemo::Prehashed;
use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
    path::PathBuf,
    sync::OnceLock,
};
use typst::{
    diag::{FileError, FileResult},
    eval::Tracer,
    foundations::{Bytes, Datetime, IntoValue, Value},
    model::Document,
    syntax::{FileId, Source, VirtualPath},
    text::{Font, FontBook},
    Library, World,
};

thread_local! {
    static WORLD: RefCell<Sandbox> = RefCell::new(Sandbox::new());
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
                let data = std::fs::read(&self.path).ok()?.into();
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
        let buffer = Bytes::from_static(data);
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
            bytes: bytes.into(),
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
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<FontSlot>,

    files: RefCell<HashMap<FileId, FileEntry>>,
    time: time::OffsetDateTime,
}

impl Sandbox {
    fn new() -> Self {
        let (book, fonts) = fonts();

        let new = Self {
            library: Prehashed::new(Library::builder().build()),
            book: Prehashed::new(book),
            fonts,
            source: Source::detached(include_str!("../../templates/invoice.typ")),
            time: time::OffsetDateTime::now_utc(),
            files: RefCell::new(HashMap::new()),
        };

        new.files.borrow_mut().insert(
            FileId::new(None, VirtualPath::new("/tik.png")),
            FileEntry::new(include_bytes!("../../templates/tik.png").to_vec(), None),
        );

        new
    }

    fn sandbox_file(&self, id: FileId) -> FileResult<RefMut<'_, FileEntry>> {
        if let Ok(entry) = RefMut::filter_map(self.files.borrow_mut(), |files| files.get_mut(&id)) {
            Ok(entry)
        } else {
            Err(FileError::NotFound(
                id.vpath().as_rootless_path().to_path_buf(),
            ))
        }
    }

    fn with_data(&self, data: impl IntoValue) -> Self {
        let mut new = self.clone();
        new.library.update(|l| {
            let scope = l.global.scope_mut();
            scope.define("data", data);
            scope.define("COMMIT_HASH", Value::Str(env!("COMMIT_HASH").into()));
            scope.define("VERSION", Value::Str(env!("CARGO_PKG_VERSION").into()));
        });
        new.time = time::OffsetDateTime::now_utc();
        new
    }
}

impl World for Sandbox {
    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.book
    }

    fn main(&self) -> Source {
        self.source.clone()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            self.sandbox_file(id)?.source(id)
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

impl TryInto<Document> for Invoice {
    type Error = Error;

    fn try_into(self) -> Result<Document, Error> {
        let w = WORLD.with_borrow(|w| w.with_data(self.clone()));
        self.attachments.into_iter().for_each(|a| {
            w.files.borrow_mut().insert(
                FileId::new(
                    None,
                    VirtualPath::new("/attachments/".to_owned() + &a.filename),
                ),
                FileEntry::new(a.bytes, None),
            );
        });

        let mut tracer = Tracer::default();
        let template = typst::compile(&w, &mut tracer).map_err(|e| {
            dbg!(e);
            Error::TypstError
        })?;

        Ok(template)
    }
}
