import XCTest

@MainActor
final class SplitViewTests: ZintlUITestCase {
  /// The divider's hit area in points; the second pane starts after it.
  private let divider: CGFloat = 6

  /// Returns the second pane's distance from the window's left edge.
  ///
  /// A label's accessibility frame sits one point inside its layout frame.
  private func secondPaneOffset() -> CGFloat {
    app.staticTexts["top-pane"].frame.minX - app.windows["main-window"].frame.minX - 1
  }

  /// Drags the divider of a first pane `width` points wide by `distance` points.
  private func dragDivider(at width: CGFloat, by distance: CGFloat) {
    let window = app.windows["main-window"]
    let y = app.staticTexts["bottom-pane"].frame.midY - window.frame.minY
    let start = window.coordinate(withNormalizedOffset: .zero)
      .withOffset(CGVector(dx: width + divider / 2, dy: y))
    start.click(forDuration: 0.2, thenDragTo: start.withOffset(CGVector(dx: distance, dy: 0)))
  }

  /// Verifies dragging the divider resizes the first pane by the drag distance and writes the Store.
  func testDraggingTheDividerResizesTheFirstPane() throws {
    try launch(scenario: "split-view")

    XCTAssertTrue(app.staticTexts["top-pane"].waitToExist(timeout: 2))
    XCTAssertEqual(secondPaneOffset(), 200 + divider, accuracy: 1)

    dragDivider(at: 200, by: 60)
    assertLabel("width-value", "Width: 260")
    XCTAssertEqual(secondPaneOffset(), 260 + divider, accuracy: 1)
  }

  /// Verifies the divider stops at both ends of its range.
  func testDividerStaysInsideItsRange() throws {
    try launch(scenario: "split-view")

    XCTAssertTrue(app.staticTexts["top-pane"].waitToExist(timeout: 2))
    dragDivider(at: 200, by: 250)
    assertLabel("width-value", "Width: 300")

    dragDivider(at: 300, by: -280)
    assertLabel("width-value", "Width: 120")
    XCTAssertEqual(secondPaneOffset(), 120 + divider, accuracy: 1)
  }

  /// Verifies writing the bound Store from the app moves the divider.
  func testStoreChangeMovesTheDivider() throws {
    try launch(scenario: "split-view")

    app.buttons["reset-width"].click()
    assertLabel("width-value", "Width: 150")
    XCTAssertTrue(waitUntil { abs(self.secondPaneOffset() - (150 + self.divider)) <= 1 })
  }

  /// Verifies a vertical split starts at its declared size clamped into its range.
  func testVerticalSplitStartsClampedToItsRange() throws {
    try launch(scenario: "split-view")

    let top = app.staticTexts["top-pane"]
    let bottom = app.staticTexts["bottom-pane"]
    XCTAssertTrue(bottom.waitToExist(timeout: 2))
    // The split asks for 20 points, but its range starts at 60.
    XCTAssertEqual(bottom.frame.minY - top.frame.minY, 60 + divider, accuracy: 1)
  }
}
