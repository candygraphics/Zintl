import XCTest

@MainActor
final class AlertTests: ZintlUITestCase {
  /// Returns the alert sheet on the main window.
  private var sheet: XCUIElement {
    app.windows["main-window"].sheets.firstMatch
  }

  /// Shows the confirmation alert and waits for its sheet.
  private func showConfirmation(file: StaticString = #filePath, line: UInt = #line) {
    app.buttons["delete-route"].click()
    XCTAssertTrue(
      sheet.waitForExistence(timeout: 2), "The alert did not appear", file: file, line: line)
  }

  /// Verifies the alert shows its title, message, and buttons as a sheet on its window.
  func testAlertShowsItsContent() throws {
    try launch(scenario: "alert")

    showConfirmation()
    XCTAssertTrue(sheet.staticTexts["Delete the route?"].exists)
    XCTAssertTrue(sheet.staticTexts["This cannot be undone."].exists)
    XCTAssertTrue(sheet.buttons["Delete"].exists)
    XCTAssertTrue(sheet.buttons["Keep"].exists)
    assertLabel("confirm-value", "true")
  }

  /// Verifies clicking a button runs its action once and dismisses the alert.
  func testClickedButtonRunsOnceAndDismisses() throws {
    try launch(scenario: "alert")

    showConfirmation()
    sheet.buttons["Delete"].click()
    XCTAssertTrue(waitUntil { !self.sheet.exists })
    assertLabel("log", "Log: delete")
    assertLabel("confirm-value", "false")

    // The alert can be shown again, and the other button runs only its own action.
    showConfirmation()
    sheet.buttons["Keep"].click()
    assertLabel("log", "Log: delete,keep")
  }

  /// Verifies Return chooses a plain first button.
  func testReturnChoosesThePlainFirstButton() throws {
    try launch(scenario: "alert")

    app.buttons["show-timed"].click()
    XCTAssertTrue(sheet.waitForExistence(timeout: 2))
    sheet.typeKey(.return, modifierFlags: [])
    assertLabel("log", "Log: wait")
    XCTAssertTrue(waitUntil { !self.sheet.exists })
  }

  /// Verifies Return leaves a destructive first button alone and Escape chooses the cancel button.
  func testReturnSkipsDestructiveButtonAndEscapeCancels() throws {
    try launch(scenario: "alert")

    showConfirmation()
    sheet.typeKey(.return, modifierFlags: [])
    // The alert stays open without running the destructive action.
    XCTAssertFalse(waitUntil(timeout: 1) { !self.sheet.exists })
    assertLabel("log", "Log: ")

    sheet.typeKey(.escape, modifierFlags: [])
    assertLabel("log", "Log: keep")
    assertLabel("confirm-value", "false")
  }

  /// Verifies an alert dismissed by the app closes without running any button's action.
  func testAppDismissalRunsNoAction() throws {
    try launch(scenario: "alert")

    app.buttons["show-timed"].click()
    // The app closes the alert 3 seconds after showing it.
    XCTAssertTrue(sheet.waitForExistence(timeout: 2))
    XCTAssertTrue(sheet.staticTexts["Closing soon"].exists)
    XCTAssertTrue(waitUntil(timeout: 8) { !self.sheet.exists })
    assertLabel("timed-value", "false")
    assertLabel("log", "Log: ")
  }

  /// Verifies an alert without buttons gets an OK button that dismisses it.
  func testAlertWithoutButtonsOffersOK() throws {
    try launch(scenario: "alert")

    app.buttons["show-notice"].click()
    XCTAssertTrue(sheet.waitForExistence(timeout: 2))
    XCTAssertTrue(sheet.staticTexts["Sync finished"].exists)
    sheet.buttons["OK"].click()
    XCTAssertTrue(waitUntil { !self.sheet.exists })
    assertLabel("notice-value", "false")
  }
}
