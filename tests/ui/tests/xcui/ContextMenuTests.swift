import XCTest

@MainActor
final class ContextMenuTests: ZintlUITestCase {
  /// Opens the station label's context menu and waits for it.
  private func openStationMenu(file: StaticString = #filePath, line: UInt = #line) {
    app.staticTexts["station"].rightClick()
    XCTAssertTrue(
      app.menuItems["Rename"].waitForExistence(timeout: 2), "The context menu did not open",
      file: file, line: line)
  }

  /// Verifies right-clicking shows the items in their declared order.
  func testItemsAppearInOrder() throws {
    try launch(scenario: "context-menu")

    openStationMenu()
    let tops = ["Rename", "Duplicate", "Unavailable", "Remove"].map {
      app.menuItems[$0].frame.minY
    }
    XCTAssertEqual(tops, tops.sorted())
    XCTAssertEqual(Set(tops).count, tops.count)
  }

  /// Verifies each item runs its own action, including items after a separator.
  func testItemsRouteToTheirActions() throws {
    try launch(scenario: "context-menu")

    openStationMenu()
    app.menuItems["Remove"].click()
    assertLabel("log", "Log: Remove")
    openStationMenu()
    app.menuItems["Rename"].click()
    assertLabel("log", "Log: Remove,Rename")
  }

  /// Verifies an item without an action is disabled while the others are enabled.
  func testItemWithoutActionIsDisabled() throws {
    try launch(scenario: "context-menu")

    openStationMenu()
    XCTAssertFalse(app.menuItems["Unavailable"].isEnabled)
    XCTAssertTrue(app.menuItems["Duplicate"].isEnabled)
  }

  /// Verifies a container's context menu opens from its own area with only its items.
  func testContainerShowsItsOwnMenu() throws {
    try launch(scenario: "context-menu")

    let label = app.staticTexts["line-panel"]
    XCTAssertTrue(label.waitForExistence(timeout: 2))
    // Clicks the container's padding above its label.
    label.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0)).withOffset(
      CGVector(dx: 0, dy: -8)
    ).rightClick()
    let edit = app.menuItems["Edit Line"]
    XCTAssertTrue(edit.waitForExistence(timeout: 2))
    XCTAssertFalse(app.menuItems["Rename"].exists)
    edit.click()
    assertLabel("log", "Log: Edit Line")
  }
}
