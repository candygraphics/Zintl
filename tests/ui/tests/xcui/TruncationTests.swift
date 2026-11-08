import XCTest

@MainActor
final class TruncationTests: ZintlUITestCase {
  /// Verifies a line-limited label that fits keeps its text's width instead of collapsing.
  func testLineLimitedLabelKeepsItsWidthWhenItFits() throws {
    try launch(scenario: "truncation")

    let label = app.staticTexts["short-label"]
    XCTAssertTrue(label.waitToExist(timeout: 2))
    XCTAssertGreaterThan(label.frame.width, 40)
  }

  /// Verifies a line-limited label that does not fit shrinks so the controls after it stay in the window.
  func testLongLineLimitedLabelShrinksBetweenControls() throws {
    try launch(scenario: "truncation")

    let window = app.windows["main-window"]
    let label = app.staticTexts["long-label"]
    let reload = app.buttons["reload"]
    XCTAssertTrue(label.waitToExist(timeout: 2))
    XCTAssertGreaterThan(label.frame.width, 40)
    XCTAssertLessThanOrEqual(label.frame.maxX, reload.frame.minX)
    XCTAssertLessThanOrEqual(reload.frame.maxX, window.frame.maxX)
  }
}
