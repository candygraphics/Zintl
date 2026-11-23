import XCTest

@MainActor
final class ClipboardTests: ZintlUITestCase {
  /// Verifies a button's action puts its text on the system clipboard.
  func testCopyButtonWritesTheClipboard() throws {
    try launch(scenario: "clipboard")
    let pasteboard = borrowPasteboard()
    pasteboard.setString("before", forType: .string)

    app.buttons["copy-result"].click()
    XCTAssertTrue(waitForPasteboard("1,234.50"))
  }

  /// Verifies a button's action reads the system clipboard's text into a Store.
  func testPasteButtonReadsTheClipboard() throws {
    try launch(scenario: "clipboard")
    let pasteboard = borrowPasteboard()
    pasteboard.setString("42 apples", forType: .string)

    app.buttons["paste"].click()
    assertLabel("pasted-value", "Pasted: 42 apples")
  }
}
