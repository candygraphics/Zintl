import XCTest

@MainActor
final class TextEditorTests: ZintlUITestCase {
  /// Returns the editor's text view.
  private var editor: XCUIElement {
    app.textViews.firstMatch
  }

  /// Verifies typing writes the bound Store and runs on_change for each edit.
  func testTypingWritesTheStoreAndRunsOnChange() throws {
    try launch(scenario: "text-editor")

    editor.click()
    editor.typeText("Kyoto")
    assertLabel("text-value", "Text: Kyoto")
    assertLabel("changes", "Changes: 5")
  }

  /// Verifies styled text written by the app shows in the editor and keeps its bold run.
  func testStyledTextFromTheAppIsShown() throws {
    try launch(scenario: "text-editor")

    app.buttons["load"].click()
    XCTAssertTrue(waitForValue(editor, "Timetable\nDeparts 9:00"))
    assertLabel("bold-runs", "Bold runs: 1")

    // Typing after the load keeps the styles the app wrote.
    editor.click()
    editor.typeKey(.downArrow, modifierFlags: .command)
    editor.typeText("!")
    assertLabel("text-value", "Text: Timetable\nDeparts 9:00!")
    assertLabel("bold-runs", "Bold runs: 1")
  }

  /// Verifies ⌘Z undoes typing in the editor and writes the Store back.
  func testUndoRevertsTyping() throws {
    try launch(scenario: "text-editor")

    editor.click()
    editor.typeText("Nara")
    assertLabel("text-value", "Text: Nara")
    app.typeKey("z", modifierFlags: .command)
    XCTAssertTrue(waitForValue(editor, ""))
    assertLabel("text-value", "Text: ")
  }
}
