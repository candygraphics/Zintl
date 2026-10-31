# Notes example plan

An Apple Notes–style app built only with `zintl-ui-desktop`, used to drive the
desktop API toward what a real document app needs. macOS (AppKit) comes first.
Windows follows once each API exists on both backends.

```sh
cd examples && cargo run --bin notes
```

## Target

| Area | Apple Notes behavior |
| --- | --- |
| Layout | Three columns: folders sidebar, note list, editor; resizable dividers; collapsible sidebar |
| Sidebar | Smart folders (All Notes, Recently Deleted), user folders with note counts, nested folders, tags, "New Folder" button, rename/delete context menu, drop notes onto folders |
| Note list | Sections (Pinned, Today, Previous 7 Days, …); two-line rows (bold title, date + preview in secondary color); selection with arrow keys; context menu (Pin, Move, Delete); list/gallery toggle |
| Editor | Rich text: Title/Heading/Subheading/Body/Monospaced, bold/italic/underline/strikethrough, bulleted/dashed/numbered lists, checklists, links, tables, attachments; first line is the title; autosave |
| Toolbar | Sidebar toggle, list/gallery, delete, new note, format (Aa) menu, checklist, table, attachment, lock, share, search field |
| Search | Toolbar search across all notes, highlighting matches |
| Menus | File › New Note (⌘N), New Folder (⇧⌘N); Format › Title/Heading/Body, Checklist (⇧⌘L), Bold/Italic/Underline; Edit › Find |
| Persistence | Notes on disk, saved after edits and on quit; window frame restored |

Out of scope: iCloud sync, sharing/collaboration, locking, drawing, scanning.

## Phases

1. **Skeleton (done)** — model (`src/model.rs`) with tests; sidebar folders;
   note list as buttons in a `ScrollView`; plain multiline `TextField` editor
   with a draft committed on selection change; File menu items.
2. **Layout** — three-column split, list rows with styled text, column widths.
   Needs A1–A4. A1, A2, A3, A5, and A6 are done, and A4 is partly done.
3. **List** — native selectable list with sections, keyboard navigation, and
   row context menus. Needs B1–B3. B1 is done.
4. **Editing** — live autosave, rich text editor, Format menu, checklists.
   Needs C1–C5.
5. **Toolbar and search** — toolbar items, toolbar search field, focus control.
   Needs D1–D3.
6. **Sidebar polish** — counts, folder editing, nested folders, drag and drop.
   Needs E1–E4.
7. **Persistence and lifecycle** — JSON files under Application Support,
   debounced saving, save on quit, window frame restoration. Needs F1–F3.
8. **Windows** — run the same app on WinUI 3.

## Missing APIs

Found while writing phase 1 (see `TODO` comments in `src/main.rs`).

### A. Layout and styling

- **A1 Split view (done, simple).** `SplitView` is a regular view: two panes
  and a `Canvas2D` divider that resizes the first pane while dragging. The
  list and editor use it inside the window content. Not done yet: a resize
  cursor, and a native `NSSplitView` column with collapse and autosave.
  Formerly: `Window` has only `sidebar` +
  `content`. Notes needs a third, resizable column (`NSSplitViewController`
  with a content-list item, like SwiftUI `NavigationSplitView`). Today the list
  and editor sit in an `HStack` with no divider and widths decided by content.
- **A2 Frame sizing (done).** Stacks take `width` and `height`.
  Formerly: No fixed/min/max width or height on stacks and controls
  (only `minimum_size`, `fill_width`, `grow`). Needed for the list column
  (~250 pt) and the editor filling the remaining height.
- **A3 Text styling (done).** `Text::font`, `color` (`TextColor`), `alignment`,
  `line_limit`, and `fill_width`; AppKit only for now. Formerly: `Text` has no font, weight, size, color (primary /
  secondary / tertiary), alignment, line limit, or truncation.
- **A4 Primitives.** Done: `Spacer` and stack `alignment` (`Center` and `End`
  were added to `CrossAxisAlignment`). Remaining: `Divider`, a standalone
  `Image` (an SF Symbol such as the pin), and button title alignment and fonts
  (a borderless button centers its title within its minimum width).
- **A5 Re-export `KeyedElement` (done).**
- **A6 Native measuring (done).** AppKit used to lay out leaves at declared
  estimates (`Button::new` is 80×32), so longer titles wrapped. The AppKit
  backend now raises minimums to `intrinsicContentSize` for buttons, labels,
  switches, pop-ups, and segmented controls, as WinUI already did.

### B. Lists

- **B1 `List` view (done).** `List`/`ListSection`/`ListRow`: an `NSTableView`
  hosting arbitrary row views, single selection bound to `Store<Option<String>>`,
  group-row headers, inset or plain style. Not yet: multiple selection, lazy
  row creation for large lists. Formerly: A native `NSTableView`/`NSOutlineView` with arbitrary row
  views, a selection binding (`Store<Option<Id>>` and multiple selection),
  sections with headers, keyboard navigation, and source-list or inset styles.
  `Sidebar` covers only string items.
- **B2 Row actions.** Context menu with the clicked row's ID, double-click,
  delete key, swipe actions.
- **B3 Lazy grid** for the gallery view (lower priority).

### C. Text editing

- **C1 Change events.** `TextField` has `bind` but no `on_change`, so the
  app cannot autosave, update the list title live, or bump the modified date.
  Phase 1 commits the draft only when the selection changes.
- **C2 Growing multiline editor.** A multiline `TextField` shows one line and
  does not fill the column; it should be a scrolling `NSTextView`.
- **C3 Rich text model.** An `AttributedText` (runs with font, traits,
  paragraph style, lists, checklist items, links) bound to the editor, with a
  serialization the app can persist.
- **C4 Formatting commands.** Apply styles to the selection from menus and
  toolbar; standard actions for Bold, Italic, Underline, Strikethrough, and font
  panel through the responder chain; typing attributes; selection state for
  enabling and checking menu items.
- **C5 Find.** `NSTextFinder`-backed Find/Replace bar (`StandardAction::Find`).

### D. Toolbar and focus

- **D1 Toolbar.** `Window::toolbar` with items (buttons with SF Symbols, menus,
  segmented controls, flexible space) and placement relative to split columns
  (`NSTrackingSeparatorToolbarItem`). Only the automatic sidebar toggle exists.
- **D2 Toolbar search.** `NSSearchToolbarItem`; `Sidebar::searchable` places
  the field in the sidebar only.
- **D3 Focus control.** Programmatic focus (`focus(Store<Option<FocusId>>)`) so a
  new note focuses the editor and ⌥⌘F focuses search. Only canvases are
  focusable today.

### E. Sidebar

- **E1** Trailing badge text on `SidebarItem` (note counts).
- **E2** Per-item context menus and inline renaming.
- **E3** Nested items with disclosure.
- **E4** Drag and drop: drag sources on list rows, drop targets on sidebar
  items, with typed payloads.

### F. App lifecycle and async

- **F1 Timers and background results.** `Context::perform_main` runs a closure
  without a `Context`, so nothing outside an event handler can update a Store.
  Needed for debounced autosave and background loading
  (e.g. `cx.after(duration, |cx| …)`, `cx.spawn` with a main-thread callback).
- **F2 App lifecycle.** `on_terminate` / will-resign-active hooks to flush
  saves; the app name shown in the menu bar comes from the binary (`notes`)
  instead of `MainMenu::app_name`; it probably needs a bundle or
  `NSProcessInfo.processName`.
- **F3 Window state.** Frame autosave name and restoring the split column
  widths and sidebar collapse state.
- **F4 Menu item state.** Checked state and validation for menu items bound to
  Stores (Format menu checkmarks, disabling Delete without a selection).
