import XCTest

@MainActor
final class WgpuFirstFrameTests: ZintlUITestCase {
  /// Verifies an on-demand wgpu canvas sets up and renders once its window appears, without a resize.
  func testOnDemandCanvasRendersItsFirstFrame() throws {
    try launch(scenario: "wgpu-first-frame")

    let status = app.staticTexts["render-status"]
    XCTAssertTrue(status.waitToExist(timeout: 2))
    XCTAssertTrue(waitForValue(status, "Rendered", timeout: 5))
  }
}
