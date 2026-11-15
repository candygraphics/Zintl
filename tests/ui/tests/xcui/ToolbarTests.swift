import XCTest

@MainActor
final class ToolbarTests: ZintlUITestCase {
  /// Verifies a toolbar button is found by its item ID, independent of its label.
  func testButtonIsFoundByItemID() throws {
    try launch(scenario: "toolbar")

    let add = app.toolbars.buttons["add"]
    XCTAssertTrue(add.waitToExist(timeout: 2))
    add.click()
    assertLabel("count", "Count: 1")
  }

  /// Verifies a toolbar button click runs its action.
  func testButtonClickRunsItsAction() throws {
    try launch(scenario: "toolbar")

    let add = app.toolbars.buttons["Add"]
    XCTAssertTrue(add.waitToExist(timeout: 2))
    add.click()
    add.click()
    assertLabel("count", "Count: 2")
  }

  /// Verifies a button's enabled state follows the Store it is computed from.
  ///
  /// Toolbar buttons report themselves enabled to accessibility either way, so
  /// this checks whether clicks run the action.
  func testEnabledStateFollowsTheStore() throws {
    try launch(scenario: "toolbar")

    let remove = app.toolbars.buttons["Remove"]
    XCTAssertTrue(remove.waitToExist(timeout: 2))
    remove.click()
    app.toolbars.buttons["Add"].click()
    assertLabel("count", "Count: 1")
    assertLabel("removes", "Removes: 0")

    remove.click()
    assertLabel("removes", "Removes: 1")
    assertLabel("count", "Count: 0")
    remove.click()
    app.toolbars.buttons["Add"].click()
    assertLabel("count", "Count: 1")
    assertLabel("removes", "Removes: 1")
  }

  /// Verifies typing in the toolbar search field writes the query Store.
  func testSearchFieldWritesTheQuery() throws {
    try launch(scenario: "toolbar")

    let search = app.toolbars.searchFields.firstMatch
    XCTAssertTrue(search.waitToExist(timeout: 2))
    search.click()
    search.typeText("Kyoto")
    assertLabel("query-value", "Query: Kyoto")
  }

  /// Verifies writing the query Store from the app updates the search field.
  func testQueryStoreUpdatesTheSearchField() throws {
    try launch(scenario: "toolbar")

    app.buttons["set-query"].click()
    assertLabel("query-value", "Query: Osaka")
    XCTAssertTrue(waitForValue(app.toolbars.searchFields.firstMatch, "Osaka"))
  }

  /// Verifies the search field is found by its ID inside the toolbar.
  func testSearchFieldIsFoundByID() throws {
    try launch(scenario: "toolbar")

    XCTAssertTrue(app.toolbars.searchFields["toolbar-search"].waitToExist(timeout: 2))
  }

  /// Verifies a focus request from the app moves typing into the search field.
  func testFocusRequestMovesTypingToTheSearchField() throws {
    try launch(scenario: "toolbar")

    let search = app.toolbars.searchFields["toolbar-search"]
    XCTAssertTrue(search.waitToExist(timeout: 2))
    app.buttons["find"].click()
    XCTAssertTrue(waitUntil { (search.value(forKey: "hasKeyboardFocus") as? Bool) == true })
    app.typeText("abc")
    assertLabel("query-value", "Query: abc")
    XCTAssertTrue(waitForValue(search, "abc"))
  }
}
