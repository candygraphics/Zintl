import XCTest

@MainActor
final class PickersTests: ZintlUITestCase {
  /// Verifies choosing a pop-up menu item writes its index and reports it to on_change.
  func testMenuPickerWritesTheChosenIndex() throws {
    try launch(scenario: "pickers")

    let picker = app.popUpButtons["menu-picker"]
    XCTAssertTrue(picker.waitToExist(timeout: 2))
    picker.click()
    app.menuItems["Rapid"].click()
    assertLabel("menu-value", "Rapid")
    assertLabel("changes", "Changes: 1")
    XCTAssertEqual(picker.value as? String, "Rapid")
  }

  /// Verifies clicking a segment selects it and writes its index.
  func testSegmentedPickerWritesTheClickedSegment() throws {
    try launch(scenario: "pickers")

    let picker = app.radioGroups["segmented-picker"]
    XCTAssertTrue(picker.waitToExist(timeout: 2))
    picker.radioButtons["Express"].click()
    assertLabel("segmented-value", "Express")
    XCTAssertEqual(picker.radioButtons["Express"].value as? Int, 1)
    XCTAssertEqual(picker.radioButtons["Local"].value as? Int, 0)
  }

  /// Clicks the radio button titled `title` on its circle.
  ///
  /// The stack stretches radio buttons, but AppKit only toggles them on their circle and title.
  private func clickRadio(_ title: String) {
    radio(title).coordinate(withNormalizedOffset: CGVector(dx: 0.05, dy: 0.5)).click()
  }

  /// Returns the radio-style option titled `title`.
  ///
  /// The radio group's container is not an accessibility element, so its ID is not exposed;
  /// segments are labelled instead of titled, which keeps them out of this match.
  private func radio(_ title: String) -> XCUIElement {
    app.radioButtons.matching(NSPredicate(format: "title == %@", title)).firstMatch
  }

  /// Verifies radio buttons keep exactly one option checked and write the checked index.
  func testRadioPickerChecksExactlyOneOption() throws {
    try launch(scenario: "pickers")

    XCTAssertTrue(radio("Rapid").waitToExist(timeout: 2))
    clickRadio("Rapid")
    assertLabel("radio-value", "Rapid")
    let checked = ["Local", "Rapid", "Express"].map { radio($0).value as? Int }
    XCTAssertEqual(checked, [0, 1, 0])

    clickRadio("Express")
    assertLabel("radio-value", "Express")
    XCTAssertEqual(radio("Rapid").value as? Int, 0)
  }

  /// Verifies writing the Stores from the app moves every picker style to that option.
  func testStoreChangesReachEveryPickerStyle() throws {
    try launch(scenario: "pickers")

    app.buttons["select-last"].click()
    let picker = app.popUpButtons["menu-picker"]
    XCTAssertTrue(waitForValue(picker, "Express"))
    let segment = app.radioGroups["segmented-picker"].radioButtons["Express"]
    XCTAssertTrue(waitUntil { segment.value as? Int == 1 })
    let express = radio("Express")
    XCTAssertTrue(waitUntil { express.value as? Int == 1 })
    // An app-side write is not a user change.
    assertLabel("changes", "Changes: ")
  }

  /// Verifies a disabled picker shows its selection and does not open its menu.
  func testDisabledPickerDoesNotOpen() throws {
    try launch(scenario: "pickers")

    let picker = app.popUpButtons["disabled-picker"]
    XCTAssertTrue(picker.waitToExist(timeout: 2))
    XCTAssertFalse(picker.isEnabled)
    XCTAssertEqual(picker.value as? String, "Rapid")
    picker.click()
    XCTAssertFalse(app.menuItems["Express"].waitToExist(timeout: 1))
  }
}
