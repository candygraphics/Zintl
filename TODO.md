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
- Cursor shape control from the handler.
- An opt-in render thread (`Canvas2D::threaded`) for heavy drawing.
