import XCTest

@MainActor
final class SelectableTextTests: ZintlUITestCase {
  /// Verifies selectable text can be selected and copied, and stays static text with its label.
  func testSelectableTextCanBeCopied() throws {
    try launch(scenario: "selectable-text")
    let pasteboard = borrowPasteboard()
    pasteboard.setString("before", forType: .string)

    let total = app.staticTexts["total"]
    XCTAssertTrue(total.waitToExist(timeout: 2))
    total.doubleClick()
    total.typeKey("c", modifierFlags: .command)
    XCTAssertTrue(waitForPasteboard("1,234"))
    assertLabel("total", "1,234")
  }

  /// Verifies plain text cannot be selected, so Copy leaves the clipboard alone.
  func testPlainTextIsNotSelectable() throws {
    try launch(scenario: "selectable-text")
    let pasteboard = borrowPasteboard()
    pasteboard.setString("before", forType: .string)

    let plain = app.staticTexts["plain"]
    XCTAssertTrue(plain.waitToExist(timeout: 2))
    plain.doubleClick()
    plain.typeKey("c", modifierFlags: .command)
    XCTAssertFalse(waitForPasteboard("5,678", timeout: 0.5))
    XCTAssertEqual(pasteboard.string(forType: .string), "before")
  }

  /// Verifies the first click on a selectable table cell selects the row and a later
  /// double-click selects the cell's text.
  func testTableCellSelectsTheRowFirst() throws {
    try launch(scenario: "selectable-text")
    let pasteboard = borrowPasteboard()
    pasteboard.setString("before", forType: .string)

    let amount = app.staticTexts["amount-2"]
    XCTAssertTrue(amount.waitToExist(timeout: 2))
    amount.click()
    assertLabel("selection", "Selected: 2")
    amount.doubleClick()
    amount.typeKey("c", modifierFlags: .command)
    XCTAssertTrue(waitForPasteboard("20"))
    assertLabel("selection", "Selected: 2")
  }
}
