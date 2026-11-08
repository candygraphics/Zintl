import XCTest

@MainActor
final class IndicatorsTests: ZintlUITestCase {
  /// Returns the progress bar's fraction from its accessibility value.
  private func progress() -> Double? {
    (app.progressIndicators["progress"].value as? NSNumber)?.doubleValue
  }

  /// Verifies the bar shows its fraction and follows the Store up to completion.
  func testProgressBarFollowsItsStore() throws {
    try launch(scenario: "indicators")

    XCTAssertTrue(app.progressIndicators["progress"].waitForExistence(timeout: 2))
    XCTAssertEqual(progress() ?? -1, 0.25, accuracy: 0.001)

    app.buttons["advance"].click()
    XCTAssertTrue(waitUntil { abs((self.progress() ?? -1) - 0.75) < 0.001 })
    app.buttons["advance"].click()
    XCTAssertTrue(waitUntil { abs((self.progress() ?? -1) - 1.0) < 0.001 })
  }

  /// Verifies a separator adds one point of line plus one stack gap between its neighbours.
  ///
  /// Each gap is compared with a neighbouring gap without a separator, which cancels out
  /// the labels' accessibility insets.
  func testSeparatorsSitBetweenTheirNeighbours() throws {
    try launch(scenario: "indicators")

    let text = { (id: String) in self.app.staticTexts[id].frame }
    XCTAssertTrue(app.staticTexts["above"].waitForExistence(timeout: 2))
    let separated = text("below").minY - text("above").maxY
    let plain = text("left").minY - text("below").maxY
    XCTAssertEqual(separated - plain, 9, accuracy: 0.5)

    let separatedRow = text("middle").minX - text("left").maxX
    let plainRow = text("extra").minX - text("middle").maxX
    XCTAssertEqual(separatedRow - plainRow, 9, accuracy: 0.5)
  }

  /// Verifies a spacer pushes the following label to the far edge of the row.
  func testSpacerPushesTheLastLabelToTheEdge() throws {
    try launch(scenario: "indicators")

    let window = app.windows["main-window"].frame
    let right = app.staticTexts["right"].frame
    XCTAssertEqual(right.maxX, window.maxX, accuracy: 2)
    XCTAssertGreaterThan(right.minX - app.staticTexts["extra"].frame.maxX, 100)
  }
}
