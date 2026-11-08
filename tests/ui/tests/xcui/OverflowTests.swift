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
}
