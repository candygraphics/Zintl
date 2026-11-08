import XCTest

@MainActor
final class SidebarFeaturesTests: ZintlUITestCase {
  /// Verifies clicking an item writes its ID and the app can select another item.
  func testSelectionRoundTrips() throws {
    try launch(scenario: "sidebar-features")

    assertLabel("selection-value", "Selected: inbox")
    app.staticTexts["Drafts"].click()
    assertLabel("selection-value", "Selected: drafts")

    app.buttons["show-sent"].click()
    assertLabel("selection-value", "Selected: sent")
    let sent = app.tables.tableRows.containing(.staticText, identifier: "Sent").firstMatch
    XCTAssertTrue(waitUntil { sent.isSelected })
  }

  /// Verifies an item's badge is shown beside its title.
  func testBadgeIsShown() throws {
    try launch(scenario: "sidebar-features")

    let inbox = app.tables.tableRows.containing(.staticText, identifier: "Inbox").firstMatch
    XCTAssertTrue(inbox.waitToExist(timeout: 2))
    let badge = inbox.staticTexts.matching(NSPredicate(format: "value == '12'")).firstMatch
    XCTAssertTrue(badge.exists)
    XCTAssertGreaterThan(badge.frame.minX, app.staticTexts["Inbox"].frame.maxX)
  }

  /// Verifies typing in the sidebar search field writes the query Store.
  func testSearchFieldWritesTheQuery() throws {
    try launch(scenario: "sidebar-features")

    let search = app.searchFields.firstMatch
    XCTAssertTrue(search.waitToExist(timeout: 2))
    XCTAssertEqual(search.placeholderValue, "Filter")
    search.click()
    search.typeText("dra")
    assertLabel("query-value", "Query: dra")
  }

  /// Verifies an item's context menu shows its items and runs the chosen one.
  func testItemContextMenuRunsTheChosenItem() throws {
    try launch(scenario: "sidebar-features")

    app.staticTexts["Drafts"].rightClick()
    let archive = app.menuItems["Archive Drafts"]
    XCTAssertTrue(archive.waitToExist(timeout: 2))
    XCTAssertTrue(app.menuItems["Rename Drafts"].exists)
    archive.click()
    assertLabel("log", "Log: Archive Drafts")
  }

  /// Verifies items without a context menu show none.
  func testItemsWithoutAMenuShowNone() throws {
    try launch(scenario: "sidebar-features")

    app.staticTexts["Sent"].rightClick()
    XCTAssertFalse(app.menuItems["Archive Drafts"].waitToExist(timeout: 1))
  }
}
