import XCTest

@MainActor
final class ControlsTests: ZintlUITestCase {
  /// Verifies a menu button opens its menu on click and runs the chosen item.
  func testMenuButtonRunsTheChosenItem() throws {
    try launch(scenario: "controls")

    let loaded = app.staticTexts["loaded"]
    app.buttons["samples"].click()
    let item = app.menuItems["Load B"]
    XCTAssertTrue(item.waitToExist(timeout: 2))
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

    XCTAssertTrue(app.activityIndicators["spinner"].waitToExist(timeout: 2))
  }

  /// Verifies an unbound color well shows its declared color.
  func testColorWellShowsItsDeclaredColor() throws {
    try launch(scenario: "controls")

    let well = app.colorWells["declared-color"]
    XCTAssertTrue(well.waitToExist(timeout: 2))
    XCTAssertEqual(well.value as? String, "rgb 1 0 0 1")
  }

  /// Verifies a button with a title color draws its title in that color, unlike a plain button.
  func testButtonTitleColorReachesTheScreen() throws {
    try launch(scenario: "controls")

    let colored = app.buttons["colored-button"]
    let plain = app.buttons["plain-button"]
    XCTAssertTrue(colored.waitToExist(timeout: 2))
    XCTAssertGreaterThan(pixelCount(in: colored, where: Self.isStronglyRed), 10)
    XCTAssertEqual(pixelCount(in: plain, where: Self.isStronglyRed), 0)
  }

  /// Matches strongly red pixels, which only a red title draws in a button.
  private static func isStronglyRed(_ color: NSColor) -> Bool {
    color.redComponent > 0.7 && color.greenComponent < 0.35 && color.blueComponent < 0.35
  }
}
