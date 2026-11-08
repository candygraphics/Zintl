import XCTest

@MainActor
final class WindowsTests: ZintlUITestCase {
  /// Returns the inspector panel.
  private var inspector: XCUIElement {
    app.windows["inspector"]
  }

  /// Verifies the panel floats above the main window even after the main window is clicked.
  func testPanelFloatsAboveTheMainWindow() throws {
    try launch(scenario: "windows")

    let content = inspector.staticTexts["inspector-content"]
    XCTAssertTrue(content.waitForExistence(timeout: 2))
    app.staticTexts["main-window-top"].click()
    XCTAssertTrue(content.isHittable)
    // The panel's corner over the main window still belongs to the panel.
    let main = app.windows["main-window"].frame
    XCTAssertLessThan(inspector.frame.minX, main.maxX)
    XCTAssertTrue(inspector.isHittable)
  }

  /// Verifies closing the panel writes its Store and runs on_close, and the Store reopens it.
  func testClosedPanelReopensFromItsStore() throws {
    try launch(scenario: "windows")

    XCTAssertTrue(inspector.waitForExistence(timeout: 2))
    inspector.buttons[XCUIIdentifierCloseWindow].click()
    XCTAssertTrue(waitUntil { !self.inspector.exists })
    assertLabel("inspector-value", "Inspector: false")
    assertLabel("closes", "Closes: 1")

    app.buttons["open-inspector"].click()
    XCTAssertTrue(inspector.waitForExistence(timeout: 2))
    assertLabel("inspector-value", "Inspector: true")
    XCTAssertTrue(inspector.staticTexts["inspector-content"].exists)
  }

  /// Returns how far the top label of window `id` sits below the window's top edge.
  private func contentInset(_ id: String) -> CGFloat {
    app.staticTexts["\(id)-top"].frame.minY - app.windows[id].frame.minY
  }

  /// Verifies content starts below the title bar by default, even with an overlay title bar,
  /// and extends under overlay and hidden title bars once it ignores the safe area.
  func testTitleBarStylesPlaceContent() throws {
    try launch(scenario: "windows")

    XCTAssertTrue(app.windows["overlay-safe-window"].waitForExistence(timeout: 2))
    XCTAssertGreaterThan(contentInset("main-window"), 20)
    XCTAssertGreaterThan(contentInset("overlay-safe-window"), 20)
    XCTAssertLessThan(contentInset("overlay-window"), 10)
    XCTAssertLessThan(contentInset("hidden-window"), 10)
  }

  /// Verifies the overlay title bar keeps the title, while the hidden title bar hides it.
  func testHiddenTitleBarHidesTheTitle() throws {
    try launch(scenario: "windows")

    let overlay = app.windows["overlay-window"]
    let hidden = app.windows["hidden-window"]
    XCTAssertTrue(hidden.waitForExistence(timeout: 2))
    XCTAssertTrue(overlay.staticTexts["Zintl overlay-window"].exists)
    XCTAssertFalse(hidden.staticTexts["Zintl hidden-window"].exists)
    // Both keep the window buttons.
    XCTAssertTrue(hidden.buttons[XCUIIdentifierCloseWindow].exists)
  }
}
