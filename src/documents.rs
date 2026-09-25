use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone)]
pub struct Document {
    pub id: u64,
    pub path: Option<PathBuf>,
    pub source: String,
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
        Self {
            id: 0,
            seen_mtime: path
                .as_ref()
                .and_then(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok()),
            path,
            saved_source: source.clone(),
            source,
            dirty: false,
            edit_revision: 0,
            editing: false,
            scroll: 0.0,
            editor_scroll: 0.0,
            selection_start: 0,
            selection_end: 0,
        }
    }
    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("Untitled {}", self.id))
    }
    pub fn edit(&mut self, source: String) {
        self.source = source;
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
            .find(|d| d.path.as_deref() == Some(path))
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
    pub fn replace(&mut self, mut document: Document) {
        document.id = self.next_id;
        self.next_id += 1;
        self.tabs[self.active] = document;
    }
    // The caller must resolve unsaved changes before removing a document.
    pub fn remove(&mut self, id: u64) {
        if let Some(index) = self.tabs.iter().position(|d| d.id == id) {
            let active_id = self.current().id;
            self.tabs.remove(index);
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
        }
    }
}
