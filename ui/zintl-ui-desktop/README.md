# zintl-ui-desktop

Declarative desktop views rendered with AppKit on macOS and WinUI 3 on Windows.

Attach navigation to a window with `Window::sidebar`:

```rust,ignore
// Create this Store in View::init and retain it on the view.
self.selection = cx.store(Some("home".to_owned()));

// Build this window in View::render.
Window::new(Rect::new(100.0, 100.0, 640.0, 400.0), "Navigation")
    .sidebar(
        Sidebar::new([SidebarSection::new([
            SidebarItem::new("home", "Home").system_image("house"),
            SidebarItem::new("settings", "Settings").system_image("gearshape"),
        ]).title("Pages")])
        .bind(self.selection)
        .on_select(|_cx, id| println!("Selected: {id}")),
    )
    .content(cx.watch(self.selection, |selected| {
        Text::new(selected.as_deref().unwrap_or("No selection"))
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

`on_select` receives the selected item ID after the bound Store is updated. It
also works without `bind`. Initial/programmatic selection and deselection do not
invoke it; native item selection changes do.

Item IDs must be unique across the sidebar. `bind` reads and updates a
`Store<Option<String>>`; programmatic changes also update the native selection.
AppKit automatically adds a standard sidebar toggle to the window toolbar.
Removing the sidebar also removes the toolbar item it added; preexisting toolbar
items are preserved. Collapse state survives reactive updates.

Unknown selection events are ignored. Rebuilding a window without `.sidebar(...)`
removes its sidebar. SF Symbol names are interpreted by AppKit.

## Controls

Controls bind to Stores with `bind` and report changes with `on_change`.
Each renders a native control.

```rust,ignore
VStack::new(list![
    Toggle::new("Notifications").bind(self.notifications),            // NSSwitch
    Toggle::new("Agree").style(ToggleStyle::Checkbox).bind(self.agree),
    Toggle::new("Bold").style(ToggleStyle::Button).bind(self.bold),   // push on/push off
    Picker::new(["Small", "Medium"]).bind(self.size),                 // NSPopUpButton
    Picker::new(["List", "Grid"]).style(PickerStyle::Segmented).bind(self.layout),
    Picker::new(["Low", "High"]).style(PickerStyle::Radio).bind(self.priority),
    ComboBox::new(["Apple", "Banana"]).placeholder("Fruit").bind(self.fruit),
    Slider::new(0.0..=100.0).ticks(11).bind(self.volume),
    NumberField::new().range(0.0..=99.0).step(1.0).bind(self.quantity),
    ColorWell::new().bind(self.tint),
    SearchField::new().placeholder("Search").bind(self.query),
])
```

Buttons take a `ButtonStyle` (push, borderless, toolbar, help, …), a
`ButtonRole` (`Default` answers Return, `Cancel` answers Escape, `Destructive`
warns), and SF Symbols through `system_image`.

`ScrollView::new(content)` and `TabView::new([Tab::new("General", page)])`
fill their parent's width and the remaining space. Stacks accept `padding` and
`grow`.

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
    .popover(Popover::new(Text::new("Details")).bind(info));
```

## Windows, sheets, and panels

Return several windows from the root view, for example with `list!`.
`Window::panel` creates a floating utility panel. `bind_open` shows a window
while its Store is `true`; closing it writes `false`. The app quits when its
last normal window closes; panels do not keep it running.

Alerts and file dialogs are window-modal sheets, and `ColorPanel` connects the
shared color panel. Each is shown while its bound Store is `true` and writes
`false` when AppKit dismisses it. A window shows one sheet at a time; a sheet
requested while another is open appears after it ends.

```rust,ignore
Window::new(bounds, "Editor")
    .alert(
        Alert::new("Delete the item?")
            .button(AlertButton::new("Delete").role(ButtonRole::Destructive).on_click(delete))
            .button(AlertButton::new("Cancel").role(ButtonRole::Cancel))
            .bind(self.confirm),
    )
    .file_dialog(FileDialog::open().multiple().bind(self.open).on_select(|cx, paths| { /* … */ }))
    .color_panel(ColorPanel::new().bind_color(self.tint).bind(self.colors));

Window::panel(bounds, "Inspector").bind_open(self.inspector)
```

Only one `ColorPanel` should be presented at a time because AppKit shares it.

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
    Window::new(bounds, "Notes"),
]
```

Standard titles are resolved through a `MenuLocalizer`. Each `StandardText`
has a stable catalog key (`StandardText::key`, such as `menu.edit.copy`).
`EnglishMenuLocalizer` is the default and `JapaneseMenuLocalizer` is provided;
implement the trait to supply other translations.

On Windows the menus become a menu bar in each normal window. App menu items
such as Settings and Quit (shown as Exit) join the File menu, editing items act
on the focused text box, and macOS-only items such as Hide and Services are
omitted. Shortcuts use Control in place of Command; only letter and digit keys
are shown, because WinUI menus cannot display OEM keys such as the comma.

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

```rust,ignore
Window::new(bounds, "Orbits")
    .title_bar(TitleBar::Hidden) // or TitleBar::Overlay to keep the title
    .content(Canvas2D::new(Orbits::new()).fill_width().grow())
```

Clicks in the title bar area move the window and do not reach the views under
it. Mark a view with `title_bar_interactive()` (from `Attach`) to keep mouse
input for it and its subviews there; a hovering canvas sees `Exited` while the
pointer is in that area.

```rust,ignore
Button::new("Share").title_bar_interactive()
```

On Windows the canvas is not drawn yet; see `TODO.md`. See
`examples/canvas.rs` for a sketch pad and `examples/fullscreen_canvas.rs` for a
window-filling animation.

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
