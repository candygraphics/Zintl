//! A Notes-style app built with zintl-ui-desktop. See `PLAN.md`.

mod model;
mod storage;

use model::{Folder, Library, Note, NoteId, Place};
use storage::{Saver, Storage};
use zintl_ui_desktop::accessibility::*;
use zintl_ui_desktop::menu::*;
use zintl_ui_desktop::*;

/// Stores shared by the views and their actions.
#[derive(Clone, Copy, Default)]
struct State {
    library: Store<Library>,
    /// The sidebar's selection.
    folder: Store<Option<Place>>,
    /// The note list's selection.
    selected: Store<Option<NoteId>>,
    /// The editor's text, which mirrors the selected note's body.
    draft: Store<AttributedText>,
    saver: Store<Saver>,
    /// Text in the toolbar search field; non-empty text searches every folder.
    query: Store<String>,
    /// Fires to move keyboard focus to the editor.
    focus: Trigger,
    /// The folder being renamed and the name being typed.
    renaming: Store<Option<String>>,
    rename_text: Store<String>,
    rename_open: Store<bool>,
}

impl State {
    /// Changes the library and queues it to be saved.
    fn change<R>(&self, cx: &mut Context<'_>, update: impl FnOnce(&mut Library) -> R) -> R {
        let result = cx.update(self.library, update);
        cx.get(self.saver).save(cx.get(self.library));
        result
    }

    fn place(&self, cx: &Context<'_>) -> Place {
        cx.get(self.folder).clone().unwrap_or(Place::All)
    }

    /// Stores the edited draft in the selected note, which moves it to the top of its list.
    fn save(&self, cx: &mut Context<'_>) {
        let Some(id) = *cx.get(self.selected) else {
            return;
        };
        let draft = cx.get(self.draft).clone();
        self.change(cx, |library| library.set_body(id, &draft));
    }

    fn select(&self, cx: &mut Context<'_>, id: Option<NoteId>) {
        let body = id
            .and_then(|id| cx.get(self.library).note(id))
            .map(|note| note.body.clone())
            .unwrap_or_default();
        cx.update(self.selected, |selected| *selected = id);
        cx.update(self.draft, |draft| *draft = body);
    }

    /// Selects the first note of the folder chosen in the sidebar.
    fn select_folder(&self, cx: &mut Context<'_>) {
        let place = self.place(cx);
        let first = cx
            .get(self.library)
            .notes_in(&place)
            .first()
            .map(|note| note.id);
        self.select(cx, first);
    }

    fn new_note(&self, cx: &mut Context<'_>) {
        let place = self.place(cx);
        let id = self.change(cx, |library| library.create(&place));
        cx.update(self.query, String::clear);
        self.select(cx, Some(id));
        cx.fire(self.focus, ());
        cx.announce("New note", AnnouncementPriority::Medium);
    }

    fn delete_selected(&self, cx: &mut Context<'_>) {
        if let Some(id) = *cx.get(self.selected) {
            self.delete(cx, id);
        }
    }

    /// Moves a note to Recently Deleted, or removes it from there.
    fn delete(&self, cx: &mut Context<'_>, id: NoteId) {
        let deleted = cx
            .get(self.library)
            .note(id)
            .is_some_and(|note| note.deleted);
        self.change(cx, |library| library.delete(id));
        // Deleting from the toolbar or a menu leaves focus elsewhere, so the result is spoken.
        let message = if deleted {
            "Note deleted"
        } else {
            "Moved to Recently Deleted"
        };
        cx.announce(message, AnnouncementPriority::Medium);
        if *cx.get(self.selected) == Some(id) {
            cx.update(self.selected, |selected| *selected = None);
            self.select_folder(cx);
        }
    }

    fn move_note(&self, cx: &mut Context<'_>, id: NoteId, folder: String) {
        self.change(cx, |library| library.move_note(id, &folder));
        let name = cx
            .get(self.library)
            .folders
            .iter()
            .find(|candidate| candidate.id == folder)
            .map(|folder| folder.name.clone())
            .unwrap_or(folder);
        cx.announce(format!("Moved to {name}"), AnnouncementPriority::Medium);
    }

    /// Creates a folder, selects it, and asks for its name.
    fn new_folder(&self, cx: &mut Context<'_>) {
        let id = self.change(cx, |library| library.create_folder("New Folder"));
        cx.update(self.folder, |folder| *folder = Some(Place::folder(&id)));
        self.select_folder(cx);
        self.start_rename(cx, id);
    }

    fn start_rename(&self, cx: &mut Context<'_>, id: String) {
        let name = cx
            .get(self.library)
            .folders
            .iter()
            .find(|folder| folder.id == id)
            .map(|folder| folder.name.clone())
            .unwrap_or_default();
        cx.update(self.rename_text, |text| *text = name);
        cx.update(self.renaming, |renaming| *renaming = Some(id));
        cx.update(self.rename_open, |open| *open = true);
    }

    fn finish_rename(&self, cx: &mut Context<'_>) {
        let name = cx.get(self.rename_text).trim().to_owned();
        if let Some(id) = cx.get(self.renaming).clone()
            && !name.is_empty()
        {
            self.change(cx, |library| library.rename_folder(&id, &name));
        }
        cx.update(self.rename_open, |open| *open = false);
    }

    /// Deletes a folder; its notes go to Recently Deleted.
    fn delete_folder(&self, cx: &mut Context<'_>, id: String) {
        self.change(cx, |library| library.delete_folder(&id));
        if *cx.get(self.folder) == Some(Place::folder(&id)) {
            cx.update(self.folder, |folder| *folder = Some(Place::All));
            self.select_folder(cx);
        }
    }

    fn toggle_pin(&self, cx: &mut Context<'_>, id: NoteId) {
        let pinned = self.change(cx, |library| {
            let pinned = !library.note(id).is_some_and(|note| note.pinned);
            library.set_pinned(id, pinned);
            pinned
        });
        let message = if pinned { "Pinned" } else { "Unpinned" };
        cx.announce(message, AnnouncementPriority::Medium);
    }
}

// Folder IDs are prefixed so no folder can take a smart folder's ID.
impl ItemKey for Place {
    fn item_id(&self) -> String {
        match self {
            Place::All => "all".to_owned(),
            Place::Deleted => "deleted".to_owned(),
            Place::Folder(id) => format!("folder:{id}"),
        }
    }
}

struct NotesApp {
    state: State,
    /// The loaded library and its saver, moved into Stores by `init`.
    initial: Option<(Library, Saver)>,
}

impl NotesApp {
    fn new(library: Library, saver: Saver) -> Self {
        Self {
            state: State::default(),
            initial: Some((library, saver)),
        }
    }
}

impl View for NotesApp {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        // Open with the first note selected, as Notes does.
        let (library, saver) = self.initial.take().expect("init runs once");
        let first = library
            .notes_in(&Place::All)
            .first()
            .map(|note| (*note).clone());
        self.state = State {
            library: cx.store(library),
            folder: cx.store(Some(Place::All)),
            selected: cx.store(first.as_ref().map(|note| note.id)),
            draft: cx.store(first.map(|note| note.body).unwrap_or_default()),
            saver: cx.store(saver),
            query: cx.store(String::new()),
            focus: cx.trigger(),
            renaming: cx.store(None),
            rename_text: cx.store(String::new()),
            rename_open: cx.store(false),
        };
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let state = self.state;
        let folders = cx.get(state.library).folders.clone();
        let library = cx.get(state.library);
        let count = |place: &Place| library.notes_in(place).len().to_string();

        list![
            MainMenu::standard()
                .app_name("Notes")
                .extend(
                    StandardMenu::File,
                    [
                        MenuItem::new("New Note")
                            .shortcut(Shortcut::command("n"))
                            .on_select(move |cx| state.new_note(cx)),
                        MenuItem::new("New Folder")
                            .shortcut(Shortcut::command("n").shift())
                            .on_select(move |cx| state.new_folder(cx)),
                        MenuItem::new("Delete Note")
                            .enabled(cx.get(state.selected).is_some())
                            .on_select(move |cx| state.delete_selected(cx)),
                    ],
                )
                .menu(Menu::new(
                    "Format",
                    [
                        MenuItem::standard(StandardAction::TitleStyle),
                        MenuItem::standard(StandardAction::HeadingStyle),
                        MenuItem::standard(StandardAction::SubheadingStyle),
                        MenuItem::standard(StandardAction::BodyStyle),
                        MenuItem::standard(StandardAction::MonospacedStyle),
                        MenuItem::separator(),
                        MenuItem::standard(StandardAction::BulletedList),
                        MenuItem::standard(StandardAction::DashedList),
                        MenuItem::standard(StandardAction::NumberedList),
                        MenuItem::standard(StandardAction::Checklist),
                        MenuItem::separator(),
                        MenuItem::standard(StandardAction::Bold),
                        MenuItem::standard(StandardAction::Italic),
                        MenuItem::standard(StandardAction::Underline),
                    ],
                )),
            Window::new(Rect::new(120.0, 120.0, 960.0, 600.0), "Notes")
                .autosave("notes.main")
                .toolbar(Toolbar::new([
                    ToolbarItem::button("delete", "Delete", "trash")
                        .enabled(cx.get(state.selected).is_some())
                        .on_click(move |cx| state.delete_selected(cx)),
                    ToolbarItem::flexible_space(),
                    ToolbarItem::button("new", "New Note", "square.and.pencil")
                        .on_click(move |cx| state.new_note(cx)),
                    ToolbarItem::search(state.query).placeholder("Search"),
                ]))
                .sidebar(
                    Sidebar::new([
                        SidebarSection::new([SidebarItem::new(Place::All, "All Notes")
                            .system_image("tray.full")
                            .badge(count(&Place::All))])
                        .title("iCloud"),
                        SidebarSection::new(folders.into_iter().map(|folder| {
                            let (rename, delete) = (folder.id.clone(), folder.id.clone());
                            let place = Place::Folder(folder.id);
                            let badge = count(&place);
                            SidebarItem::new(place, folder.name)
                                .system_image("folder")
                                .badge(badge)
                                .context_menu([
                                    MenuItem::new("Rename Folder…").on_select(move |cx| {
                                        state.start_rename(cx, rename.clone())
                                    }),
                                    MenuItem::new("Delete Folder").on_select(move |cx| {
                                        state.delete_folder(cx, delete.clone())
                                    }),
                                ])
                        }))
                        .title("Folders"),
                        SidebarSection::new([
                            SidebarItem::new(Place::Deleted, "Recently Deleted",)
                                .system_image("trash")
                                .badge(count(&Place::Deleted))
                        ]),
                    ])
                    .bind(state.folder)
                    .on_select(move |cx, _| state.select_folder(cx)),
                )
                .content(
                    SplitView::horizontal(NoteList { state }, Editor { state })
                        .position(260.0)
                        .range(200.0..=420.0),
                ),
            Window::panel(Rect::new(400.0, 300.0, 320.0, 120.0), "Rename Folder")
                .bind_open(state.rename_open)
                .content(
                    VStack::new(list![
                        TextField::new()
                            .bind(state.rename_text)
                            .placeholder("Folder name")
                            .accessibility_label("Folder name"),
                        HStack::new(list![
                            Spacer::new(),
                            Button::new("Cancel").role(ButtonRole::Cancel).on_click(
                                move |cx| cx.update(state.rename_open, |open| *open = false)
                            ),
                            Button::new("Rename")
                                .role(ButtonRole::Default)
                                .on_click(move |cx| state.finish_rename(cx)),
                        ])
                        .fill_width(),
                    ])
                    .padding(16.0)
                    .fill_width(),
                ),
        ]
    }
}

/// The middle column: the selected folder's notes, pinned ones first.
#[derive(Clone)]
struct NoteList {
    state: State,
}

impl NoteList {
    fn row(state: State, note: &Note, folders: &[Folder]) -> ListRow<NoteId> {
        let id = note.id;
        let destinations = folders
            .iter()
            .filter(|folder| note.deleted || folder.id != note.folder)
            .map(|folder| {
                let target = folder.id.clone();
                MenuItem::new(folder.name.clone())
                    .on_select(move |cx| state.move_note(cx, id, target.clone()))
            })
            .collect::<Vec<_>>();
        let pin = if note.pinned {
            "Unpin Note"
        } else {
            "Pin Note"
        };
        ListRow::new(
            note.id,
            VStack::new(list![
                Text::new(note.title())
                    .font(Font::system(13.0).bold())
                    .line_limit(1)
                    .fill_width(),
                Text::new(note.preview())
                    .color(TextColor::Secondary)
                    .line_limit(1)
                    .fill_width(),
            ])
            .spacing(2.0)
            .padding(6.0)
            .fill_width()
            // VoiceOver reads each row as one item instead of two separate texts.
            .accessibility_label(note.title())
            .accessibility_value(if note.pinned {
                format!("Pinned, {}", note.preview())
            } else {
                note.preview().to_owned()
            })
            .context_menu([
                MenuItem::new(pin).on_select(move |cx| state.toggle_pin(cx, id)),
                MenuItem::submenu(Menu::new("Move to", destinations)),
                MenuItem::separator(),
                MenuItem::new("Delete").on_select(move |cx| state.delete(cx, id)),
            ]),
        )
    }
}

impl View for NoteList {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let state = self.state;
        let place = state.place(cx);
        let folders = cx.get(state.library).folders.clone();
        let query = cx.get(state.query).clone();
        let notes = if query.is_empty() {
            cx.get(state.library).notes_in(&place)
        } else {
            cx.get(state.library).search(&query)
        };
        let count = notes.len();
        let (pinned, others): (Vec<&Note>, Vec<&Note>) =
            notes.into_iter().partition(|note| note.pinned);
        let mut sections = Vec::new();
        if !pinned.is_empty() {
            sections.push(
                ListSection::new(
                    pinned
                        .into_iter()
                        .map(|note| Self::row(state, note, &folders)),
                )
                .title("Pinned"),
            );
            sections.push(
                ListSection::new(
                    others
                        .into_iter()
                        .map(|note| Self::row(state, note, &folders)),
                )
                .title("Notes"),
            );
        } else {
            sections.push(ListSection::new(
                others
                    .into_iter()
                    .map(|note| Self::row(state, note, &folders)),
            ));
        }
        VStack::new(list![
            HStack::new(list![
                Text::new(match (count, query.is_empty()) {
                    (1, true) => "1 note".to_owned(),
                    (count, true) => format!("{count} notes"),
                    (1, false) => "1 result".to_owned(),
                    (count, false) => format!("{count} results"),
                })
                .color(TextColor::Secondary)
                .id("note-count"),
            ])
            .alignment(CrossAxisAlignment::Center)
            .padding(12.0)
            .fill_width(),
            // The list is named by the count above it, such as "3 notes".
            List::new(sections)
                .accessibility_labeled_by("note-count")
                .bind(state.selected)
                .on_select(move |cx, id| state.select(cx, id.copied())),
        ])
        .spacing(0.0)
        .fill_width()
        .grow()
    }
}

/// The detail column: the selected note's text.
#[derive(Clone)]
struct Editor {
    state: State,
}

impl View for Editor {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let state = self.state;
        if cx.get(state.selected).is_none() {
            return VStack::new(list![Text::new("No Note Selected")])
                .padding(24.0)
                .fill_width()
                .grow();
        }
        let title = cx
            .get(state.selected)
            .and_then(|id| cx.get(state.library).note(id))
            .map(|note| note.title().to_owned())
            .unwrap_or_default();
        VStack::new(list![
            TextEditor::new()
                .bind(state.draft)
                .focus_on(state.focus)
                .on_change(move |cx, _| state.save(cx))
                .accessibility_label(title),
        ])
        .fill_width()
        .grow()
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn main() -> Result<(), AppError> {
    let directory = Storage::directory();
    // A library that cannot be read is never replaced by the sample.
    let library = match Storage::load(&directory) {
        Ok(library) => library.unwrap_or_else(Library::sample),
        Err(error) => {
            eprintln!("notes: could not read {}: {error}", directory.display());
            std::process::exit(1);
        }
    };
    let saver = Storage::writer(directory);
    let flush = saver.clone();
    App::new(NotesApp::new(library, saver))
        .on_exit(move || flush.flush())
        .run()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn main() {}
