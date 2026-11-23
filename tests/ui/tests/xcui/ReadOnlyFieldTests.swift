import XCTest

@MainActor
final class ReadOnlyFieldTests: ZintlUITestCase {
  /// Verifies typing into a read-only field changes neither its text nor reports an edit.
  func testTypingDoesNotChangeTheField() throws {
    try launch(scenario: "read-only-field")

    let kind = app.textFields["kind"]
    XCTAssertTrue(kind.waitToExist(timeout: 2))
    XCTAssertEqual(kind.currentValue, "Expense")
    kind.click()
    kind.typeText("x")
    kind.typeKey(.return, modifierFlags: [])
    XCTAssertEqual(kind.currentValue, "Expense")
    let total = app.textFields["total"]
    total.click()
    total.typeText("9")
    total.typeKey(.return, modifierFlags: [])
    XCTAssertEqual(total.currentValue, "1,234")
    assertLabel("changes", "Changes: 0")
  }

  /// Verifies a read-only field's text can be selected and copied.
  func testReadOnlyTextCanBeCopied() throws {
    try launch(scenario: "read-only-field")
    let pasteboard = borrowPasteboard()
    pasteboard.setString("before", forType: .string)

    let kind = app.textFields["kind"]
    XCTAssertTrue(kind.waitToExist(timeout: 2))
    kind.doubleClick()
    kind.typeKey("c", modifierFlags: .command)
    XCTAssertTrue(waitForPasteboard("Expense"))
  }

  /// Verifies read-only fields show new Store values.
  func testReadOnlyFieldsFollowTheirStores() throws {
    try launch(scenario: "read-only-field")

    XCTAssertTrue(app.textFields["kind"].waitToExist(timeout: 2))
    app.buttons["change-values"].click()
    XCTAssertTrue(waitUntil { self.app.textFields["kind"].currentValue == "Income" })
    XCTAssertTrue(waitUntil { self.app.textFields["total"].currentValue == "5,678" })
  }

  /// Verifies read-only fields are as tall as editable ones and not dimmed, and only the
  /// editable integer field has a stepper.
  func testReadOnlyFieldsKeepTheirFrame() throws {
    try launch(scenario: "read-only-field")

    let kind = app.textFields["kind"]
    XCTAssertTrue(kind.waitToExist(timeout: 2))
    XCTAssertEqual(kind.frame.height, app.textFields["note"].frame.height, accuracy: 1)
    let total = app.textFields["total"]
    let count = app.textFields["count"]
    XCTAssertEqual(total.frame.height, count.frame.height, accuracy: 1)
    XCTAssertEqual(app.steppers.count, 1)
    // The read-only field's text reaches the stepper's place, as wide as the whole editable field.
    XCTAssertGreaterThan(total.frame.width, count.frame.width + 10)
    XCTAssertTrue(kind.isEnabled)
  }
}
