import XCTest

@MainActor
final class ControlsTests: ZintlUITestCase {
  /// Verifies a menu button opens its menu on click and runs the chosen item.
  func testMenuButtonRunsTheChosenItem() throws {
    try launch(scenario: "controls")

    let loaded = app.staticTexts["loaded"]
    app.buttons["samples"].click()
    let item = app.menuItems["Load B"]
    XCTAssertTrue(item.waitForExistence(timeout: 2))
    item.click()
    XCTAssertTrue(waitForValue(loaded, "Loaded B"))
  }

  /// Returns a row number button; selectable buttons are toggles to accessibility.
  private func row(_ number: Int) -> XCUIElement {
    app.descendants(matching: .any)["row-\(number)"]
  }

  /// Verifies the selected row number stays on and moves when another is clicked.
  func testSelectedButtonFollowsTheSelection() throws {
    try launch(scenario: "controls")

    let selected = app.staticTexts["selected"]
    row(2).click()
    XCTAssertTrue(waitForValue(selected, "Selected: 2"))
    XCTAssertEqual(row(2).value as? Int, 1)
    XCTAssertEqual(row(1).value as? Int, 0)

    // Clicking the selected row keeps it selected.
    row(2).click()
    XCTAssertEqual(row(2).value as? Int, 1)
  }

  /// Verifies the spinner appears as a progress indicator.
  func testSpinnerIsShown() throws {
    try launch(scenario: "controls")

    XCTAssertTrue(app.activityIndicators["spinner"].waitForExistence(timeout: 2))
  }
}
