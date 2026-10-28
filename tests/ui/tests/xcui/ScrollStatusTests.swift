import XCTest

@MainActor
final class ScrollStatusTests: ZintlUITestCase {
  /// Verifies a bound scroll view saves its offset and restores it after being recreated.
  func testScrollPositionSurvivesRecreation() throws {
    try launch(scenario: "scroll-status")

    let rows = app.scrollViews["rows"]
    let offset = app.staticTexts["scroll-offset"]
    XCTAssertTrue(rows.waitForExistence(timeout: 2))
    XCTAssertEqual(offset.value as? String, "Offset: 0")

    // The label shows the bound Store, so a change proves native scrolling reached Rust.
    rows.scroll(byDeltaX: 0, deltaY: -300)
    let scrolled = NSPredicate(format: "value != %@", "Offset: 0")
    wait(for: [expectation(for: scrolled, evaluatedWith: offset)], timeout: 2)
    let saved = offset.value as? String
    let marker = app.staticTexts["row-30"].frame

    // Hiding removes the native scroll view; showing creates a fresh one at offset zero.
    let toggle = app.buttons["toggle-list"]
    toggle.click()
    XCTAssertTrue(app.staticTexts["hidden"].waitForExistence(timeout: 2))
    toggle.click()
    XCTAssertTrue(rows.waitForExistence(timeout: 2))

    // The restored view must show the same rows without reporting a reset offset.
    XCTAssertEqual(offset.value as? String, saved)
    XCTAssertEqual(app.staticTexts["row-30"].frame.minY, marker.minY, accuracy: 1)
  }
}
