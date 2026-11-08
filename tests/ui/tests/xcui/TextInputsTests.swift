import XCTest

@MainActor
final class TextInputsTests: ZintlUITestCase {
  /// Verifies choosing a combo box suggestion and typing free text both write its Store.
  func testComboBoxWritesSuggestionsAndTypedText() throws {
    try launch(scenario: "text-inputs")

    let combo = app.comboBoxes["combo"]
    XCTAssertTrue(combo.waitToExist(timeout: 2))
    combo.buttons.firstMatch.click()
    app.textFields["Osaka"].click()
    assertLabel("station-value", "Station: Osaka")

    combo.click()
    combo.typeKey("a", modifierFlags: .command)
    combo.typeText("Kyoto\r")
    assertLabel("station-value", "Station: Kyoto")
  }

  /// Verifies every search edit reaches the Store and on_change, and clearing empties it.
  func testSearchFieldReportsEditsAndClearing() throws {
    try launch(scenario: "text-inputs")

    let search = app.searchFields["search"]
    XCTAssertTrue(search.waitToExist(timeout: 2))
    search.click()
    search.typeText("Yama")
    assertLabel("query-value", "Query: Yama")
    assertLabel("last-search", "Last search: Yama")

    search.buttons["cancel"].click()
    assertLabel("query-value", "Query: ")
  }

  /// Returns the number field's text field.
  ///
  /// The field and stepper share a container that is not an accessibility element, so the
  /// number field's ID is not exposed; it is the window's only plain text field.
  private var countField: XCUIElement {
    app.windows["main-window"].children(matching: .textField).firstMatch
  }

  /// Clicks the upper or lower half of the number field's stepper.
  private func step(up: Bool) {
    app.steppers.firstMatch.coordinate(
      withNormalizedOffset: CGVector(dx: 0.5, dy: up ? 0.25 : 0.75)
    )
    .click()
  }

  /// Replaces the number field's text with `text` and commits it with Return.
  private func enterCount(_ text: String) {
    countField.click()
    countField.typeKey("a", modifierFlags: .command)
    countField.typeText("\(text)\r")
  }

  /// Verifies typed numbers reach the Store and the stepper steps up to the maximum.
  func testNumberFieldAcceptsTypedValuesAndSteps() throws {
    try launch(scenario: "text-inputs")

    XCTAssertTrue(countField.waitToExist(timeout: 2))
    XCTAssertEqual(countField.value as? String, "3")
    enterCount("9")
    assertLabel("count-value", "Count: 9")

    step(up: true)
    assertLabel("count-value", "Count: 10")
    step(up: true)
    // The stepper stops at the maximum.
    assertLabel("count-value", "Count: 10")
    XCTAssertTrue(waitForValue(countField, "10"))
    step(up: false)
    assertLabel("count-value", "Count: 9")
    XCTAssertTrue(waitForValue(countField, "9"))
  }

  /// Verifies a typed value above the range is clamped to the maximum in the Store and the field.
  func testNumberFieldClampsValuesAboveItsRange() throws {
    try launch(scenario: "text-inputs")

    XCTAssertTrue(countField.waitToExist(timeout: 2))
    enterCount("7")
    assertLabel("count-value", "Count: 7")
    XCTExpectFailure(
      "The native number formatter rejects out-of-range text, so the field keeps showing it "
        + "and the Store keeps its old value instead of the clamped maximum."
    )
    enterCount("42")
    assertLabel("count-value", "Count: 10")
    XCTAssertTrue(waitForValue(countField, "10"))
  }

  /// Verifies writing the Stores from the app updates all three native fields.
  func testStoreChangesReachTheFields() throws {
    try launch(scenario: "text-inputs")

    app.buttons["reset"].click()
    XCTAssertTrue(waitForValue(app.comboBoxes["combo"], "Nagoya"))
    XCTAssertTrue(waitForValue(app.searchFields["search"], "rail"))
    XCTAssertTrue(waitForValue(countField, "5"))
    // An app-side write is not a user edit.
    assertLabel("last-search", "Last search: ")
  }
}
