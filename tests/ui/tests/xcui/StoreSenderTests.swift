import XCTest

@MainActor
final class StoreSenderTests: ZintlUITestCase {
  /// Verifies a worker thread's Store write re-renders the view without further input.
  func testWorkerThreadUpdatesTheView() throws {
    try launch(scenario: "store-sender")

    let status = app.staticTexts["build-status"]
    app.buttons["build"].click()
    XCTAssertTrue(waitForValue(status, "Built", timeout: 5))
  }
}
