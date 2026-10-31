//! A Notes-style app built with zintl-ui-desktop. See `PLAN.md`.

mod model;
mod storage;

use model::{Library, Note, NoteId, SmartFolder};
use storage::{Saver, Storage};
use zintl_ui_desktop::*;

/// Stores shared by the views and their actions.
#[derive(Clone, Copy, Default)]
struct State {
    library: Store<Library>,
    folder: Store<Option<String>>,
    selected: Store<Option<NoteId>>,
    /// The list's selection, which mirrors `selected` as a row ID.
    row: Store<Option<String>>,
    /// The editor's text, which mirrors the selected note's body.
    draft: Store<AttributedText>,
    saver: Store<Saver>,
    /// Text in the toolbar search field; non-empty text searches every folder.
    query: Store<String>,
    /// Increases to move keyboard focus to the editor.
    focus: Store<u64>,
}

impl State {
    /// Changes the library and queues it to be saved.
    fn change<R>(&self, cx: &mut Context<'_>, update: impl FnOnce(&mut Library) -> R) -> R {
        let result = cx.update(self.library, update);
        cx.get(self.saver).save(cx.get(self.library));
        result
    }

    fn folder_id(&self, cx: &Context<'_>) -> String {
        cx.get(self.folder)
            .clone()
            .unwrap_or_else(|| SmartFolder::ALL.to_owned())
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
        cx.update(self.row, |row| *row = id.map(|id| id.to_string()));
        cx.update(self.draft, |draft| *draft = body);
    }

    /// Selects the first note of the folder chosen in the sidebar.
    fn select_folder(&self, cx: &mut Context<'_>) {
        let folder = self.folder_id(cx);
        let first = cx
            .get(self.library)
            .notes_in(&folder)
            .first()
            .map(|note| note.id);
        self.select(cx, first);
    }

    fn new_note(&self, cx: &mut Context<'_>) {
        let folder = self.folder_id(cx);
        let id = self.change(cx, |library| library.create(&folder));
        cx.update(self.query, String::clear);
        self.select(cx, Some(id));
        cx.update(self.focus, |focus| *focus += 1);
    }

    fn delete_selected(&self, cx: &mut Context<'_>) {
        if let Some(id) = *cx.get(self.selected) {
            self.delete(cx, id);
        }
    }

    /// Moves a note to Recently Deleted, or removes it from there.
    fn delete(&self, cx: &mut Context<'_>, id: NoteId) {
        self.change(cx, |library| library.delete(id));
        if *cx.get(self.selected) == Some(id) {
            cx.update(self.selected, |selected| *selected = None);
            self.select_folder(cx);
        }
    }

    fn toggle_pin(&self, cx: &mut Context<'_>, id: NoteId) {
        self.change(cx, |library| {
            let pinned = library.note(id).is_some_and(|note| note.pinned);
            library.set_pinned(id, !pinned);
        });
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
            .notes_in(SmartFolder::ALL)
            .first()
            .map(|note| (*note).clone());
        self.state = State {
            library: cx.store(library),
            folder: cx.store(Some(SmartFolder::ALL.to_owned())),
            selected: cx.store(first.as_ref().map(|note| note.id)),
            row: cx.store(first.as_ref().map(|note| note.id.to_string())),
            draft: cx.store(first.map(|note| note.body).unwrap_or_default()),
            saver: cx.store(saver),
            query: cx.store(String::new()),
            focus: cx.store(0),
        };
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let state = self.state;
        let folders = cx.get(state.library).folders.clone();

        list![
            MainMenu::standard()
                .app_name("Notes")
                .extend(
                    StandardMenu::File,
                    [
                        MenuItem::new("New Note")
                            .shortcut(Shortcut::command("n"))
                            .on_select(move |cx| state.new_note(cx)),
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
                        SidebarSection::new([SidebarItem::new(SmartFolder::ALL, "All Notes")
                            .system_image("tray.full")])
                        .title("iCloud"),
                        SidebarSection::new(folders.into_iter().map(|folder| {
                            SidebarItem::new(folder.id, folder.name).system_image("folder")
                        }))
                        .title("Folders"),
                        SidebarSection::new([SidebarItem::new(
                            SmartFolder::DELETED,
                            "Recently Deleted",
                        )
                        .system_image("trash")]),
                    ])
                    .bind(state.folder)
                    .on_select(move |cx, _| state.select_folder(cx)),
                )
                .content(
                    SplitView::horizontal(NoteList { state }, Editor { state })
                        .position(260.0)
                        .range(200.0..=420.0),
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
    fn row(state: State, note: &Note) -> ListRow {
        let id = note.id;
        let pin = if note.pinned {
            "Unpin Note"
        } else {
            "Pin Note"
        };
        ListRow::new(
            note.id.to_string(),
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
            .context_menu([
                MenuItem::new(pin).on_select(move |cx| state.toggle_pin(cx, id)),
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
        let folder = state.folder_id(cx);
        let query = cx.get(state.query).clone();
        let notes = if query.is_empty() {
            cx.get(state.library).notes_in(&folder)
        } else {
            cx.get(state.library).search(&query)
        };
        let count = notes.len();
        let (pinned, others): (Vec<&Note>, Vec<&Note>) =
            notes.into_iter().partition(|note| note.pinned);
        let mut sections = Vec::new();
        if !pinned.is_empty() {
            sections.push(
                ListSection::new(pinned.into_iter().map(|note| Self::row(state, note)))
                    .title("Pinned"),
            );
            sections.push(
                ListSection::new(others.into_iter().map(|note| Self::row(state, note)))
                    .title("Notes"),
            );
        } else {
            sections.push(ListSection::new(
                others.into_iter().map(|note| Self::row(state, note)),
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
                .color(TextColor::Secondary),
            ])
            .alignment(CrossAxisAlignment::Center)
            .padding(12.0)
            .fill_width(),
            List::new(sections)
                .bind(state.row)
                .on_select(move |cx, id| {
                    state.select(cx, id.and_then(|id| id.parse().ok()));
                }),
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
        VStack::new(list![
            TextEditor::new()
                .bind_rich(state.draft)
                .focus_on(state.focus)
                .on_change(move |cx, _| state.save(cx)),
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
