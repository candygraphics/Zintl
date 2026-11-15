import XCTest

@MainActor
final class ButtonsTests: ZintlUITestCase {
  /// Verifies Return runs the default button and Escape runs the cancel button.
  func testReturnAndEscapeRunTheirRoles() throws {
    try launch(scenario: "buttons")

    let window = app.windows["main-window"]
    window.typeKey(.return, modifierFlags: [])
    assertLabel("log", "Log: save")
    window.typeKey(.escape, modifierFlags: [])
    assertLabel("log", "Log: save,cancel")
  }

  /// Verifies Return in a single-line text field still reaches the default button.
  func testReturnInATextFieldRunsTheDefaultButton() throws {
    try launch(scenario: "buttons")

    let note = app.textFields["note"]
    note.click()
    note.typeText("Draft\r")
    assertLabel("log", "Log: save")
  }

  /// Verifies the default button draws in the accent color, unlike a plain button.
  func testDefaultButtonIsHighlighted() throws {
    try launch(scenario: "buttons")

    let save = app.buttons["save"]
    let plain = app.buttons["plain"]
    XCTAssertTrue(save.waitToExist(timeout: 2))
    XCTAssertGreaterThan(pixelCount(in: save, where: Self.isSaturated), 50)
    XCTAssertEqual(pixelCount(in: plain, where: Self.isSaturated), 0)
  }

  /// Verifies destructive, symbol-only, and styled buttons run their actions when clicked.
  func testStyledButtonsRunTheirActions() throws {
    try launch(scenario: "buttons")

    for id in ["delete", "gear", "circular", "help"] {
      app.buttons[id].click()
    }
    assertLabel("log", "Log: delete,gear,circular,help")
  }

  /// Verifies icon-only buttons take their symbol's size and still run their actions.
  func testIconOnlyButtonsFitTheirSymbol() throws {
    try launch(scenario: "buttons")

    for id in ["previous", "next", "gear"] {
      let button = app.buttons[id]
      XCTAssertTrue(button.waitToExist(timeout: 2))
      XCTAssertLessThan(button.frame.width, 40, id)
    }
    app.buttons["previous"].click()
    app.buttons["gear"].click()
    assertLabel("log", "Log: previous,gear")
  }

  /// Verifies a disabled button reports itself disabled and ignores clicks.
  func testDisabledButtonIgnoresClicks() throws {
    try launch(scenario: "buttons")

    let disabled = app.buttons["disabled"]
    XCTAssertFalse(disabled.isEnabled)
    disabled.click()
    app.buttons["plain"].click()
    assertLabel("log", "Log: plain")
  }

  /// Matches clearly colored pixels; plain buttons draw only grays.
  private static func isSaturated(_ color: NSColor) -> Bool {
    let channels = [color.redComponent, color.greenComponent, color.blueComponent]
    return (channels.max() ?? 0) - (channels.min() ?? 0) > 0.3
  }
}
