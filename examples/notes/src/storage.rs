//! Saves the library as JSON in the user's application data folder.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use serde::{Deserialize, Serialize};
use zintl_ui_desktop::{AttributedText, ParagraphStyle, SpanStyle};

use crate::model::Library;

/// The saved library; `version` changes when the layout does.
#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryFile {
    pub version: u32,
    pub folders: Vec<FolderFile>,
    pub notes: Vec<NoteFile>,
}

impl LibraryFile {
    pub const VERSION: u32 = 1;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FolderFile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoteFile {
    pub id: u64,
    pub folder: String,
    pub body: TextFile,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub deleted: bool,
    pub revision: u64,
}

/// Rich text as its runs and one style per paragraph.
#[derive(Debug, Serialize, Deserialize)]
pub struct TextFile {
    pub runs: Vec<RunFile>,
    pub paragraphs: Vec<ParagraphFile>,
}

impl From<&AttributedText> for TextFile {
    fn from(text: &AttributedText) -> Self {
        Self {
            runs: text
                .runs()
                .map(|(text, style)| RunFile {
                    text: text.to_owned(),
                    bold: style.bold,
                    italic: style.italic,
                    underline: style.underline,
                    strikethrough: style.strikethrough,
                    monospace: style.monospace,
                    link: style.link.clone(),
                })
                .collect(),
            paragraphs: text
                .paragraphs()
                .iter()
                .map(|style| (*style).into())
                .collect(),
        }
    }
}

impl From<TextFile> for AttributedText {
    fn from(file: TextFile) -> Self {
        let mut text = AttributedText::new();
        for run in file.runs {
            let style = SpanStyle {
                bold: run.bold,
                italic: run.italic,
                underline: run.underline,
                strikethrough: run.strikethrough,
                monospace: run.monospace,
                link: run.link,
            };
            text.push(&run.text, style);
        }
        for (index, paragraph) in file.paragraphs.into_iter().enumerate() {
            text.set_paragraph(index, paragraph.into());
        }
        text
    }
}

/// A run of text; formatting flags are omitted when off.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunFile {
    pub text: String,
    #[serde(default, skip_serializing_if = "RunFile::is_off")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "RunFile::is_off")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "RunFile::is_off")]
    pub underline: bool,
    #[serde(default, skip_serializing_if = "RunFile::is_off")]
    pub strikethrough: bool,
    #[serde(default, skip_serializing_if = "RunFile::is_off")]
    pub monospace: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

impl RunFile {
    fn is_off(value: &bool) -> bool {
        !value
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParagraphFile {
    Title,
    Heading,
    Subheading,
    Body,
    Monospaced,
    Bulleted,
    Dashed,
    Numbered,
    Checklist { done: bool },
}

impl From<ParagraphStyle> for ParagraphFile {
    fn from(style: ParagraphStyle) -> Self {
        match style {
            ParagraphStyle::Title => Self::Title,
            ParagraphStyle::Heading => Self::Heading,
            ParagraphStyle::Subheading => Self::Subheading,
            ParagraphStyle::Body => Self::Body,
            ParagraphStyle::Monospaced => Self::Monospaced,
            ParagraphStyle::Bulleted => Self::Bulleted,
            ParagraphStyle::Dashed => Self::Dashed,
            ParagraphStyle::Numbered => Self::Numbered,
            ParagraphStyle::Checklist { done } => Self::Checklist { done },
        }
    }
}

impl From<ParagraphFile> for ParagraphStyle {
    fn from(file: ParagraphFile) -> Self {
        match file {
            ParagraphFile::Title => Self::Title,
            ParagraphFile::Heading => Self::Heading,
            ParagraphFile::Subheading => Self::Subheading,
            ParagraphFile::Body => Self::Body,
            ParagraphFile::Monospaced => Self::Monospaced,
            ParagraphFile::Bulleted => Self::Bulleted,
            ParagraphFile::Dashed => Self::Dashed,
            ParagraphFile::Numbered => Self::Numbered,
            ParagraphFile::Checklist { done } => Self::Checklist { done },
        }
    }
}

/// Work for the writer thread.
enum Job {
    Save(String),
    /// Replies once every earlier save is on disk.
    Flush(Sender<()>),
}

/// The library file and the thread that writes it.
pub struct Storage;

impl Storage {
    const FILE: &str = "notes.json";

    /// Returns the folder for app data; `ZINTL_NOTES_DIR` overrides it.
    pub fn directory() -> PathBuf {
        if let Some(directory) = std::env::var_os("ZINTL_NOTES_DIR") {
            return directory.into();
        }
        #[cfg(target_os = "windows")]
        let base = std::env::var_os("APPDATA").map(PathBuf::from);
        #[cfg(not(target_os = "windows"))]
        let base = std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join("Library/Application Support"));
        base.unwrap_or_else(std::env::temp_dir).join("zintl-notes")
    }

    /// Reads the library in `directory`; `Ok(None)` means nothing is saved yet.
    ///
    /// An unreadable file is renamed aside so a fresh library cannot overwrite it.
    pub fn load(directory: &Path) -> io::Result<Option<Library>> {
        let path = directory.join(Self::FILE);
        let json = match fs::read_to_string(&path) {
            Ok(json) => json,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        match serde_json::from_str::<LibraryFile>(&json) {
            Ok(file) => Ok(Some(Library::from_file(file))),
            Err(error) => {
                fs::rename(&path, directory.join("notes.unreadable.json"))?;
                eprintln!("notes: moved an unreadable library aside: {error}");
                Ok(None)
            }
        }
    }

    /// Starts the writer thread for `directory`.
    pub fn writer(directory: PathBuf) -> Saver {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || Self::write_jobs(&directory, receiver));
        Saver { sender }
    }

    /// Writes the newest pending save, then answers flushes in order.
    fn write_jobs(directory: &Path, receiver: Receiver<Job>) {
        let mut pending = None;
        while let Ok(job) = receiver.recv() {
            let mut jobs = vec![job];
            jobs.extend(receiver.try_iter());
            for job in jobs {
                match job {
                    Job::Save(json) => pending = Some(json),
                    Job::Flush(done) => {
                        Self::write(directory, pending.take());
                        let _ = done.send(());
                    }
                }
            }
            Self::write(directory, pending.take());
        }
    }

    /// Replaces the file atomically, so a crash leaves the previous version.
    fn write(directory: &Path, json: Option<String>) {
        let Some(json) = json else {
            return;
        };
        let temporary = directory.join("notes.json.tmp");
        let result = fs::create_dir_all(directory)
            .and_then(|()| fs::write(&temporary, json))
            .and_then(|()| fs::rename(&temporary, directory.join(Self::FILE)));
        if let Err(error) = result {
            eprintln!("notes: could not save: {error}");
        }
    }
}

/// Sends snapshots to the writer thread; clones share the thread.
#[derive(Clone)]
pub struct Saver {
    sender: Sender<Job>,
}

impl Saver {
    /// Queues `library` to be written; only the newest queued snapshot is written.
    pub fn save(&self, library: &Library) {
        match serde_json::to_string_pretty(&library.to_file()) {
            Ok(json) => {
                let _ = self.sender.send(Job::Save(json));
            }
            Err(error) => eprintln!("notes: could not encode the library: {error}"),
        }
    }

    /// Waits until every queued snapshot is on disk.
    pub fn flush(&self) {
        let (done, wait) = mpsc::channel();
        if self.sender.send(Job::Flush(done)).is_ok() {
            let _ = wait.recv();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns an empty folder unique to `name`.
    fn folder(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("zintl-notes-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        directory
    }

    // A saved library loads back with the same notes, styles, and IDs.
    #[test]
    fn library_round_trips_through_the_file() {
        let directory = folder("round-trip");
        let library = Library::sample();
        let saver = Storage::writer(directory.clone());
        saver.save(&library);
        saver.flush();

        let loaded = Storage::load(&directory).unwrap().unwrap();
        assert_eq!(loaded.notes, library.notes);
        assert_eq!(loaded.folders, library.folders);
        let mut next = loaded.clone();
        let mut expected = library.clone();
        assert_eq!(next.create("notes"), expected.create("notes"));
    }

    // Loading without a file reports that nothing is saved yet.
    #[test]
    fn missing_file_loads_nothing() {
        assert!(Storage::load(&folder("missing")).unwrap().is_none());
    }

    // An unreadable file is kept aside instead of being overwritten later.
    #[test]
    fn unreadable_file_is_moved_aside() {
        let directory = folder("unreadable");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(Storage::FILE), "{ not json").unwrap();

        assert!(Storage::load(&directory).unwrap().is_none());
        assert!(directory.join("notes.unreadable.json").exists());
        assert!(!directory.join(Storage::FILE).exists());
    }

    // Formatting flags that are off are left out of the file.
    #[test]
    fn plain_runs_stay_compact() {
        let json = serde_json::to_string(&TextFile::from(&AttributedText::from("Hi"))).unwrap();
        assert_eq!(json, r#"{"runs":[{"text":"Hi"}],"paragraphs":["body"]}"#);
    }
}
