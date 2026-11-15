import XCTest

@MainActor
final class OverflowTests: ZintlUITestCase {
  /// Verifies a split view's first pane keeps its position although the second pane's content is wider.
  func testSplitPaneKeepsItsWidthBesideWideContent() throws {
    try launch(scenario: "overflow")

    let window = app.windows["main-window"]
    let detail = app.staticTexts["detail"]
    XCTAssertTrue(detail.waitToExist(timeout: 2))
    // The first pane is 200 points wide, followed by a 6-point divider.
    XCTAssertEqual(detail.frame.minX - window.frame.minX, 206, accuracy: 1)
  }

  /// Verifies grid cells holding stacks keep their content's width in a horizontally scrolling view.
  func testGridCellStacksDoNotOverlapTheNextColumn() throws {
    try launch(scenario: "overflow")

    for row in 1...2 {
      let first = app.popUpButtons["first-picker-\(row)"]
      let second = app.popUpButtons["second-picker-\(row)"]
      let after = app.staticTexts["after-\(row)"]
      XCTAssertTrue(after.waitToExist(timeout: 2))
      XCTAssertLessThanOrEqual(first.frame.maxX, second.frame.minX, "row \(row)")
      XCTAssertLessThanOrEqual(second.frame.maxX, after.frame.minX, "row \(row)")
    }
  }

  /// Verifies a horizontally scrolling view can scroll to content wider than itself.
  func testHorizontalScrollReachesTheLastColumn() throws {
    try launch(scenario: "overflow")

    let scrollView = app.scrollViews["table-scroll"]
    let after = app.staticTexts["after-1"]
    XCTAssertTrue(after.waitToExist(timeout: 2))
    // The scroll view stays in its pane although the toolbar above it is wider.
    XCTAssertLessThanOrEqual(scrollView.frame.maxX, app.windows["main-window"].frame.maxX)
    XCTAssertGreaterThan(after.frame.maxX, scrollView.frame.maxX)

    scrollView.scroll(byDeltaX: -1000, deltaY: 0)
    XCTAssertLessThanOrEqual(after.frame.maxX, scrollView.frame.maxX)
  }

  /// Verifies fields that do not fit their pane are cut to it instead of covering the next pane.
  func testOverflowingFieldsDoNotOverlap() throws {
    try launch(scenario: "overflow-fields")

    let fields = (1...4).map { app.textFields["field-\($0)"] }
    XCTAssertTrue(fields[3].waitToExist(timeout: 2))
    for (index, field) in fields.enumerated() {
      for other in fields[(index + 1)...] {
        XCTAssertFalse(
          field.frame.intersects(other.frame),
          "\(field.identifier) \(field.frame) covers \(other.identifier) \(other.frame)"
        )
      }
    }
    // The first pane is 200 points wide.
    let window = app.windows["main-window"]
    XCTAssertLessThanOrEqual(fields[0].frame.maxX, window.frame.minX + 200 + 1)
  }

  /// Verifies typing into each visible field reaches that field's own Store.
  func testTypingReachesTheClickedField() throws {
    try launch(scenario: "overflow-fields")

    for number in [1, 2, 4] {
      let field = app.textFields["field-\(number)"]
      XCTAssertTrue(field.waitToExist(timeout: 2))
      field.click()
      field.typeText("t\(number)")
      assertLabel("value-\(number)", "\(number):t\(number)")
    }
  }
}
