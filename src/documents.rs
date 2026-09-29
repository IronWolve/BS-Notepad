use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone)]
pub struct Document {
    pub pending_fragment: String,
    pub unloaded: bool,
    pub loading: bool,
    pub load_error: Option<String>,
    pub image: Option<crate::storage::ImageInfo>,
    pub image_view: Option<crate::settings::ImageView>,
    pub recovery_key: String,
    pub format: crate::storage::TextFormat,
    pub fingerprint: Option<u64>,
    pub read_only: bool,
    pub write_protected: bool,
    pub external_changed: bool,
    pub disk: crate::disk::State,
    pub id: u64,
    pub path: Option<PathBuf>,
    pub source: String,
    pub patch_index: crate::storage::PatchIndex,
    pub saved_source: String,
    pub seen_mtime: Option<SystemTime>,
    pub dirty: bool,
    pub edit_revision: u64,
    pub editing: bool,
    pub scroll: f32,
    pub editor_scroll: f64,
    pub selection_start: u64,
    pub selection_end: u64,
}
impl Document {
    pub fn new(path: Option<PathBuf>, source: String) -> Self {
        let format = crate::storage::TextFormat::from_text(&source);
        let fingerprint = path
            .as_ref()
            .map(|_| crate::storage::fingerprint(source.as_bytes()));
        let source = crate::storage::normalize(&source);
        Self {
            pending_fragment: String::new(),
            unloaded: false,
            loading: false,
            load_error: None,
            image: None,
            image_view: None,
            recovery_key: crate::recovery::key(),
            format,
            fingerprint,
            read_only: false,
            write_protected: false,
            external_changed: false,
            disk: Default::default(),
            id: 0,
            seen_mtime: None,
            path,
            saved_source: source.clone(),
            source,
            patch_index: Default::default(),
            dirty: false,
            edit_revision: 0,
            editing: false,
            scroll: 0.0,
            editor_scroll: 0.0,
            selection_start: 0,
            selection_end: 0,
        }
    }
    pub fn loaded(path: PathBuf, loaded: crate::storage::Loaded) -> Self {
        let mut doc = Self::new(None, String::new());
        doc.saved_source = loaded.source.clone();
        doc.source = loaded.source;
        doc.path = Some(path);
        doc.seen_mtime = loaded.modified;
        doc.format = loaded.format;
        doc.fingerprint = if loaded.image.is_some() {
            None
        } else {
            Some(loaded.fingerprint)
        };
        doc.image = loaded.image;
        doc.read_only = loaded.read_only;
        doc.write_protected = loaded.write_protected;
        doc
    }
    pub fn deferred(view: &crate::settings::SessionTab) -> Self {
        let mut doc = Self::new(None, String::new());
        doc.path = Some(crate::paths::display_form(Path::new(&view.path)));
        doc.unloaded = true;
        doc.read_only = true;
        doc.apply_view(view);
        doc
    }
    pub fn apply_view(&mut self, view: &crate::settings::SessionTab) {
        self.scroll = view.scroll;
        self.editing = view.editing;
        self.editor_scroll = view.editor_scroll;
        self.selection_start = view.selection_start;
        self.selection_end = view.selection_end;
        self.image_view = view.image_view.clone();
    }
    pub fn view(&self) -> Option<crate::settings::SessionTab> {
        Some(crate::settings::SessionTab {
            path: self.path.as_ref()?.to_str()?.to_owned(),
            scroll: self.scroll,
            editing: self.editing,
            editor_scroll: self.editor_scroll,
            selection_start: self.selection_start,
            selection_end: self.selection_end,
            image_view: self.image_view.clone(),
        })
    }
    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("Untitled {}", self.id))
    }
    pub fn accepts_save(&self, text: &str) -> bool {
        !self.read_only && text == self.source
    }

    pub fn edit(&mut self, source: String) {
        self.source = source;
        self.patch_index = Default::default();
        self.dirty = self.source != self.saved_source;
    }
}

pub struct Documents {
    pub tabs: Vec<Document>,
    active: usize,
    next_id: u64,
}
impl Documents {
    pub fn new(mut first: Document) -> Self {
        first.id = 1;
        Self {
            tabs: vec![first],
            active: 0,
            next_id: 2,
        }
    }
    pub fn current(&self) -> &Document {
        &self.tabs[self.active]
    }
    pub fn current_mut(&mut self) -> &mut Document {
        &mut self.tabs[self.active]
    }
    pub fn get_mut(&mut self, id: u64) -> Option<&mut Document> {
        self.tabs.iter_mut().find(|d| d.id == id)
    }
    pub fn find_path(&self, path: &Path) -> Option<u64> {
        self.tabs
            .iter()
            .find(|d| {
                d.path
                    .as_deref()
                    .is_some_and(|p| crate::paths::same(p, path))
            })
            .map(|d| d.id)
    }
    pub fn activate(&mut self, id: u64) -> bool {
        if let Some(index) = self.tabs.iter().position(|d| d.id == id) {
            self.active = index;
            true
        } else {
            false
        }
    }
    pub fn insert(&mut self, mut document: Document) {
        document.id = self.next_id;
        self.next_id += 1;
        self.tabs.push(document);
        self.active = self.tabs.len() - 1;
    }
    pub fn replace(&mut self, mut document: Document) -> Document {
        document.id = self.next_id;
        self.next_id += 1;
        std::mem::replace(&mut self.tabs[self.active], document)
    }
    pub fn move_before(&mut self, id: u64, before: Option<u64>) -> bool {
        if before == Some(id)
            || before.is_some_and(|target| !self.tabs.iter().any(|doc| doc.id == target))
        {
            return false;
        }
        let Some(index) = self.tabs.iter().position(|doc| doc.id == id) else {
            return false;
        };
        let active = self.current().id;
        let doc = self.tabs.remove(index);
        let destination = before
            .and_then(|target| self.tabs.iter().position(|doc| doc.id == target))
            .unwrap_or(self.tabs.len());
        self.tabs.insert(destination, doc);
        self.active = self.tabs.iter().position(|doc| doc.id == active).unwrap();
        true
    }

    // The caller must resolve unsaved changes before removing a document.
    pub fn remove(&mut self, id: u64) {
        let _ = self.take(id);
    }
    pub fn take(&mut self, id: u64) -> Option<Document> {
        if let Some(index) = self.tabs.iter().position(|d| d.id == id) {
            let active_id = self.current().id;
            let removed = self.tabs.remove(index);
            if self.tabs.is_empty() {
                let mut blank = Document::new(None, String::new());
                blank.editing = true;
                self.insert(blank);
            } else {
                self.active = self
                    .tabs
                    .iter()
                    .position(|d| d.id == active_id)
                    .unwrap_or(index.min(self.tabs.len() - 1));
            }
            Some(removed)
        } else {
            None
        }
    }
}
