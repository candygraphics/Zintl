import XCTest

@MainActor
final class WgpuInputTests: ZintlUITestCase {
  private var canvasPoint: XCUICoordinate {
    app.windows["main-window"].coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.75))
  }

  /// Counts the window's pixels close to `red`, `green`, `blue`.
  private func pixels(red: CGFloat, green: CGFloat, blue: CGFloat) -> Int {
    pixelCount(in: app.windows["main-window"]) { color in
      abs(color.redComponent - red) < 0.2 && abs(color.greenComponent - green) < 0.2
        && abs(color.blueComponent - blue) < 0.2
    }
  }

  /// Verifies clicks and keys on a focused wgpu canvas reach on_event.
  func testClicksAndKeysReachOnEvent() throws {
    try launch(scenario: "wgpu-input")

    canvasPoint.click()
    assertLabel("clicks", "Clicks: 1")
    app.typeKey("w", modifierFlags: [])
    assertLabel("key", "Key: w")
  }

  /// Verifies a Store change renders the canvas again in the new color.
  func testStoreChangeRedrawsTheCanvas() throws {
    try launch(scenario: "wgpu-input")

    XCTAssertTrue(waitUntil(timeout: 5) { self.pixels(red: 0, green: 0, blue: 1) > 1000 })
    XCTAssertEqual(pixels(red: 1, green: 0, blue: 0), 0)

    app.buttons["paint-red"].click()
    XCTAssertTrue(waitUntil(timeout: 5) { self.pixels(red: 1, green: 0, blue: 0) > 1000 })
    XCTAssertLessThan(pixels(red: 0, green: 0, blue: 1), 100)
  }
}
