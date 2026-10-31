//! A Notes-style app built with zintl-ui-desktop. See `PLAN.md`.

mod model;

use model::{Library, Note, NoteId, SmartFolder};
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
    draft: Store<String>,
}

impl State {
    fn folder_id(&self, cx: &Context<'_>) -> String {
        cx.get(self.folder)
            .clone()
            .unwrap_or_else(|| SmartFolder::ALL.to_owned())
    }

    /// Stores an edit of the selected note, which moves it to the top of its list.
    fn save(&self, cx: &mut Context<'_>, text: &str) {
        let Some(id) = *cx.get(self.selected) else {
            return;
        };
        cx.update(self.library, |library| library.set_body(id, text));
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
        let id = cx.update(self.library, |library| library.create(&folder));
        self.select(cx, Some(id));
    }

    fn delete_selected(&self, cx: &mut Context<'_>) {
        let Some(id) = *cx.get(self.selected) else {
            return;
        };
        cx.update(self.library, |library| library.delete(id));
        cx.update(self.selected, |selected| *selected = None);
        self.select_folder(cx);
    }
}

#[derive(Default)]
struct NotesApp {
    state: State,
}

impl View for NotesApp {
    type Output = RenderNode;

    fn init(&mut self, cx: &mut Context<'_>) {
        // Open with the first note selected, as Notes does.
        let library = Library::sample();
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
        };
    }

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let state = self.state;
        let folders = cx.get(state.library).folders.clone();

        list![
            MainMenu::standard().app_name("Notes").extend(
                StandardMenu::File,
                [
                    MenuItem::new("New Note")
                        .shortcut(Shortcut::command("n"))
                        .on_select(move |cx| state.new_note(cx)),
                    MenuItem::new("Delete Note").on_select(move |cx| state.delete_selected(cx)),
                ],
            ),
            Window::new(Rect::new(120.0, 120.0, 960.0, 600.0), "Notes")
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
    fn row(note: &Note) -> ListRow {
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
            .fill_width(),
        )
    }
}

impl View for NoteList {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let state = self.state;
        let folder = state.folder_id(cx);
        let notes = cx.get(state.library).notes_in(&folder);
        let count = notes.len();
        let (pinned, others): (Vec<&Note>, Vec<&Note>) =
            notes.into_iter().partition(|note| note.pinned);
        let mut sections = Vec::new();
        if !pinned.is_empty() {
            sections.push(ListSection::new(pinned.into_iter().map(Self::row)).title("Pinned"));
            sections.push(ListSection::new(others.into_iter().map(Self::row)).title("Notes"));
        } else {
            sections.push(ListSection::new(others.into_iter().map(Self::row)));
        }
        VStack::new(list![
            HStack::new(list![
                Text::new(match count {
                    1 => "1 note".to_owned(),
                    count => format!("{count} notes"),
                })
                .color(TextColor::Secondary),
                Spacer::new(),
                Button::new("")
                    .system_image("square.and.pencil")
                    .style(ButtonStyle::Borderless)
                    .on_click(move |cx| state.new_note(cx)),
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
// TODO: Edit rich text; see PLAN.md (C3, C4).
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
            HStack::new(list![
                Button::new("")
                    .system_image("trash")
                    .style(ButtonStyle::Borderless)
                    .on_click(move |cx| state.delete_selected(cx)),
            ])
            .padding(8.0),
            TextEditor::new()
                .bind(state.draft)
                .on_change(move |cx, text| state.save(cx, text)),
        ])
        .spacing(0.0)
        .fill_width()
        .grow()
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn main() -> Result<(), AppError> {
    App::new(NotesApp::default()).run()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn main() {}
