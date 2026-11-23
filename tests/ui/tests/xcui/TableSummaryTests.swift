import XCTest

@MainActor
final class TableSummaryTests: ZintlUITestCase {
  /// Verifies the summary cells are found by ID and line up with their columns' row cells.
  func testSummaryCellsLineUpWithTheirColumns() throws {
    try launch(scenario: "table-summary")

    let total = app.staticTexts["total-budget"]
    XCTAssertTrue(total.waitToExist(timeout: 2))
    assertLabel("total-budget", "4650")
    let budget = app.staticTexts["budget-1"]
    XCTAssertEqual(total.frame.minX, budget.frame.minX, accuracy: 1)
    XCTAssertEqual(total.frame.width, budget.frame.width, accuracy: 1)
    XCTAssertEqual(
      app.staticTexts["total-spent"].frame.minX, app.staticTexts["spent-1"].frame.minX, accuracy: 1)
    // The summary sits below the rows, at the bottom of the window.
    XCTAssertGreaterThan(total.frame.minY, budget.frame.maxY)
    let window = app.windows["main-window"]
    XCTAssertLessThan(window.frame.maxY - total.frame.maxY, 30)
  }

  /// Verifies clicking the summary row neither changes the selection nor reports one.
  func testClickingTheSummaryDoesNotSelect() throws {
    try launch(scenario: "table-summary")

    let name = app.staticTexts["name-2"]
    XCTAssertTrue(name.waitToExist(timeout: 2))
    name.click()
    assertLabel("selection", "Selected: 2")
    assertLabel("selections", "Selections: 1")
    app.staticTexts["total-name"].click()
    app.staticTexts["total-budget"].click()
    assertLabel("selection", "Selected: 2")
    assertLabel("selections", "Selections: 1")
  }

  /// Verifies the last row scrolls above the summary instead of under it, and sorting leaves the summary in place.
  func testRowsScrollAboveTheSummary() throws {
    try launch(scenario: "table-summary")

    let total = app.staticTexts["total-name"]
    XCTAssertTrue(total.waitToExist(timeout: 2))
    let summaryTop = total.frame.minY
    app.tables["budget"].scroll(byDeltaX: 0, deltaY: -5000)
    let last = app.staticTexts["name-30"]
    XCTAssertTrue(waitUntil { last.exists && last.frame.maxY <= summaryTop + 1 })
    XCTAssertEqual(total.frame.minY, summaryTop, accuracy: 1)

    app.checkBoxes["descending"].click()
    XCTAssertTrue(waitUntil { app.staticTexts["name-1"].exists })
    XCTAssertEqual(app.staticTexts["total-name"].frame.minY, summaryTop, accuracy: 1)
    assertLabel("total-budget", "4650")
  }

  /// Verifies summary cells follow a column the user resizes.
  func testSummaryFollowsAResizedColumn() throws {
    try launch(scenario: "table-summary")

    let table = app.tables["budget"]
    let header = table.buttons["Category"]
    XCTAssertTrue(header.waitToExist(timeout: 2))
    let before = app.staticTexts["total-budget"].frame.minX
    // Drag the divider at the Category header's trailing edge to the right.
    let divider = header.coordinate(withNormalizedOffset: CGVector(dx: 1, dy: 0.5))
      .withOffset(CGVector(dx: -1, dy: 0))
    divider.click(forDuration: 0.1, thenDragTo: divider.withOffset(CGVector(dx: 40, dy: 0)))
    XCTAssertTrue(waitUntil { app.staticTexts["total-budget"].frame.minX > before + 20 })
    XCTAssertEqual(
      app.staticTexts["total-budget"].frame.minX, app.staticTexts["budget-1"].frame.minX,
      accuracy: 1)
  }
}
