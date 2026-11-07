import XCTest

@MainActor
final class ListTests: ZintlUITestCase {
  /// Returns the table row that holds the row title `id`.
  private func row(_ id: String) -> XCUIElement {
    app.tableRows.containing(.staticText, identifier: "\(id)-title").firstMatch
  }

  /// Verifies section titles and rows appear in order.
  func testSectionsShowTitlesAndRows() throws {
    try launch(scenario: "list")

    XCTAssertTrue(app.staticTexts["alpha-title"].waitForExistence(timeout: 2))
    let pinned = app.staticTexts["Pinned"]
    let recent = app.staticTexts["Recent"]
    XCTAssertTrue(pinned.exists)
    XCTAssertTrue(recent.exists)
    let order = [pinned, app.staticTexts["alpha-title"], recent, app.staticTexts["bravo-title"]]
      .map(\.frame.minY)
    XCTAssertEqual(order, order.sorted())
  }

  /// Verifies clicking a row selects it and reports the pick once.
  func testClickingARowSelectsIt() throws {
    try launch(scenario: "list")

    app.staticTexts["bravo-title"].click()
    assertLabel("selection-value", "Selected: bravo")
    assertLabel("picks", "Picks: bravo")
    XCTAssertTrue(row("bravo").isSelected)
  }

  /// Verifies the arrow keys move the selection, skipping section headers.
  func testArrowKeysMoveTheSelection() throws {
    try launch(scenario: "list")

    app.staticTexts["alpha-title"].click()
    assertLabel("selection-value", "Selected: alpha")
    app.typeKey(.downArrow, modifierFlags: [])
    assertLabel("selection-value", "Selected: bravo")
    app.typeKey(.downArrow, modifierFlags: [])
    assertLabel("selection-value", "Selected: charlie")
    app.typeKey(.upArrow, modifierFlags: [])
    assertLabel("selection-value", "Selected: bravo")
  }

  /// Verifies the app can select and clear rows without reporting them as user picks.
  func testStoreChangesSelectAndClearRows() throws {
    try launch(scenario: "list")

    app.buttons["select-charlie"].click()
    XCTAssertTrue(waitUntil { self.row("charlie").isSelected })
    assertLabel("picks", "Picks: ")

    app.buttons["clear"].click()
    XCTAssertTrue(waitUntil { !self.row("charlie").isSelected })
    assertLabel("selection-value", "Selected: none")
    assertLabel("picks", "Picks: ")
  }
}
