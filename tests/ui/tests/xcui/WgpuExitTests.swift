import XCTest

@MainActor
final class WgpuExitTests: ZintlUITestCase {
  /// Verifies quitting with ⌘Q after the last wgpu canvas was removed exits without a panic.
  func testQuitAfterRenderingDoesNotPanic() throws {
    try assertExitsWithoutPanic { app in
      app.typeKey("q", modifierFlags: .command)
    }
  }

  /// Verifies closing the last window after the last wgpu canvas was removed exits without a panic.
  func testClosingTheWindowAfterRenderingDoesNotPanic() throws {
    try assertExitsWithoutPanic { app in
      app.windows["main-window"].buttons[XCUIIdentifierCloseWindow].click()
    }
  }

  /// Renders and removes the wgpu canvas, ends the app with `quit`, and checks no panic was recorded.
  ///
  /// Removing the canvas leaves the app as the only owner of the GPU device,
  /// so the device is released while the app exits.
  private func assertExitsWithoutPanic(
    file: StaticString = #filePath,
    line: UInt = #line,
    _ quit: (XCUIApplication) -> Void
  ) throws {
    let marker = FileManager.default.temporaryDirectory
      .appendingPathComponent("zintl-ui-panic-\(UUID().uuidString)")
    addTeardownBlock {
      try? FileManager.default.removeItem(at: marker)
    }
    try launch(
      scenario: "wgpu-first-frame",
      environment: ["ZINTL_UI_TEST_PANIC_MARKER": marker.path],
      exitsOnItsOwn: true
    )
    XCTAssertTrue(
      waitForValue(app.staticTexts["render-status"], "Rendered", timeout: 5), file: file, line: line
    )

    app.buttons["remove-canvas"].click()
    XCTAssertTrue(
      app.buttons["remove-canvas"].waitToExist(timeout: 2), file: file, line: line)

    quit(app)

    XCTAssertTrue(app.wait(for: .notRunning, timeout: 5), file: file, line: line)
    let panic = try? String(contentsOf: marker, encoding: .utf8)
    XCTAssertNil(panic, "The app panicked while exiting: \(panic ?? "")", file: file, line: line)
  }
}
