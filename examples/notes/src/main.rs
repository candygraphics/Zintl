//! A Notes-style app built with zintl-ui-desktop. See `PLAN.md`.

mod model;

use model::{Library, NoteId, SmartFolder};
use zintl_ui_desktop::*;

/// Stores shared by the views and their actions.
#[derive(Clone, Copy, Default)]
struct State {
    library: Store<Library>,
    folder: Store<Option<String>>,
    selected: Store<Option<NoteId>>,
    /// Text of the selected note while it is edited.
    draft: Store<String>,
}

impl State {
    fn folder_id(&self, cx: &Context<'_>) -> String {
        cx.get(self.folder)
            .clone()
            .unwrap_or_else(|| SmartFolder::ALL.to_owned())
    }

    /// Writes the draft back to the selected note.
    // TODO: Save on every edit once text fields report changes.
    fn commit(&self, cx: &mut Context<'_>) {
        let Some(id) = *cx.get(self.selected) else {
            return;
        };
        let draft = cx.get(self.draft).clone();
        cx.update(self.library, |library| library.set_body(id, &draft));
    }

    fn select(&self, cx: &mut Context<'_>, id: Option<NoteId>) {
        self.commit(cx);
        let body = id
            .and_then(|id| cx.get(self.library).note(id))
            .map(|note| note.body.clone())
            .unwrap_or_default();
        cx.update(self.selected, |selected| *selected = id);
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
        self.commit(cx);
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
                    HStack::new(list![NoteList { state }, Editor { state }])
                        .spacing(1.0)
                        .fill_width()
                        .grow(),
                ),
        ]
    }
}

/// The middle column: one row per note in the selected folder.
// TODO: Replace with a selectable list view with styled two-line rows.
#[derive(Clone)]
struct NoteList {
    state: State,
}

impl View for NoteList {
    type Output = RenderNode;

    fn render(&self, cx: &mut Context<'_>) -> impl IntoElement<Output = RenderNode> {
        let state = self.state;
        let folder = state.folder_id(cx);
        let selected = *cx.get(state.selected);
        let mut rows = list![
            HStack::new(list![
                Text::new(format!(
                    "{} notes",
                    cx.get(state.library).notes_in(&folder).len()
                )),
                Button::new("")
                    .system_image("square.and.pencil")
                    .style(ButtonStyle::Borderless)
                    .on_click(move |cx| state.new_note(cx)),
            ])
            .space_between()
            .fill_width()
        ];
        for note in cx.get(state.library).notes_in(&folder) {
            let id = note.id;
            let marker = if Some(id) == selected { "▸ " } else { "" };
            let pin = if note.pinned { "📌 " } else { "" };
            rows.extend(list![
                VStack::new(list![
                    Button::new(format!("{marker}{pin}{}", note.title()))
                        .style(ButtonStyle::Borderless)
                        .on_click(move |cx| state.select(cx, Some(id))),
                    Text::new(note.preview()),
                ])
                .spacing(2.0)
                .fill_width()
            ]);
        }
        ScrollView::new(VStack::new(rows).spacing(12.0).padding(12.0).fill_width())
    }
}

/// The detail column: the selected note's text.
// TODO: Replace with a rich text editor.
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
            ]),
            TextField::new()
                .multiline()
                .placeholder("Start typing")
                .bind(state.draft),
        ])
        .spacing(8.0)
        .padding(12.0)
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
