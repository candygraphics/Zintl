import XCTest

@MainActor
final class TextFieldTests: ZintlUITestCase {
  /// Verifies native text input updates the Store-backed label.
  func testInputUpdatesStoredValue() throws {
    try launch(scenario: "text-field")

    let input = app.textFields["text-input"]
    XCTAssertTrue(input.waitToExist(timeout: 2))
    input.click()
    input.typeText("typed value")

    let output = app.staticTexts["stored-value"]
    XCTAssertTrue(output.waitToExist(timeout: 2))
    XCTAssertEqual(output.value as? String, #"Stored value: "typed value""#)
  }

  /// Verifies multiline input accepts a line break and updates its bound Store.
  func testMultilineInputUpdatesStoredValue() throws {
    try launch(scenario: "text-field")

    let input = app.textFields["multiline-input"]
    XCTAssertTrue(input.waitToExist(timeout: 2))
    input.click()
    input.typeText("first line")
    input.typeKey(.return, modifierFlags: .option)
    input.typeText("second line")

    XCTAssertEqual(input.value as? String, "first line\nsecond line")
    let output = app.staticTexts["stored-multiline-value"]
    XCTAssertTrue(output.waitToExist(timeout: 2))
    XCTAssertEqual(
      output.value as? String,
      #"Stored multiline value: "first line\nsecond line""#
    )
  }

  /// Verifies a single-line field in a centered row gets a line's height and accepts typing.
  func testFieldInCenteredRowIsTallEnoughToType() throws {
    try launch(scenario: "text-field")

    let input = app.textFields["row-input"]
    XCTAssertTrue(input.waitToExist(timeout: 2))
    XCTAssertGreaterThanOrEqual(input.frame.height, 20)
    XCTAssertTrue(input.isHittable)
    input.click()
    input.typeText("row")
    assertLabel("row-value", #"Row value: "row""#)
  }
}
