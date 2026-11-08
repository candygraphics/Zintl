import XCTest

@MainActor
final class ColorPanelTests: ZintlUITestCase {
  /// Returns the shared color panel window.
  private var panel: XCUIElement {
    app.windows["Colors"]
  }

  /// Verifies writing the bound Store from the app recolors the well without reporting a change.
  func testStoreChangeRecolorsTheWell() throws {
    try launch(scenario: "color-panel")

    let well = app.colorWells["well"]
    XCTAssertTrue(well.waitToExist(timeout: 2))
    XCTAssertEqual(well.value as? String, "rgb 1 0 0 1")

    app.buttons["set-blue"].click()
    XCTAssertTrue(waitForValue(well, "rgb 0 0 1 1"))
    assertLabel("well-value", "Well: 0.00 0.00 1.00")
    assertLabel("changes", "Changes: 0")
  }

  /// Verifies showing the panel opens the system color panel and closing it clears the Store.
  func testClosingThePanelClearsItsStore() throws {
    try launch(scenario: "color-panel")

    app.buttons["show-colors"].click()
    XCTAssertTrue(panel.waitToExist(timeout: 5))
    assertLabel("panel-shown", "Panel shown: true")

    panel.buttons[XCUIIdentifierCloseWindow].click()
    XCTAssertTrue(waitUntil { !self.panel.exists })
    assertLabel("panel-shown", "Panel shown: false")
  }

  /// Verifies a color picked in the panel reaches the panel's bound Store.
  func testPickedColorReachesTheStore() throws {
    try launch(scenario: "color-panel")

    app.buttons["show-colors"].click()
    XCTAssertTrue(panel.waitToExist(timeout: 5))
    try pick(hex: "00FF00")
    assertLabel("panel-value", "Panel: 0.00 1.00 0.00", timeout: 5)
  }

  /// Verifies clicking the well connects it to the panel, so a picked color reaches the well's Store.
  func testWellReceivesColorsPickedInThePanel() throws {
    try launch(scenario: "color-panel")

    app.colorWells["well"].click()
    XCTAssertTrue(panel.waitToExist(timeout: 5))
    try pick(hex: "0000FF")
    assertLabel("well-value", "Well: 0.00 0.00 1.00", timeout: 5)
    XCTAssertTrue(waitForValue(app.colorWells["well"], "rgb 0 0 1 1"))
    XCTAssertNotEqual(app.staticTexts["changes"].value as? String, "Changes: 0")
  }

  /// Types `hex` into the hex field of the panel's RGB sliders.
  private func pick(hex: String) throws {
    panel.toolbars.buttons["Color Sliders"].click()
    let mode = panel.popUpButtons.firstMatch
    if mode.value as? String != "RGB Sliders" {
      mode.click()
      panel.menuItems["RGB Sliders"].click()
    }
    let field = panel.textFields.matching(NSPredicate(format: "value MATCHES '[0-9A-Fa-f]{6}'"))
      .firstMatch
    XCTAssertTrue(field.waitToExist(timeout: 2), "The panel shows no hex field")
    field.doubleClick()
    field.typeKey("a", modifierFlags: .command)
    field.typeText("\(hex)\r")
  }
}
