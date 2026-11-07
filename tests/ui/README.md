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
