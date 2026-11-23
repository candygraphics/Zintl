# zintl-ui-desktop

Declarative desktop views rendered with AppKit on macOS and WinUI 3 on Windows.

Attach navigation to a window with `Window::sidebar`:

```rust,ignore
// Create this Store in View::init and retain it on the view.
self.selection = cx.store(Some("home"));

// Build this window in View::render.
Window::new("Navigation").size(Size::new(640.0, 400.0))
    .sidebar(
        Sidebar::new([SidebarSection::new([
            SidebarItem::new("home", "Home").system_image("house"),
            SidebarItem::new("settings", "Settings").system_image("gearshape"),
        ]).title("Pages")])
        .bind(self.selection)
        .on_select(|_cx, id| println!("Selected: {id}")),
    )
    .content(cx.watch(self.selection, |selected| {
        Text::new(selected.unwrap_or("No selection"))
    }))
```

## Child lists

Use `list!` for heterogeneous stack children. The macro returns a
`ListFactory<RenderNode>`, an alias for `Vec<ElementFactory<RenderNode>>`, so the
number of children is not limited by a tuple arity.

```rust,ignore
VStack::new(list![
    Text::new("Profile"),
    TextField::new().placeholder("Name"),
    TextField::new().placeholder("Email"),
    Text::new("Preferences"),
    Button::new("Import"),
    Button::new("Export"),
    Button::new("Reset"),
    Button::new("Save"),
])
```

The returned vector can also be extended at runtime.

```rust,ignore
let mut actions = list![Button::new("Save")];
if can_delete {
    actions.extend(list![Button::new("Delete")]);
}
actions.extend(list![Text::new("Ready"), Button::new("Close")]);

HStack::new(actions)
```

AppKit uses a scrollable `NSTableView` with source-list styling and 32-point rows.
Section titles are nonselectable group rows; item cells show a symbol and a
truncating label. The sidebar width is limited to 180–320 points.

`on_select` receives the selected item's key after the bound Store is updated. It
also works without `bind`. Initial/programmatic selection and deselection do not
invoke it; native item selection changes do.

Item keys must be unique across the sidebar. `bind` reads and updates a
`Store<Option<K>>` of the item keys; programmatic changes also update the native
selection, and a key that no item has shows nothing selected.
AppKit automatically adds a standard sidebar toggle to the window toolbar.
Removing the sidebar also removes the toolbar item it added; preexisting toolbar
items are preserved. Collapse state survives reactive updates.

Unknown selection events are ignored. Rebuilding a window without `.sidebar(...)`
removes its sidebar. SF Symbol names are interpreted by AppKit.

## Controls

Controls bind to Stores with `bind` and report changes with `on_change`.
Without a Store, `value` (or `selection` for a `Picker` or `TabView`) shows a
value the app owns; `bind` takes precedence. Each renders a native control.

A `Picker` selects one of its values, given with their labels, such as an enum;
`Picker::indexed` selects plain labels by index. A value that no option has
shows nothing selected.

```rust,ignore
VStack::new(list![
    Toggle::new("Notifications").bind(self.notifications),            // NSSwitch
    Toggle::new("Agree").style(ToggleStyle::Checkbox).bind(self.agree),
    Toggle::new("Bold").style(ToggleStyle::Button).bind(self.bold),   // push on/push off
    Picker::new([(Size::Small, "Small"), (Size::Medium, "Medium")])  // NSPopUpButton
        .bind(self.size),
    Picker::new([("list", "List"), ("grid", "Grid")]).style(PickerStyle::Segmented).bind(self.layout),
    Picker::indexed(["Low", "High"]).style(PickerStyle::Radio).bind(self.priority),
    ComboBox::new(["Apple", "Banana"]).placeholder("Fruit").bind(self.fruit),
    Slider::new(0.0..=100.0).ticks(11).snaps_to_ticks(true).bind(self.volume),
    NumberField::new().range(0.0..=99.0).step(1.0).bind(self.quantity),
    ColorWell::new().bind(self.tint),
    SearchField::new().placeholder("Search").bind(self.query),
])
```

Buttons take a `ButtonStyle` (push, borderless, toolbar, …), a
`ButtonRole` (`Default` answers Return, `Cancel` answers Escape, `Destructive`
warns), and SF Symbols through `system_image`. `Button::icon("gearshape", "Settings")`
shows a symbol alone and takes its size; VoiceOver reads the second argument as
its name. `Button::help()` is the round question mark button.

`TextField`, `IntegerField`, and `NumberField` take a button inside their
trailing edge with `accessory`, like a search field's clear button. It does
not take keyboard focus or change the text, the text stops before it, and a
popover on the field still anchors to the whole field:

```rust,ignore
TextField::new().bind(self.amount).accessory(
    FieldAccessory::button("plus.forwardslash.minus", "Calculator") // SF Symbol, VoiceOver name
        .id("amount-calculator")
        .on_click(open_calculator),
)
```

Builder methods that turn a state or feature on take a `bool`, such as
`enabled(false)`, `multiline(true)`, or `accessibility_hidden(true)`, so a
condition does not need an `if` around the builder. Layout instructions take
none: `fill_width()`, `fill_height()`, `grow()`, `ignores_safe_area()`.

`ScrollView::new(content)` and `TabView::new([Tab::new("general", "General", page)])`
fill their parent's width and the remaining space. Each `Tab` carries the value
that selects it. Stacks accept `padding`, `grow`, `equal_children()`, and a
`MainAxisDistribution` such as `distribution(MainAxisDistribution::SpaceBetween)`.

`Grid` takes one `GridRow` per row and has as many columns as its widest row.
`GridRow::span` makes a cell cover several columns, and `striped` shades every
other row:

```rust,ignore
Grid::new([
    GridRow::new(list![Text::new("Name"), TextField::new()]),
    GridRow::new(list![Text::new("Advanced")]).span(0, 2), // covers both columns
    GridRow::new(list![Text::new("Port"), IntegerField::new()]),
])
.striped(true)
```

## Text editing

`TextEditor` is a scrolling rich text editor (`NSTextView` on macOS) that fills
its parent, with undo and a find bar. Use it for document text; for a few lines
of plain text that do not scroll, use `TextField::new().multiline(true)`. `TextField` and `TextEditor` take
`on_change`, which runs after each edit once the bound Store is updated.

```rust,ignore
TextEditor::new()
    .bind(self.draft)
    .on_change(|cx, text| { /* save */ })
```

Replacing the bound text from code moves the insertion point to the end and
clears the editor's undo history, since it describes the replaced text.

`Text::selectable(true)` lets the user select a displayed value and copy it,
while it still looks like plain text. In a table, the first click selects the
row and a click on the selected row's text selects the text.

```rust,ignore
Text::new(format_amount(total)).selectable(true).id("total");
```

Actions read and write plain text on the system clipboard with
`cx.clipboard()`; the test `Harness` keeps its own clipboard instead.

```rust,ignore
Button::new("Copy").on_click(move |cx| {
    let text = cx.get(result).to_string();
    cx.clipboard().set_text(text);
});
let pasted: Option<String> = cx.clipboard().text();
```

Actions read and write plain text on the system clipboard with
`cx.clipboard()`; the test `Harness` keeps its own clipboard instead.

```rust,ignore
Button::new("Copy").on_click(move |cx| {
    let text = cx.get(result).to_string();
    cx.clipboard().set_text(text);
});
let pasted: Option<String> = cx.clipboard().text();
```

The editor edits an `AttributedText`: plain text with styled runs (bold,
italic, underline, strikethrough, monospace, link) and one `ParagraphStyle`
per line (Title, Heading, Subheading, Body, Monospaced, or a bulleted, dashed,
numbered, or checklist item). List markers are drawn beside the text and are
not part of it. Clicking a checklist circle toggles it; Return continues a
list, ends it on an empty item, and returns to body text after a heading. Formatting comes from
standard actions that the focused editor performs, so it is undoable and the
menu shows the current paragraph style with a check mark.

```rust,ignore
MainMenu::standard().menu(Menu::new("Format", [
    MenuItem::standard(StandardAction::TitleStyle),   // ⇧⌘T
    MenuItem::standard(StandardAction::BodyStyle),    // ⇧⌘B
    MenuItem::separator(),
    MenuItem::standard(StandardAction::Bold),         // ⌘B
]));
```

## Lists

`List` is a native `NSTableView` with inset rows, rounded selection, arrow-key
navigation, and section headers. Each row is any view; it lays out at the
cell's width and its content decides the row height.

```rust,ignore
List::new([
    ListSection::new(pinned.iter().map(|note| ListRow::new(note.id, NoteRow::new(note))))
        .title("Pinned"),
    ListSection::new(others.iter().map(|note| ListRow::new(note.id, NoteRow::new(note)))),
])
.bind(self.selection)                    // Store<Option<NoteId>> of row keys
.on_select(|cx, id| { /* … */ })        // id: Option<&NoteId>
```

`List`, `Table`, and `Sidebar` select rows by a key of any `ItemKey` type.
Strings, `&'static str`, and the integer types already are keys; an app's own
ID type implements `item_id`, which must give different keys different strings:

```rust,ignore
#[derive(Clone, PartialEq)]
enum Place { All, Folder(String), Deleted }

impl ItemKey for Place {
    fn item_id(&self) -> String {
        match self {
            Place::All => "all".to_owned(),
            Place::Folder(id) => format!("folder:{id}"),
            Place::Deleted => "deleted".to_owned(),
        }
    }
}
```

Keys must be unique across the list; rows are keyed by them, so a row's view
keeps its state when the row moves. String literal keys infer
`K = &'static str`, so bind them to a `Store<Option<&'static str>>`. A bound key
that no row has shows nothing selected. `on_select` runs after the bound Store
is updated, for user changes only. On Windows the
rows are shown in a scroll view without selection for now.

`Table::summary_row` shows a row below the rows, such as totals, with one cell
per column. It stays visible while the rows scroll, cannot be selected, is not
sorted, and its cells follow the columns' widths and alignment (macOS only for
now):

```rust,ignore
Table::new(columns, rows).summary_row(list![
    Text::new("Total"),
    Text::new(format_amount(totals.budget)).id("total-budget"),
    Text::new(format_amount(totals.spent)).id("total-spent"),
])
```

## Split views

`SplitView` places two panes on either side of a divider. Dragging the
divider resizes the first pane within `range`; the second pane takes the rest.
Nest split views for more panes.

```rust,ignore
SplitView::horizontal(NoteList::new(), Editor::new())
    .position(260.0)       // the first pane's starting width
    .range(200.0..=420.0)
    .bind(self.list_width) // optional; keeps the width in a Store
```

The divider is a `Canvas2D` that shows a resize cursor and draws in the
platform's separator color. It keeps the drag in progress itself, so the drag
continues when a parent re-renders the split view. Split views take the shared
modifiers, such as `id` and `accessibility_label`.

## Context menus and popovers

`context_menu` and `popover` attach to any built-in view through the `Attach`
trait. Menu items without an action are disabled; standard items such as
`StandardAction::Copy` use the responder chain, so AppKit enables them.

```rust,ignore
Text::new("Item").context_menu([
    MenuItem::new("Rename").on_select(|cx| { /* … */ }),
    MenuItem::separator(),
    MenuItem::standard(StandardAction::Copy),
]);

Button::new("Info")
    .on_click(move |cx| cx.update(info, |shown| *shown = true))
    .popover(Popover::new(Text::new("Details")).bind_presented(info));
```

## Windows, sheets, and panels

Return several windows from the root view, for example with `list!`.
`Window::panel` creates a floating utility panel.

`size` sets the content area below the title bar; windows default to 640 × 480
and panels to 320 × 400. A new window opens centered unless `position` places
its top-left corner, in screen points from the top left of the main screen.
A frame saved with `autosave` takes precedence over both. Changing `size`
later keeps the top-left corner; a window the user moved or resized stays put
until its declaration changes.

```rust,ignore
Window::new("Editor")
    .size(Size::new(800.0, 600.0))
    .autosave("editor")
```

`bind_open` shows a window while its Store is `true`; closing it writes `false`. The app quits when its
last normal window closes; panels do not keep it running.

Alerts and file dialogs are window-modal sheets, and `ColorPanel` connects the
shared color panel. Each is shown while its bound Store is `true` and writes
`false` when AppKit dismisses it. A window shows one sheet at a time; a sheet
requested while another is open appears after it ends.

```rust,ignore
Window::new("Editor")
    .alert(
        Alert::new("Delete the item?")
            .button(AlertButton::new("Delete").role(ButtonRole::Destructive).on_click(delete))
            .button(AlertButton::new("Cancel").role(ButtonRole::Cancel))
            .bind_presented(self.confirm),
    )
    .file_dialog(FileDialog::open().multiple(true).bind_presented(self.open).on_select(|cx, paths| { /* … */ }))
    .color_panel(ColorPanel::new().bind(self.tint).bind_presented(self.colors));

Window::panel("Inspector").bind_open(self.inspector)
```

Only one `ColorPanel` should be presented at a time because AppKit shares it.

## Quitting

The app quits when its last normal window closes or the user chooses Quit.
`App::on_exit` runs once before the process ends on either path; use it to
finish saving. On macOS, Quit (⌘Q) exits the process from inside AppKit
without returning from `App::run`, so code after `run` alone is not enough.

```rust,ignore
App::new(Root::new(saver.clone()))
    .on_exit(move || saver.flush())
    .run()
```

## Main menu

Declare a `MainMenu` at the root next to the windows. `MainMenu::standard()`
builds the App, File, Edit, View, Window, and Help menus with conventional
shortcuts and responder-chain actions; AppKit fills Services, the window list,
and Help search. Without a declaration, the standard English menu is used.

```rust,ignore
list![
    MainMenu::standard()
        .app_name("Notes")
        .settings(|cx| { /* … */ })
        .extend(StandardMenu::File, [MenuItem::new("New Note").shortcut(Shortcut::command("n"))])
        .menu(Menu::new("Format", [MenuItem::new("Bold").on_select(|cx| { /* … */ })])),
    Window::new("Notes"),
]
```

The standard Edit menu ends with a Find submenu (Find…, Find and Replace…,
Find Next, Find Previous, Use Selection for Find) that drives the focused text
view's find bar. An app without document text to search can handle Find…
(⌘F) itself with `MainMenu::on_find`, for example to focus a toolbar search
field; the item keeps its title, position, and shortcut, and the other Find
items still act on the focused text view.

`StandardAction::Bold`, `Italic`, and `Underline` apply font traits to the
selection of a rich text view; put them in your own Format menu.

Standard titles are resolved through a `MenuLocalizer`. Each `StandardText`
has a stable catalog key (`StandardText::key`, such as `menu.edit.copy`).
`EnglishMenuLocalizer` is the default and `JapaneseMenuLocalizer` is provided;
implement the trait to supply other translations.

On Windows the menus become a menu bar in each normal window. App menu items
such as Settings and Quit (shown as Exit) join the File menu, editing items act
on the focused text box, and macOS-only items such as Hide and Services are
omitted. Shortcuts use Control in place of Command; only letter and digit keys
are shown, because WinUI menus cannot display OEM keys such as the comma.

## Toolbar

`Window::toolbar` shows buttons with SF Symbols, flexible space, and a search
field bound to a Store. With a sidebar, the sidebar toggle comes first and the
items sit over the content. Changing only enabled states or search text
updates the items in place.

```rust,ignore
Window::new("Notes")
    .autosave("notes.main") // remembers the frame between launches
    .toolbar(Toolbar::new([
        ToolbarItem::button("new", "New Note", "square.and.pencil").on_click(new_note),
        ToolbarItem::flexible_space(),
        ToolbarItem::search(self.query).placeholder("Search"),
    ]))
```

`focus_on(trigger)` moves keyboard focus to a `TextEditor`, `TextField`,
`IntegerField`, `NumberField`, or `ToolbarItem::search` each time the
`Trigger` fires, for example after creating a note. Create it in
`View::init` with `cx.trigger()` and fire it with `cx.fire(trigger, ())`.
`ScrollView::scroll_to` takes a `Trigger<ViewId>` fired with a view ID.

## Sidebar badges and menus

`SidebarItem::badge` shows trailing text such as a count, and
`SidebarItem::context_menu` shows a menu on secondary click. Rows in a `List`
take `context_menu` like any view; the table shows the clicked row's menu.

```rust,ignore
SidebarItem::new(Place::Folder(folder.id), folder.name)
    .badge(count.to_string())
    .context_menu([MenuItem::new("Rename Folder…").on_select(rename)])
```

## Sidebar search

`Sidebar::searchable(query)` adds a search field above the items. Filter the
sections from the same Store; updates keep the field focused.

See `examples/gallery.rs` for every control, presentation, and menu.

## Canvas

`Canvas2D` draws with the platform's 2D API (Core Graphics on macOS) outside
the view update cycle, like `MTKView` or HTML Canvas. A `Canvas2DHandler` draws
frames and handles input on the main thread.

```rust,ignore
#[derive(Clone)]
struct Chart { points: Vec<f64>, hover: Option<usize> }

impl Canvas2DHandler for Chart {
    fn draw(&mut self, g: &mut Graphics2D<'_>, frame: &Frame) {
        g.clear(Color::WHITE);
        let mut path = Path::new();
        for (i, y) in self.points.iter().enumerate() {
            path.line_to(Point::new(i as f64 * 10.0, frame.height() - y));
        }
        g.stroke_path(&path, Color::rgb(0.2, 0.4, 1.0), &StrokeStyle::new(2.0));
    }

    fn event(&mut self, event: &CanvasEvent, cx: &mut CanvasEventContext) -> bool {
        if let CanvasEvent::Pointer(pointer) = event {
            self.hover = Some((pointer.location.x / 10.0) as usize);
            cx.invalidate(); // redraw without rebuilding views
            return true;
        }
        false
    }

    // Keep handler-owned state such as `hover` when the view re-renders.
    fn reconcile(&mut self, next: Self) -> Redraw {
        self.points = next.points;
        Redraw::Yes
    }
}

cx.watch(self.samples, |samples| {
    Canvas2D::new(Chart { points: samples.clone(), hover: None })
        .redraw(RedrawPolicy::OnDemand)
        .focusable(true)
        .fill_width()
        .grow()
})
```

The canvas draws again after resizing, after a `reconcile` that returns
`Redraw::Yes`, and after `CanvasEventContext::invalidate`. `Invalidator` (from
`Canvas2DHandler::setup`) requests a redraw from any thread.
`RedrawPolicy::Continuous` draws every display refresh with a display link;
`Frame::time` and `Frame::delta` drive animation. Coordinates are points with a
top-left origin.

Each render creates a new handler. The mounted canvas keeps its first handler
and passes later ones to `reconcile`. Handlers of different types are different
views. Input reaches the handler first. Unconsumed input goes to the next
responder, and `on_event` then receives it with a `Context` for Store updates.
Focusable canvases take keyboard focus when clicked.

To fill a window, hide its title bar. On macOS only the window buttons stay
above the content; on Windows the content extends into the title bar.

Content starts below the title bar by default. A top-level view that calls
`ignores_safe_area()` extends under it instead; a canvas then reads
`Frame::safe_area` to keep text and controls clear of the title bar.

```rust,ignore
Window::new("Orbits")
    .title_bar(TitleBar::Hidden) // or TitleBar::Overlay to keep the title
    .content(Canvas2D::new(Orbits::new()).fill_width().grow().ignores_safe_area())
```

Clicks in the title bar area move the window and do not reach the views under
it. A canvas claims points there with `Canvas2DHandler::title_bar_hit`, for
example for tabs it draws; other views opt in as a whole with
`title_bar_interactive(true)` (from `Attach`). A hovering canvas sees `Exited`
while the pointer is in the title bar area outside the points it claims.

```rust,ignore
fn title_bar_hit(&self, point: Point) -> bool {
    self.tabs.iter().any(|tab| tab.contains(point))
}
```

On Windows the canvas is not drawn yet; see `TODO.md`. See
`examples/canvas.rs` for a sketch pad and `examples/fullscreen_canvas.rs` for a
window-filling animation.

## Custom views

Compose views into your own by implementing `View`. To give such a view the
shared modifiers (`id`, sizes, `context_menu`, `accessibility_*`), implement
`render::NativeView` instead, when its root is one native view such as a stack:

```rust,ignore
use zintl_ui_desktop::render::{Element, NativeView, ViewModifiers, modifiers, native_view};

#[derive(Clone)]
pub struct Card {
    title: String,
    modifiers: ViewModifiers,
}

impl NativeView for Card {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        VStack::new(list![Text::new(self.title.clone())]).padding(12.0).native_element(cx)
    }
}

native_view!(Card); // implements View
```

Modifiers on `Card` win over those its root stack declares; popovers of both
are kept. A view with its own `View::init` implements `View` by hand and is
mounted only as a child, never through another view's `native_element`.

State that a view keeps unless the app binds a Store goes in a `Local<T>`:
call `bind` from the builder, `init` from `View::init`, and read `store()`.

Apps cannot add native control kinds. Build views by composing existing ones,
or draw them with `Canvas2D` and report to the app with
`CanvasEventContext::emit` and `on_action`. Canvases read platform colors from
`Frame::palette`, including `separator` and `alternating_row`, and
`render::TextEstimate` estimates text sizes without the backend. See
`examples/custom_view.rs`.

## Crate responsibilities

```
zintl-ui-desktop ─┬─ zintl-ui-native ── zintl-ui-backend-core
                  ├─ [macOS]   zintl-ui-appkit-backend ── zpd-appkit
                  └─ [Windows] zintl-ui-winui3-backend ── zpd-winui3
```

- `zintl-ui-desktop` owns the cross-platform builders and picks the backend.
- `zintl-ui-graphics` owns `Graphics2D`, paths, paints, fonts, and images, and
  the `GraphicsBackend` trait that native canvases implement.
- `zintl-ui-native` owns the declarative views, Store bindings, and menus; views
  render `RenderNode` descriptions.
- `zintl-ui-backend-core` defines `RenderNode`, `Event`, and `Context`, and the
  node tree, layout, and `HeadlessBackend` that backends share.
- `zintl-ui-appkit-backend` (macOS) and `zintl-ui-winui3-backend` (Windows)
  synchronize render nodes with native views and report events.
- `zpd-appkit` and `zpd-winui3` own the native controls, their lifetime, and
  native events. They define their own value types, so they do not depend on
  the UI crates.
- `zintl-ui` and `zintl-ui-layout` provide composition and layout primitives.

Tests, and platforms other than macOS and Windows, use `HeadlessBackend`, which
retains render nodes without native views.

See `examples/app.rs` for a complete application.
