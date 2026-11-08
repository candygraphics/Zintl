import XCTest

@MainActor
final class CanvasActionTests: ZintlUITestCase {
  private var canvasPoint: XCUICoordinate {
    app.windows["main-window"].coordinate(withNormalizedOffset: CGVector(dx: 0.3, dy: 0.6))
  }

  /// Verifies a click reaches the view as the action the canvas handler emitted.
  func testClickEmitsTapAction() throws {
    try launch(scenario: "canvas-action")

    let status = app.staticTexts["canvas-status"]
    XCTAssertTrue(status.waitToExist(timeout: 2))
    XCTAssertEqual(status.value as? String, "Idle")

    canvasPoint.click()
    XCTAssertTrue(waitForValue(status, "Tapped 1"))
  }

  /// Verifies a drag is reported once, when the button is released, with the handler's measured distance.
  func testDragEmitsDraggedActionOnRelease() throws {
    try launch(scenario: "canvas-action")

    let status = app.staticTexts["canvas-status"]
    XCTAssertTrue(status.waitToExist(timeout: 2))

    canvasPoint.click(
      forDuration: 0.2, thenDragTo: canvasPoint.withOffset(CGVector(dx: 100, dy: 0)))
    XCTAssertTrue(waitForValue(status, "Dragged 100"))
  }
}
