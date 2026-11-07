# TODO

## Canvas2D: follow-ups

- IME and marked text for canvases that accept text (`NSTextInputClient` on
  macOS, `CoreTextEditContext` on Windows). WinUI canvases only receive
  committed characters.
- An opt-in render thread (`Canvas2D::threaded`) for heavy drawing.

## Windows follow-ups

- The toolbar's `CommandBar` sits in a row below the menu bar rather than
  inside the title bar. With an extended title bar the menu bar takes the
  title bar row, so content starts below it instead of under the title bar.
- `ListStyle::Inset` and `ListStyle::Plain` look the same in the WinUI 3
  `ListView`.
- Rich text links are written with `ITextRange::Link` but have not been checked
  in a running app.
