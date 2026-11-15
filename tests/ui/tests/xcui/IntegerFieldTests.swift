import XCTest

@MainActor
final class IntegerFieldTests: ZintlUITestCase {
  /// Returns the integer field's text field, which carries the field's ID.
  private var field: XCUIElement {
    app.textFields["quantity"]
  }

  /// Replaces the field's text with `text` and commits it with Return.
  private func enter(_ text: String) {
    field.click()
    field.typeKey("a", modifierFlags: .command)
    field.typeText("\(text)\r")
  }

  /// Verifies grouped input reaches the Store and is shown with separators.
  func testGroupedInputReachesTheStore() throws {
    try launch(scenario: "integer-field")

    XCTAssertTrue(field.waitToExist(timeout: 2))
    enter("1,234")
    assertLabel("integer-value", "Integer: 1234")
    XCTAssertTrue(waitForValue(field, "1,234"))
  }

  /// Verifies text that is not an integer restores the last integer typed.
  func testInvalidInputKeepsTheValue() throws {
    try launch(scenario: "integer-field")

    XCTAssertTrue(field.waitToExist(timeout: 2))
    enter("1234")
    assertLabel("integer-value", "Integer: 1234")
    enter("abc")
    XCTAssertTrue(waitForValue(field, "1,234"))
    assertLabel("integer-value", "Integer: 1234")
    // Typing reports `12` before `12.5`, so that is the value the field restores.
    enter("12.5")
    XCTAssertTrue(waitForValue(field, "12"))
    assertLabel("integer-value", "Integer: 12")
  }

  /// Verifies a value above 2^53 set by the app is shown exactly.
  func testLargeValuesStayExact() throws {
    try launch(scenario: "integer-field")

    XCTAssertTrue(field.waitToExist(timeout: 2))
    app.buttons["set-big"].click()
    assertLabel("integer-value", "Integer: 9007199254740993")
    XCTAssertTrue(waitForValue(field, "9,007,199,254,740,993"))
  }
  /// Verifies a typed value reaches the Store before Return, so a button click reads it.
  func testTypedValueReachesTheStoreWithoutReturn() throws {
    try launch(scenario: "integer-field")

    XCTAssertTrue(field.waitToExist(timeout: 2))
    field.click()
    field.typeKey("a", modifierFlags: .command)
    field.typeText("1200")
    app.buttons["copy"].click()
    assertLabel("copied-value", "Copied: 1200")
    // The text keeps what was typed until editing ends, then shows separators.
    XCTAssertEqual(field.value as? String, "1200")
    field.typeText("\r")
    XCTAssertTrue(waitForValue(field, "1,200"))
  }
}
