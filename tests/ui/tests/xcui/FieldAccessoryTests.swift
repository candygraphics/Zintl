import XCTest

@MainActor
final class FieldAccessoryTests: ZintlUITestCase {
  /// Verifies clicking the button inside a field runs its action and leaves typing in the field.
  func testAccessoryClickKeepsFocusAndText() throws {
    try launch(scenario: "field-accessory")

    let field = app.textFields["amount"]
    XCTAssertTrue(field.waitToExist(timeout: 2))
    field.click()
    field.typeText("12")
    app.buttons["amount-calculator"].click()
    assertLabel("clicks", "Clicks: 1")
    // Typing continues at the caret, so the click changed neither focus nor selection.
    app.typeText("3")
    assertLabel("amount-value", "Amount: 123")
  }

  /// Verifies the accessory sits inside the field's trailing edge and a popover still anchors to the field.
  func testAccessoryIsInsideTheFieldAndPopoverAnchorsToIt() throws {
    try launch(scenario: "field-accessory")

    let field = app.textFields["amount"]
    let button = app.buttons["amount-calculator"]
    XCTAssertTrue(field.waitToExist(timeout: 2))
    XCTAssertTrue(button.waitToExist(timeout: 2))
    XCTAssertGreaterThan(button.frame.minX, field.frame.midX)
    XCTAssertLessThanOrEqual(button.frame.maxX, field.frame.maxX + 1)
    button.click()
    let calculator = app.popovers.staticTexts["calculator"]
    XCTAssertTrue(calculator.waitToExist(timeout: 2))
    XCTAssertEqual(app.popovers.firstMatch.frame.midX, field.frame.midX, accuracy: 2)
  }

  /// Verifies an integer field's accessory is clickable by ID and does not change the value.
  func testIntegerFieldAccessoryRunsItsAction() throws {
    try launch(scenario: "field-accessory")

    let button = app.buttons["count-calculator"]
    XCTAssertTrue(button.waitToExist(timeout: 2))
    XCTAssertLessThanOrEqual(button.frame.maxX, app.textFields["count"].frame.maxX + 1)
    button.click()
    assertLabel("clicks", "Clicks: 1")
    assertLabel("count-value", "Count: 5")
  }

  /// Verifies a field with an accessory is as wide as one without in the same grid column.
  func testAccessoryKeepsTheFieldWidthInAGrid() throws {
    try launch(scenario: "field-accessory")

    let plain = app.textFields["plain-field"]
    let accessory = app.textFields["accessory-field"]
    XCTAssertTrue(accessory.waitToExist(timeout: 2))
    XCTAssertEqual(accessory.frame.width, plain.frame.width, accuracy: 1)
    XCTAssertEqual(accessory.frame.minX, plain.frame.minX, accuracy: 1)
  }
}
