# TODO

## Canvas2D: follow-ups

- IME and marked text for canvases that accept text (`NSTextInputClient` on
  macOS, `CoreTextEditContext` on Windows). WinUI canvases only receive
  committed characters.
- An opt-in render thread (`Canvas2D::threaded`) for heavy drawing.

## Accessibility: follow-ups

- Accessibility trees inside canvases. `Canvas2DHandler` and
  `WgpuCanvasHandler` would return virtual nodes (frame, role, label, value,
  actions such as press or increment), mapped to `NSAccessibilityElement`
  children on macOS and automation peers on Windows. VoiceOver actions would
  arrive as a `CanvasEvent`, and the tree would be rebuilt on `reconcile` or an
  explicit invalidation, posting a layout-changed notification. Until then a
  canvas can only carry one label, like an image.
- `SplitView` cannot be resized with VoiceOver or Narrator, because its divider
  is a canvas.

## Windows follow-ups

- The toolbar's `CommandBar` sits in a row below the menu bar rather than
  inside the title bar. With an extended title bar the menu bar takes the
  title bar row, so content starts below it instead of under the title bar.
- `ListStyle::Inset` and `ListStyle::Plain` look the same in the WinUI 3
  `ListView`.
- Rich text links are written with `ITextRange::Link` but have not been checked
  in a running app.
