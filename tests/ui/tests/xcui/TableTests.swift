import XCTest

@MainActor
final class TableTests: ZintlUITestCase {
  /// Verifies the table shows column headers and lines cells up under them.
  func testColumnsHaveHeadersAndAlignedCells() throws {
    try launch(scenario: "table")

    let table = app.tables["sections"]
    XCTAssertTrue(table.waitToExist(timeout: 2))
    // The ID names the table only, not the scroll view around it.
    XCTAssertFalse(app.scrollViews["sections"].exists)
    XCTAssertTrue(table.buttons["Name"].exists || table.staticTexts["Name"].exists)
    let first = app.staticTexts["length-1"]
    let second = app.staticTexts["length-2"]
    XCTAssertEqual(first.frame.minX, second.frame.minX, accuracy: 1)
    XCTAssertGreaterThan(first.frame.minX, app.staticTexts["name-1"].frame.maxX)
  }

  /// Verifies a declared column width is the rendered width, and the last column fills the rest.
  func testColumnWidthsMatchTheDeclaredWidths() throws {
    try launch(scenario: "table")

    let table = app.tables["sections"]
    XCTAssertTrue(table.waitToExist(timeout: 2))
    let name = table.buttons["Name"]
    let length = table.buttons["Length"]
    XCTAssertEqual(name.frame.width, 200, accuracy: 1)
    XCTAssertGreaterThanOrEqual(length.frame.width, 119)
    // The table fills the window's width, which the columns add up to.
    let window = app.windows["main-window"]
    XCTAssertEqual(name.frame.width + length.frame.width, window.frame.width, accuracy: 1)
  }

  /// Verifies clicking a row writes its ID to the bound Store.
  func testClickingARowSelectsIt() throws {
    try launch(scenario: "table")

    let selection = app.staticTexts["selection"]
    app.staticTexts["name-3"].click()
    XCTAssertTrue(waitForValue(selection, "Selected: section-3"))
  }

  /// Verifies a selection set by the app scrolls its row into view.
  func testProgrammaticSelectionScrollsToTheRow() throws {
    try launch(scenario: "table")

    let row = app.staticTexts["name-40"]
    XCTAssertFalse(row.exists && row.isHittable)
    app.buttons["select-40"].click()
    XCTAssertTrue(waitForValue(app.staticTexts["selection"], "Selected: section-40"))
    XCTAssertTrue(row.waitToBeHittable(timeout: 2))
  }
}
