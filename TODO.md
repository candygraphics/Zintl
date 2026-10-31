# TODO

## Canvas2D: Direct2D backend for WinUI 3

`ViewKind::Canvas2D` currently renders as an empty `Canvas` panel in
`zintl-ui-winui3-backend` (see `ControlKind::Canvas` in `backend/native.rs`).
The AppKit backend is the reference implementation
(`zintl-ui-appkit-backend/src/backend/canvas.rs`, `zpd-appkit/src/ui/canvas.rs`).

Plan:

- **Surface**: a `SwapChainPanel` in `ZintlWinUI3Support` with a D3D11 device, a
  composition swap chain (`IDXGIFactory2::CreateSwapChainForComposition`,
  `DXGI_FORMAT_B8G8R8A8_UNORM`, premultiplied alpha), and an
  `ID2D1DeviceContext` whose target bitmap wraps the back buffer.
- **Scale and size**: handle `SizeChanged` and `CompositionScaleChanged`, call
  `ResizeBuffers`, and set the swap chain's inverse scale transform so drawing
  stays in DIPs.
- **Redraw**: `OnDemand` sets a dirty flag and enqueues one draw on the
  `DispatcherQueue` (coalesced like `setNeedsDisplay:`); `Continuous` draws from
  `CompositionTarget::Rendering`, honoring `preferred_fps` by skipping frames.
- **Device loss**: on `D2DERR_RECREATE_TARGET` / `DXGI_ERROR_DEVICE_REMOVED`,
  recreate the device and drop cached brushes, geometries, and bitmaps.
- **Drawing**: a C ABI (`zpd_winui3_canvas_*`) for `GraphicsBackend` operations:
  save/restore via drawing state blocks plus a layer stack for clips,
  `ID2D1PathGeometry` for paths, solid/linear/radial gradient brushes,
  `ID2D1StrokeStyle` for caps, joins, and dashes, DirectWrite `IDWriteTextLayout`
  for text and metrics, and `ID2D1Bitmap1` from RGBA8 (convert to premultiplied
  BGRA) cached by `Image::id`.
- **Input**: `PointerPressed/Moved/Released/Entered/Exited`,
  `PointerWheelChanged`, `KeyDown/KeyUp`, `CharacterReceived` for text, and
  `GotFocus/LostFocus`; map to `CanvasEvent` and send through the route only
  when one is attached. Set `IsTabStop` from `focusable`.
- Adopt handlers in `update` (already done) and call `CanvasNode::attach` with a
  host whose `invalidate` requests a draw and whose `set_focus` calls `Focus`.

The WinUI changes for `Canvas2D` were made on macOS and have not been compiled
for Windows yet.

## Title bar interactive views on Windows

`title_bar_interactive`, `Canvas2DHandler::title_bar_hit`, and the title bar
inset are ignored by the WinUI 3 backend (it passes an inset of 0). With
`ExtendsContentIntoTitleBar`:

- Report the title bar height (`AppWindow.TitleBar.Height`) to `HostLayout::layout`
  and to canvas frames as the safe area.
- Register marked views' bounds with
  `InputNonClientPointerSource::SetRegionRects(NonClientRegionKind::Passthrough, ...)`
  after each layout. Canvases need rectangles rather than a per-point query,
  so `title_bar_hit` may need a region-based companion.

## Canvas2D: follow-ups

- IME and marked text (`NSTextInputClient`) for canvases that accept text.
- Cursor shape control from the handler. `SplitView`'s divider needs a resize
  cursor (`NSCursor.resizeLeftRight`).
- An opt-in render thread (`Canvas2D::threaded`) for heavy drawing.

## Text styling on Windows

`TextStyle` (font, color, alignment, line limit on `ViewKind::TextField`) is
ignored by the WinUI 3 backend. Map it to `TextBlock`/`TextBox` `FontSize`,
`FontWeight`, `Foreground` (theme brushes for semantic colors),
`TextAlignment`, and `MaxLines` with `TextTrimming::CharacterEllipsis`.

## SplitView on Windows

`SplitView`'s divider is a `Canvas2D`, which does not draw or receive input on
WinUI 3 yet, so the split shows no divider there and cannot be resized.

## List on Windows

`ViewKind::List` falls back to a `ScrollViewer` that stacks the rows, with no
selection or section headers. Use a WinUI `ListView` with grouped items,
`SelectionMode::Single`, and row content hosted as item templates, and report
`Event::ListSelectionChanged`.

## TextEditor on Windows

`ViewKind::TextEditor` maps to a multiline `TextBox`, but its font and
read-only state are not applied yet, and it does not fill the column with
scrolling like `NSTextView`. Set `AcceptsReturn`, `TextWrapping::Wrap`,
`IsReadOnly`, `FontSize`/`FontWeight`, and vertical scrolling.

## Rich text on Windows

`TextEditor::bind_rich` edits plain text on WinUI 3: the backend shows
`AttributedText::text` in a `TextBox` and reports plain edits. Use a
`RichEditBox`, map paragraph styles to character formats, and report
`Event::RichTextChanged`. The paragraph style and font trait menu items are
omitted from WinUI menus.

## App exit on Windows

`App::on_exit` runs after the WinUI 3 message loop returns. It does not run
when Windows ends the session; handle `WM_QUERYENDSESSION`/`WM_ENDSESSION`
(or `Application.Suspending`) and call it there.

## Toolbar and window autosave on Windows

`RenderNode::Window::toolbar` and `autosave` are ignored by the WinUI 3
backend. Show toolbar items in a `CommandBar` in the title bar area, the search
item as an `AutoSuggestBox`, and save the window placement in app settings.
`TextEditor::focus_on` should call `Focus(FocusState::Programmatic)`.
