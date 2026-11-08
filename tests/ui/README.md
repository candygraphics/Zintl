# Zintl UI tests

The macOS UI tests build a dedicated Rust fixture app, package it as an app
bundle, and run XCUITest against it.

Run every UI test from the repository root:

```sh
./tests/ui/tools/scripts/test.sh
```

Run one test class:

```sh
./tests/ui/tools/scripts/test.sh \
  -only-testing:ZintlUITests/TextFieldTests
```

Rust fixture scenarios live under `tests/app/scenarios`. XCUITest sources live
under `tests/xcui`; new Swift files in that directory are discovered by
`tests/ZintlUITests.xcodeproj` automatically.

Each test switches to an ASCII keyboard and restores the previous input source
afterwards, because input methods such as Japanese change typed characters and
show a mode indicator that XCUITest treats as an interrupting dialog. Use
`paste(_:into:)` for text such as paths that must arrive unchanged.

## Writing a scenario

1. Add a module under `tests/app/scenarios` and register its launch name in the
   `scenarios!` list in `tests/app/scenarios/mod.rs`.
2. Give the main window the ID `main-window`; `ZintlUITestCase.launch` waits for it.
3. Echo every Store the test drives in a `Text` with an ID such as
   `<name>-value`. Tests read it with `assertLabel`, which proves native input
   reached Rust.
4. Add buttons that write the Stores from the app side, so tests also check that
   a Store change reaches the native view.
5. Read test-provided paths and switches with `LaunchOptions::variable`; tests
   pass them through `launch(scenario:environment:)`.

Wait with `waitToExist`, `waitForValue`, `assertLabel`, or `waitUntil` instead
of `waitForExistence` or `XCTNSPredicateExpectation`; XCTest's own waits recheck
only once per second. Pass `exitsOnItsOwn: true` to `launch` when the app quits
by itself, so XCTest does not wait about 20 seconds for a crash report.

## Not covered by XCUITest

Some behavior cannot be driven or observed through XCUITest on macOS. These
cases rely on unit tests, or the UI tests check a weaker property:

- **Canvas pinch (`CanvasEvent::Magnify`)**: XCUITest on macOS cannot pinch, so
  only the unit tests in `zintl-ui-backend-core` cover it.
- **`.help` tooltip text**: accessibility exposes a tooltip as a help tag
  without its text. `HelpTests` checks that a tag appears next to the pointer,
  and only over views that set help.
- **Toolbar button enabled state**: toolbar buttons always report themselves as
  enabled to accessibility. `ToolbarTests` checks whether clicks run the action
  instead.
- **Color panel controls**: the panel's UI differs between macOS versions, so
  `ColorPanelTests` picks colors only through the hex field of its RGB sliders.
