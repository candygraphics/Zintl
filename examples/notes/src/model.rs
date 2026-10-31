//! The note library, independent of the UI.

use zintl_ui_desktop::{AttributedText, ParagraphStyle};

use crate::storage::{FolderFile, LibraryFile, NoteFile};

pub type NoteId = u64;

/// Sidebar IDs of the folders the library manages itself.
pub struct SmartFolder;

impl SmartFolder {
    pub const ALL: &str = "all";
    pub const DELETED: &str = "deleted";
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub id: NoteId,
    pub folder: String,
    pub body: AttributedText,
    pub pinned: bool,
    pub deleted: bool,
    /// Increases on every edit; orders the list until real timestamps land.
    pub revision: u64,
}

impl Note {
    /// The first non-empty line, as Notes shows it in the list.
    pub fn title(&self) -> &str {
        self.lines().next().unwrap_or("New Note")
    }

    /// The first non-empty line after the title.
    pub fn preview(&self) -> &str {
        self.lines().nth(1).unwrap_or("No additional text")
    }

    fn lines(&self) -> impl Iterator<Item = &str> {
        self.body
            .text()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
    }
}

#[derive(Clone, Debug, Default)]
pub struct Library {
    pub folders: Vec<Folder>,
    pub notes: Vec<Note>,
    next_id: NoteId,
    clock: u64,
}

impl Library {
    pub fn sample() -> Self {
        let mut library = Self {
            folders: vec![
                Folder {
                    id: "notes".into(),
                    name: "Notes".into(),
                },
                Folder {
                    id: "recipes".into(),
                    name: "Recipes".into(),
                },
            ],
            ..Self::default()
        };
        let unchecked = ParagraphStyle::Checklist { done: false };
        let samples: [(&str, &str, &[ParagraphStyle], bool); 3] = [
            (
                "notes",
                "Welcome to Notes\nThis app is built with zintl-ui-desktop.",
                &[ParagraphStyle::Title],
                true,
            ),
            (
                "notes",
                "Groceries\nMilk\nEggs\nBread",
                &[
                    ParagraphStyle::Title,
                    unchecked,
                    ParagraphStyle::Checklist { done: true },
                    unchecked,
                ],
                false,
            ),
            (
                "recipes",
                "Curry\nOnion, carrot, potato, roux\nFry the onion\nAdd the roux",
                &[
                    ParagraphStyle::Title,
                    ParagraphStyle::Bulleted,
                    ParagraphStyle::Numbered,
                    ParagraphStyle::Numbered,
                ],
                false,
            ),
        ];
        for (folder, body, paragraphs, pinned) in samples {
            let id = library.create(folder);
            let mut body = AttributedText::from(body);
            for (index, paragraph) in paragraphs.iter().enumerate() {
                body.set_paragraph(index, *paragraph);
            }
            library.set_body(id, &body);
            library.set_pinned(id, pinned);
        }
        library
    }

    /// Describes the library for saving.
    pub fn to_file(&self) -> LibraryFile {
        LibraryFile {
            version: LibraryFile::VERSION,
            folders: self
                .folders
                .iter()
                .map(|folder| FolderFile {
                    id: folder.id.clone(),
                    name: folder.name.clone(),
                })
                .collect(),
            notes: self
                .notes
                .iter()
                .map(|note| NoteFile {
                    id: note.id,
                    folder: note.folder.clone(),
                    body: (&note.body).into(),
                    pinned: note.pinned,
                    deleted: note.deleted,
                    revision: note.revision,
                })
                .collect(),
        }
    }

    /// Restores a saved library; new IDs and revisions continue after the saved ones.
    pub fn from_file(file: LibraryFile) -> Self {
        let notes: Vec<Note> = file
            .notes
            .into_iter()
            .map(|note| Note {
                id: note.id,
                folder: note.folder,
                body: note.body.into(),
                pinned: note.pinned,
                deleted: note.deleted,
                revision: note.revision,
            })
            .collect();
        Self {
            folders: file
                .folders
                .into_iter()
                .map(|folder| Folder {
                    id: folder.id,
                    name: folder.name,
                })
                .collect(),
            next_id: notes.iter().map(|note| note.id).max().unwrap_or(0),
            clock: notes.iter().map(|note| note.revision).max().unwrap_or(0),
            notes,
        }
    }

    pub fn note(&self, id: NoteId) -> Option<&Note> {
        self.notes.iter().find(|note| note.id == id)
    }

    /// Notes outside Recently Deleted whose text contains `query`, ignoring case.
    pub fn search(&self, query: &str) -> Vec<&Note> {
        let query = query.to_lowercase();
        self.notes_in(SmartFolder::ALL)
            .into_iter()
            .filter(|note| note.body.text().to_lowercase().contains(&query))
            .collect()
    }

    /// Notes shown for a sidebar selection: pinned first, then most recently edited.
    pub fn notes_in(&self, folder: &str) -> Vec<&Note> {
        let mut notes: Vec<&Note> = self
            .notes
            .iter()
            .filter(|note| match folder {
                SmartFolder::ALL => !note.deleted,
                SmartFolder::DELETED => note.deleted,
                id => !note.deleted && note.folder == id,
            })
            .collect();
        notes.sort_by(|a, b| b.pinned.cmp(&a.pinned).then(b.revision.cmp(&a.revision)));
        notes
    }

    /// Creates an empty note; smart folders put it in the first folder.
    pub fn create(&mut self, folder: &str) -> NoteId {
        let folder = match folder {
            SmartFolder::ALL | SmartFolder::DELETED => self
                .folders
                .first()
                .map(|folder| folder.id.clone())
                .unwrap_or_default(),
            id => id.to_owned(),
        };
        self.next_id += 1;
        let note = Note {
            id: self.next_id,
            folder,
            body: AttributedText::new(),
            pinned: false,
            deleted: false,
            revision: self.tick(),
        };
        self.notes.push(note);
        self.next_id
    }

    /// Stores an edit; unchanged bodies keep their position in the list.
    pub fn set_body(&mut self, id: NoteId, body: &AttributedText) {
        let revision = self.clock + 1;
        if let Some(note) = self.notes.iter_mut().find(|note| note.id == id)
            && note.body != *body
        {
            note.body = body.clone();
            note.revision = revision;
            self.clock = revision;
        }
    }

    pub fn set_pinned(&mut self, id: NoteId, pinned: bool) {
        if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
            note.pinned = pinned;
        }
    }

    /// Moves a note to Recently Deleted, or removes it if it is already there.
    pub fn delete(&mut self, id: NoteId) {
        let Some(index) = self.notes.iter().position(|note| note.id == id) else {
            return;
        };
        if self.notes[index].deleted {
            self.notes.remove(index);
        } else {
            self.notes[index].deleted = true;
            self.notes[index].pinned = false;
        }
    }

    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Titles and previews skip blank lines and fall back for empty notes.
    #[test]
    fn title_and_preview_use_non_empty_lines() {
        let mut library = Library::default();
        let id = library.create("notes");
        assert_eq!(library.note(id).unwrap().title(), "New Note");
        library.set_body(id, &"\n  Title  \n\nBody".into());
        let note = library.note(id).unwrap();
        assert_eq!((note.title(), note.preview()), ("Title", "Body"));
    }

    // Pinned notes come first, then the most recently edited.
    #[test]
    fn notes_are_sorted_by_pin_then_recency() {
        let mut library = Library::default();
        let a = library.create("notes");
        let b = library.create("notes");
        let c = library.create("notes");
        library.set_body(a, &"edited".into());
        library.set_pinned(c, true);
        let order: Vec<_> = library.notes_in("notes").iter().map(|n| n.id).collect();
        assert_eq!(order, [c, a, b]);
    }

    // Saving the same body does not move the note to the top.
    #[test]
    fn unchanged_body_keeps_order() {
        let mut library = Library::default();
        let a = library.create("notes");
        let b = library.create("notes");
        library.set_body(a, &AttributedText::new());
        assert_eq!(library.notes_in("notes")[0].id, b);
    }

    // Search matches any note's text without regard to case and skips deleted notes.
    #[test]
    fn search_ignores_case_and_deleted_notes() {
        let mut library = Library::sample();
        let ids: Vec<_> = library.search("ROUX").iter().map(|note| note.id).collect();
        assert_eq!(ids.len(), 1);
        library.delete(ids[0]);
        assert!(library.search("roux").is_empty());
    }

    // Deleting moves a note to Recently Deleted; deleting it there removes it.
    #[test]
    fn delete_moves_then_removes() {
        let mut library = Library::default();
        let id = library.create("notes");
        library.delete(id);
        assert!(library.notes_in(SmartFolder::ALL).is_empty());
        assert_eq!(library.notes_in(SmartFolder::DELETED).len(), 1);
        library.delete(id);
        assert!(library.note(id).is_none());
    }

    // New notes from a smart folder land in the first real folder.
    #[test]
    fn create_in_smart_folder_uses_first_folder() {
        let mut library = Library::sample();
        let id = library.create(SmartFolder::ALL);
        assert_eq!(library.note(id).unwrap().folder, "notes");
    }
}
