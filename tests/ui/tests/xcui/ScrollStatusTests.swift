import XCTest

@MainActor
final class ScrollStatusTests: ZintlUITestCase {
  /// Verifies a bound scroll view saves its offset and restores it after being recreated.
  func testScrollPositionSurvivesRecreation() throws {
    try launch(scenario: "scroll-status")

    let rows = app.scrollViews["rows"]
    let offset = app.staticTexts["scroll-offset"]
    XCTAssertTrue(rows.waitToExist(timeout: 2))
    XCTAssertEqual(offset.value as? String, "Offset: 0")

    // The label shows the bound Store, so a change proves native scrolling reached Rust.
    rows.scroll(byDeltaX: 0, deltaY: -300)
    XCTAssertTrue(waitUntil { offset.currentValue != "Offset: 0" })
    let saved = offset.value as? String
    let marker = app.staticTexts["row-30"].frame

    // Hiding removes the native scroll view; showing creates a fresh one at offset zero.
    let toggle = app.buttons["toggle-list"]
    toggle.click()
    XCTAssertTrue(app.staticTexts["hidden"].waitToExist(timeout: 2))
    toggle.click()
    XCTAssertTrue(rows.waitToExist(timeout: 2))

    // The restored view must show the same rows without reporting a reset offset.
    XCTAssertEqual(offset.value as? String, saved)
    XCTAssertEqual(app.staticTexts["row-30"].frame.minY, marker.minY, accuracy: 1)
  }
}
