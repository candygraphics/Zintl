import XCTest

@MainActor
final class TogglesTests: ZintlUITestCase {
  /// Returns the on state an accessibility client reads from a toggle.
  private func isOn(_ id: String) -> Bool {
    (element(id).value as? Int) == 1
  }

  /// Verifies clicking a switch flips its Store and reports each new state to on_change.
  func testSwitchClickWritesItsStore() throws {
    try launch(scenario: "toggles")

    element("switch").click()
    assertLabel("switch-value", "On")
    XCTAssertTrue(isOn("switch"))

    element("switch").click()
    assertLabel("switch-value", "Off")
    assertLabel("changes", "Changes: on,off")
  }

  /// Verifies the checkbox and button styles each flip their own Store when clicked.
  func testCheckboxAndButtonStylesWriteTheirStores() throws {
    try launch(scenario: "toggles")

    element("checkbox").click()
    assertLabel("checkbox-value", "On")
    XCTAssertTrue(isOn("checkbox"))

    element("button-toggle").click()
    assertLabel("button-toggle-value", "On")
    XCTAssertTrue(isOn("button-toggle"))
    // The other toggles keep their own state.
    assertLabel("switch-value", "Off")
  }

  /// Verifies writing the Stores from the app turns every native toggle on and off.
  func testStoreChangesReachTheNativeToggles() throws {
    try launch(scenario: "toggles")

    app.buttons["all-on"].click()
    for id in ["switch", "checkbox", "button-toggle"] {
      XCTAssertTrue(waitUntil { self.isOn(id) }, "\(id) did not turn on")
    }
    // An app-side write is not a user change.
    assertLabel("changes", "Changes: ")

    app.buttons["all-off"].click()
    for id in ["switch", "checkbox", "button-toggle"] {
      XCTAssertTrue(waitUntil { !self.isOn(id) }, "\(id) did not turn off")
    }
  }

  /// Verifies a disabled toggle shows its Store but ignores clicks.
  func testDisabledToggleIgnoresClicks() throws {
    try launch(scenario: "toggles")

    let locked = element("disabled-checkbox")
    XCTAssertTrue(locked.waitForExistence(timeout: 2))
    XCTAssertFalse(locked.isEnabled)
    XCTAssertTrue(isOn("disabled-checkbox"))

    locked.click()
    assertLabel("disabled-value", "On")
    XCTAssertTrue(isOn("disabled-checkbox"))
  }
}
