import XCTest

@MainActor
final class FindMenuTests: ZintlUITestCase {
  /// Returns the find bar's search field in the main window.
  private var findField: XCUIElement {
    app.windows["main-window"].searchFields.firstMatch
  }

  /// Verifies ⌘F in the text editor shows its find bar.
  func testCommandFShowsTheFindBar() throws {
    try launch(scenario: "find-menu")

    app.textViews.firstMatch.click()
    XCTAssertFalse(findField.exists)
    app.typeKey("f", modifierFlags: .command)
    XCTAssertTrue(findField.waitToExist(timeout: 2))
  }

  /// Verifies Edit > Find > Find… shows the find bar of the focused editor.
  func testFindMenuItemShowsTheFindBar() throws {
    try launch(scenario: "find-menu")

    app.textViews.firstMatch.click()
    app.menuBars.menuBarItems["Edit"].click()
    app.menuItems["Find"].click()
    app.menuItems["Find…"].click()
    XCTAssertTrue(findField.waitToExist(timeout: 2))
  }
}
