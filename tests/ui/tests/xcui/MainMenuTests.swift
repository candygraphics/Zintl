import XCTest

@MainActor
final class MainMenuTests: ZintlUITestCase {
  /// Opens the menu bar item titled `title`.
  private func openMenu(_ title: String) {
    let item = app.menuBars.menuBarItems[title]
    XCTAssertTrue(item.waitToExist(timeout: 2), "Menu \(title) is missing")
    item.click()
  }

  /// Verifies the standard menus keep their order and the custom menu sits before Window.
  func testMenuBarOrder() throws {
    try launch(scenario: "main-menu")

    let expected = ["File", "Edit", "View", "Route", "Window", "Help"]
    XCTAssertTrue(app.menuBars.menuBarItems["Help"].waitToExist(timeout: 2))
    let titles = app.menuBars.menuBarItems.allElementsBoundByIndex.map(\.title)
    XCTAssertEqual(titles.filter(expected.contains), expected)
  }

  /// Verifies choosing a custom item from its menu runs its action.
  func testCustomItemRunsFromTheMenu() throws {
    try launch(scenario: "main-menu")

    openMenu("Route")
    app.menuItems["Say Hello"].click()
    assertLabel("log", "Log: Hello")
  }

  /// Verifies the shortcuts of custom, extended, and Settings items run their actions.
  func testShortcutsRunTheirItems() throws {
    try launch(scenario: "main-menu")

    app.typeKey("d", modifierFlags: .command)
    assertLabel("log", "Log: Duplicate")
    app.typeKey("h", modifierFlags: [.command, .shift])
    assertLabel("log", "Log: Duplicate,Hello")
    app.typeKey(",", modifierFlags: .command)
    assertLabel("log", "Log: Duplicate,Hello,Settings")
  }

  /// Verifies items without actions stay disabled and enabled(...) follows its Store.
  func testDisabledItemsFollowTheirState() throws {
    try launch(scenario: "main-menu")

    openMenu("Route")
    XCTAssertFalse(app.menuItems["No Action"].isEnabled)
    XCTAssertFalse(app.menuItems["Reverse Route"].isEnabled)
    app.typeKey(.escape, modifierFlags: [])

    app.buttons["enable-reverse"].click()
    openMenu("Route")
    let reverse = app.menuItems["Reverse Route"]
    XCTAssertTrue(waitUntil { reverse.isEnabled })
    reverse.click()
    assertLabel("log", "Log: Reverse")
  }

  /// Verifies the Japanese localizer translates standard menus but keeps custom titles.
  func testJapaneseLocalizerTranslatesStandardMenus() throws {
    try launch(scenario: "main-menu", environment: ["ZINTL_UI_TEST_MENU_LANGUAGE": "ja"])

    XCTAssertTrue(app.menuBars.menuBarItems["ファイル"].waitToExist(timeout: 2))
    XCTAssertTrue(app.menuBars.menuBarItems["編集"].exists)
    XCTAssertTrue(app.menuBars.menuBarItems["Route"].exists)
    XCTAssertFalse(app.menuBars.menuBarItems["File"].exists)
  }
}
