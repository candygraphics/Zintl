import XCTest

@MainActor
final class PopoverTests: ZintlUITestCase {
  /// Opens the popover and waits for its content.
  private func openPopover(file: StaticString = #filePath, line: UInt = #line) {
    app.buttons["info"].click()
    XCTAssertTrue(
      app.popovers.firstMatch.waitForExistence(timeout: 2), "The popover did not open",
      file: file, line: line)
    assertLabel("shown-value", "Shown: true", file: file, line: line)
  }

  /// Verifies setting the presented Store shows the popover with its content below the button.
  func testPresentingShowsTheContent() throws {
    try launch(scenario: "popover")

    XCTAssertFalse(app.popovers.firstMatch.exists)
    openPopover()
    let text = app.popovers.firstMatch.staticTexts["popover-text"]
    XCTAssertTrue(text.exists)
    XCTAssertGreaterThan(text.frame.minY, app.buttons["info"].frame.maxY)
  }

  /// Verifies Escape dismisses the popover, clears the Store, and runs on_close once.
  func testEscapeDismissesAndClearsTheStore() throws {
    try launch(scenario: "popover")

    openPopover()
    app.popovers.firstMatch.typeKey(.escape, modifierFlags: [])
    XCTAssertTrue(waitUntil { !self.app.popovers.firstMatch.exists })
    assertLabel("shown-value", "Shown: false")
    assertLabel("closes", "Closes: 1")
  }

  /// Verifies clicking outside the transient popover dismisses it and clears the Store.
  func testClickingOutsideDismisses() throws {
    try launch(scenario: "popover")

    openPopover()
    app.staticTexts["closes"].click()
    XCTAssertTrue(waitUntil { !self.app.popovers.firstMatch.exists })
    assertLabel("shown-value", "Shown: false")
    assertLabel("closes", "Closes: 1")
  }

  /// Verifies controls inside the popover write their Stores while it stays open.
  func testControlsInsideThePopoverWork() throws {
    try launch(scenario: "popover")

    openPopover()
    app.popovers.firstMatch.checkBoxes["pin"].click()
    assertLabel("pinned-value", "Pinned: true")
    XCTAssertTrue(app.popovers.firstMatch.exists)
  }

  /// Verifies clearing the Store from the app closes the popover.
  func testClearingTheStoreClosesThePopover() throws {
    try launch(scenario: "popover")

    openPopover()
    app.popovers.firstMatch.buttons["done"].click()
    XCTAssertTrue(waitUntil { !self.app.popovers.firstMatch.exists })
    assertLabel("shown-value", "Shown: false")

    // The popover opens again after an app-side close.
    openPopover()
  }
}
