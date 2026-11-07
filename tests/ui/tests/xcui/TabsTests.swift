import XCTest

@MainActor
final class TabsTests: ZintlUITestCase {
  /// Returns the tab button titled `title`.
  private func tab(_ title: String) -> XCUIElement {
    app.tabGroups["tabs"].tabs[title]
  }

  /// Verifies clicking a tab shows only its page and writes its index to the Store.
  func testClickingATabShowsItsPage() throws {
    try launch(scenario: "tabs")

    XCTAssertTrue(app.staticTexts["general-page"].waitForExistence(timeout: 2))
    XCTAssertFalse(app.staticTexts["advanced-page"].exists)

    tab("Advanced").click()
    assertLabel("tab-value", "Tab: 1")
    assertLabel("changes", "Changes: 1")
    XCTAssertTrue(app.staticTexts["advanced-page"].waitForExistence(timeout: 2))
    XCTAssertFalse(app.staticTexts["general-page"].exists)
    XCTAssertEqual(tab("Advanced").value as? Int, 1)
  }

  /// Verifies a control inside a page writes its Store and shows it again when its tab returns.
  func testPageControlsKeepTheirStateAcrossTabs() throws {
    try launch(scenario: "tabs")

    tab("Advanced").click()
    let verbose = app.checkBoxes["verbose"]
    XCTAssertTrue(verbose.waitForExistence(timeout: 2))
    // The page stretches the checkbox, but AppKit only toggles it on its box and title.
    verbose.coordinate(withNormalizedOffset: CGVector(dx: 0.02, dy: 0.5)).click()
    assertLabel("verbose-value", "Verbose: true")
    XCTAssertTrue(waitUntil { verbose.value as? Int == 1 })

    tab("General").click()
    tab("Advanced").click()
    XCTAssertTrue(verbose.waitForExistence(timeout: 2))
    XCTAssertEqual(verbose.value as? Int, 1)
  }

  /// Verifies writing the Store from the app switches the native tab without reporting a change.
  func testStoreChangeSelectsTheTab() throws {
    try launch(scenario: "tabs")

    app.buttons["show-about"].click()
    assertLabel("tab-value", "Tab: 2")
    XCTAssertTrue(app.staticTexts["about-page"].waitForExistence(timeout: 2))
    XCTAssertTrue(waitUntil { self.tab("About").value as? Int == 1 })
    assertLabel("changes", "Changes: ")
  }
}
