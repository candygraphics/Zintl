import XCTest

@MainActor
final class DocumentWindowTests: ZintlUITestCase {
  /// Verifies the close button asks the view first and closes only once the view allows it.
  func testCloseButtonAsksBeforeClosing() throws {
    try launch(scenario: "document")

    let window = app.windows["main-window"]
    let requests = app.staticTexts["close-requests"]
    window.buttons[XCUIIdentifierCloseWindow].click()
    XCTAssertTrue(waitForValue(requests, "Close requests: 1"))
    XCTAssertTrue(window.exists)

    app.buttons["allow-close"].click()
    window.buttons[XCUIIdentifierCloseWindow].click()
    XCTAssertTrue(app.wait(for: .notRunning, timeout: 5))
  }

  /// Verifies ⌘Q asks a window that confirms closing instead of quitting at once.
  func testQuitAsksTheWindowFirst() throws {
    try launch(scenario: "document")

    let requests = app.staticTexts["close-requests"]
    app.typeKey("q", modifierFlags: .command)
    XCTAssertTrue(waitForValue(requests, "Close requests: 1"))
    XCTAssertEqual(app.state, .runningForeground)

    app.buttons["allow-close"].click()
    app.typeKey("q", modifierFlags: .command)
    XCTAssertTrue(app.wait(for: .notRunning, timeout: 5))
  }

  /// Verifies ⌘Z steps the app's history outside text fields and text undo inside them.
  func testUndoGoesToTextFieldsWhileEditingAndToTheAppOtherwise() throws {
    try launch(scenario: "document")

    let edits = app.staticTexts["edits"]
    app.buttons["edit-button"].click()
    app.buttons["edit-button"].click()
    XCTAssertTrue(waitForValue(edits, "Edits: 2"))

    app.typeKey("z", modifierFlags: .command)
    XCTAssertTrue(waitForValue(edits, "Edits: 1"))
    app.typeKey("z", modifierFlags: [.command, .shift])
    XCTAssertTrue(waitForValue(edits, "Edits: 2"))

    let note = app.textFields["note-field"]
    note.click()
    note.typeText("draft")
    XCTAssertEqual(note.value as? String, "draft")
    app.typeKey("z", modifierFlags: .command)
    XCTAssertTrue(waitForValue(note, ""))
    XCTAssertEqual(edits.value as? String, "Edits: 2")
  }
}
