import XCTest

@MainActor
final class FileDialogTests: ZintlUITestCase {
  /// The directory both dialogs start in.
  private var directory: URL!

  /// Launches the scenario with dialogs that start in a fresh temporary directory.
  private func launchWithDirectory() throws {
    directory = try makeTemporaryDirectory()
    try "1k000".write(
      to: directory.appendingPathComponent("existing.txt"), atomically: true, encoding: .utf8)
    try launch(
      scenario: "file-dialog", environment: ["ZINTL_UI_TEST_DIRECTORY": directory.path])
  }

  /// Returns the dialog sheet on the main window.
  private var sheet: XCUIElement {
    app.windows["main-window"].sheets.firstMatch
  }

  /// Verifies saving reports the chosen path in the starting directory and clears the Store.
  func testSaveReportsTheChosenPath() throws {
    try launchWithDirectory()

    app.buttons["save"].click()
    XCTAssertTrue(sheet.waitToExist(timeout: 5))
    assertLabel("saving-value", "Saving: true")
    // The panel hides the extension when Finder hides filename extensions.
    let name = sheet.textFields.firstMatch.value as? String
    XCTAssertTrue(
      ["route.txt", "route"].contains(name), "The name field shows \(String(describing: name))")

    sheet.buttons["Save"].click()
    assertLabel("result", "Saved: \(directory.lastPathComponent)/route.txt", timeout: 5)
    assertLabel("saving-value", "Saving: false")
    XCTAssertFalse(sheet.exists)
  }

  /// Verifies cancelling a dialog reports the cancellation and clears the Store.
  func testCancelReportsCancellation() throws {
    try launchWithDirectory()

    app.buttons["save"].click()
    XCTAssertTrue(sheet.waitToExist(timeout: 5))
    sheet.buttons["Cancel"].click()
    assertLabel("result", "Cancelled", timeout: 5)
    assertLabel("saving-value", "Saving: false")
    XCTAssertTrue(waitUntil { !self.sheet.exists })
  }

  /// Verifies the open dialog reports the chosen file and clears the Store.
  func testOpenReportsTheChosenFile() throws {
    try launchWithDirectory()

    app.buttons["open"].click()
    XCTAssertTrue(sheet.waitToExist(timeout: 5))
    // The file browser may scroll its columns out of view, so the path is typed instead.
    sheet.typeKey("g", modifierFlags: [.command, .shift])
    let goTo = sheet.sheets["GoToWindow"]
    let field = goTo.textFields["PathTextField"]
    XCTAssertTrue(field.waitToExist(timeout: 5))
    paste(directory.appendingPathComponent("existing.txt").path, into: field)
    field.typeKey(.return, modifierFlags: [])
    XCTAssertTrue(waitUntil(timeout: 5) { !goTo.exists })
    sheet.buttons["Open"].click()
    assertLabel("result", "Opened: \(directory.lastPathComponent)/existing.txt", timeout: 5)
    assertLabel("opening-value", "Opening: false")
  }
}
