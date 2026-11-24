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

  /// Presents the alert sheet from the Delete button and dismisses it with Cancel.
  private func presentAndCancelAlert(file: StaticString = #filePath, line: UInt = #line) {
    let sheet = app.windows["main-window"].sheets.firstMatch
    app.buttons["delete"].click()
    XCTAssertTrue(sheet.waitToExist(timeout: 2), "The alert did not appear", file: file, line: line)
    sheet.buttons["Cancel"].click()
    XCTAssertTrue(waitUntil { !sheet.exists }, "The alert did not close", file: file, line: line)
  }

  /// Verifies a text field and its accessory stay queryable after a sheet closes while the field is edited.
  func testTextFieldAccessoryIsFoundAfterASheetCloses() throws {
    try launch(scenario: "field-accessory")

    let field = app.textFields["amount"]
    XCTAssertTrue(field.waitToExist(timeout: 2))
    field.click()
    field.typeText("12")
    presentAndCancelAlert()
    XCTAssertTrue(field.waitToExist(timeout: 2))
    XCTAssertTrue(app.buttons["amount-calculator"].waitToExist(timeout: 2))
    app.buttons["amount-calculator"].click()
    assertLabel("clicks", "Clicks: 1")
  }

  /// Verifies an integer field and its accessory stay queryable after a sheet closes while the field is edited.
  func testIntegerFieldAccessoryIsFoundAfterASheetCloses() throws {
    try launch(scenario: "field-accessory")

    let field = app.textFields["count"]
    XCTAssertTrue(field.waitToExist(timeout: 2))
    field.click()
    field.typeText("7")
    presentAndCancelAlert()
    XCTAssertTrue(field.waitToExist(timeout: 2))
    XCTAssertTrue(app.buttons["count-calculator"].waitToExist(timeout: 2))
    app.buttons["count-calculator"].click()
    assertLabel("clicks", "Clicks: 1")
  }

  /// Verifies the whole accessibility tree is read quickly while a field with an accessory is edited.
  func testTreeIsReadableWhileEditing() throws {
    try launch(scenario: "field-accessory")

    for id in ["amount", "count"] {
      let field = app.textFields[id]
      XCTAssertTrue(field.waitToExist(timeout: 2))
      field.click()
      field.typeText("1")
      let start = Date()
      XCTAssertFalse(app.debugDescription.isEmpty)
      XCTAssertLessThan(Date().timeIntervalSince(start), 1, "Reading the tree while editing \(id)")
    }
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
