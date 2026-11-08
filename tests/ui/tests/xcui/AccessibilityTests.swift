import XCTest

/// Checks what VoiceOver reads, through the same accessibility tree XCUITest sees.
@MainActor
final class AccessibilityTests: ZintlUITestCase {
  /// Verifies a symbol-only button reads its accessibility label.
  func testSymbolButtonReadsItsLabel() throws {
    try launch(scenario: "accessibility")

    XCTAssertEqual(app.buttons["add"].label, "Add tool")
  }

  /// Verifies a labeled container is one named group whose value follows a Store.
  func testLabeledContainerIsAGroupWithAValue() throws {
    try launch(scenario: "accessibility")

    let toolbox = app.groups["toolbox"]
    XCTAssertTrue(toolbox.waitToExist(timeout: 2))
    XCTAssertEqual(toolbox.label, "Toolbox")
    XCTAssertEqual(toolbox.currentValue, "2 tools")

    app.buttons["add"].click()
    XCTAssertTrue(waitForValue(toolbox, "3 tools"))
  }

  /// Verifies hidden decoration is absent from the accessibility tree.
  func testHiddenViewIsNotAnElement() throws {
    try launch(scenario: "accessibility")

    XCTAssertTrue(app.buttons["add"].waitToExist(timeout: 2))
    XCTAssertFalse(element("decoration").exists)
  }

  /// Verifies a disclosure group's triangle and section carry its title.
  func testDisclosureGroupIsNamed() throws {
    try launch(scenario: "accessibility")

    XCTAssertEqual(app.disclosureTriangles["line-settings"].label, "Line settings")
    let section = app.groups.matching(NSPredicate(format: "label == %@", "Line settings"))
    XCTAssertTrue(section.firstMatch.exists)
  }
}
