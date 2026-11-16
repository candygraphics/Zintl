import XCTest

@MainActor
final class FixedValuesTests: ZintlUITestCase {
  /// Verifies a slider returns to the app's value when on_change does not accept the change.
  func testRejectedChangeRevertsTheSlider() throws {
    try launch(scenario: "fixed-values")

    let slider = app.sliders["slider"]
    XCTAssertTrue(slider.waitToExist(timeout: 2))
    XCTAssertEqual(slider.normalizedSliderPosition, 0.25, accuracy: 0.02)

    // Clicking the track asks for a value near the maximum.
    slider.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5)).click()
    XCTAssertTrue(
      waitUntil { self.app.staticTexts["slider-rejected"].value as? String != "Rejected: 0" })
    XCTAssertTrue(
      waitUntil { abs(slider.normalizedSliderPosition - 0.25) < 0.02 }, "the knob stayed moved")
  }

  /// Verifies a tab view returns to the app's page when on_change does not accept the click.
  func testRejectedClickRevertsTheTabView() throws {
    try launch(scenario: "fixed-values")

    let tabs = app.tabGroups["tabs"].tabs
    XCTAssertTrue(app.staticTexts["first-page"].waitToExist(timeout: 2))

    tabs["Second"].click()
    assertLabel("tabs-rejected", "Rejected: 1")
    XCTAssertTrue(waitUntil { tabs["First"].value as? Int == 1 }, "the second tab stayed selected")
    XCTAssertTrue(app.staticTexts["first-page"].waitToExist(timeout: 2))
    XCTAssertFalse(app.staticTexts["second-page"].exists)
  }

  /// Verifies a color well returns to the app's color when on_change does not accept the pick.
  func testRejectedPickRevertsTheColorWell() throws {
    try launch(scenario: "fixed-values")

    let well = app.colorWells["well"]
    XCTAssertTrue(well.waitToExist(timeout: 2))
    XCTAssertEqual(well.value as? String, "rgb 1 0 0 1")

    well.click()
    let panel = app.windows["Colors"]
    XCTAssertTrue(panel.waitToExist(timeout: 5))
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
    field.typeText("0000FF\r")

    XCTAssertTrue(
      waitUntil(timeout: 5) {
        self.app.staticTexts["well-rejected"].value as? String != "Rejected: 0"
      })
    XCTAssertTrue(waitForValue(well, "rgb 1 0 0 1"), "the well kept the picked color")
  }
}
