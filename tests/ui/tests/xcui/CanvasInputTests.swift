import XCTest

@MainActor
final class CanvasInputTests: ZintlUITestCase {
  private var canvasPoint: XCUICoordinate {
    app.windows["main-window"].coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.75))
  }

  /// Verifies the pointer entering and leaving the canvas reaches the handler.
  func testHoverEntersAndExits() throws {
    try launch(scenario: "canvas-input")

    canvasPoint.hover()
    assertLabel("hover", "Hover: true")
    app.staticTexts["clicks"].hover()
    assertLabel("hover", "Hover: false")
  }

  /// Verifies a click focuses the canvas so typed keys reach the handler.
  func testFocusedCanvasReceivesKeys() throws {
    try launch(scenario: "canvas-input")

    canvasPoint.click()
    assertLabel("clicks", "Clicks: 1")
    app.typeKey("k", modifierFlags: [])
    assertLabel("key", "Key: k")
  }

  /// Verifies scrolling over the canvas reaches the handler.
  func testScrollReachesTheHandler() throws {
    try launch(scenario: "canvas-input")

    canvasPoint.scroll(byDeltaX: 0, deltaY: -40)
    let scrolls = app.staticTexts["scrolls"]
    XCTAssertTrue(waitUntil { (scrolls.value as? String) != "Scrolls: 0" })
  }

  /// Verifies a Store-driven render reconciles into the mounted handler and keeps its state.
  func testHandlerStateSurvivesRerender() throws {
    try launch(scenario: "canvas-input")

    canvasPoint.click()
    assertLabel("clicks", "Clicks: 1")
    app.buttons["rerender"].click()
    canvasPoint.click()
    assertLabel("clicks", "Clicks: 2")
  }
}
