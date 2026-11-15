import XCTest

@MainActor
final class DatePickerTests: ZintlUITestCase {
  /// Returns the date the picker shows as `YYYY-MM-DD`.
  ///
  /// XCUITest reports the picker's NSDate without bridging it; its description is in UTC,
  /// which is the picker's own time zone.
  private func shownDate(_ picker: XCUIElement) -> String? {
    guard let value = picker.value else { return nil }
    let description = String(describing: value)
    guard let range = description.range(of: #"\d{4}-\d{2}-\d{2}"#, options: .regularExpression)
    else {
      return nil
    }
    return String(description[range])
  }

  /// Steps the picker's first field up with the keyboard and returns the label's new date.
  private func stepFirstField(_ picker: XCUIElement) {
    picker.coordinate(withNormalizedOffset: CGVector(dx: 0.1, dy: 0.5)).click()
    picker.typeKey(.upArrow, modifierFlags: [])
  }

  /// Verifies a date entered in the picker reaches the Store, whichever field the locale puts first.
  func testSteppingChangesTheDate() throws {
    try launch(scenario: "date-picker")

    let picker = app.datePickers["date"]
    XCTAssertTrue(picker.waitToExist(timeout: 2))
    XCTAssertEqual(shownDate(picker), "2026-11-15")
    stepFirstField(picker)
    let label = app.staticTexts["date-value"]
    let stepped = ["2027-11-15", "2026-12-15", "2026-11-16"]
    XCTAssertTrue(
      waitUntil { stepped.contains { label.currentValue == "Date: \($0)" } },
      label.currentValue ?? "")
    XCTAssertEqual("Date: \(shownDate(picker) ?? "")", label.currentValue)
  }

  /// Verifies a date set by the app reaches both pickers.
  func testStoreChangeReachesThePickers() throws {
    try launch(scenario: "date-picker")

    XCTAssertTrue(app.datePickers["date"].waitToExist(timeout: 2))
    app.buttons["set-leap"].click()
    assertLabel("date-value", "Date: 2024-02-29")
    XCTAssertTrue(waitUntil { self.shownDate(self.app.datePickers["date"]) == "2024-02-29" })
    XCTAssertEqual(shownDate(app.datePickers["calendar"]), "2024-02-29")
  }

  /// Verifies the date stays the same day in a time zone far from UTC.
  func testDateStaysTheSameDayFarFromUTC() throws {
    try launch(scenario: "date-picker", environment: ["TZ": "Pacific/Kiritimati"])

    let picker = app.datePickers["date"]
    XCTAssertTrue(picker.waitToExist(timeout: 2))
    assertLabel("date-value", "Date: 2026-11-15")
    XCTAssertEqual(shownDate(picker), "2026-11-15")
    stepFirstField(picker)
    let label = app.staticTexts["date-value"]
    XCTAssertTrue(waitUntil { label.currentValue != "Date: 2026-11-15" })
    XCTAssertEqual("Date: \(shownDate(picker) ?? "")", label.currentValue)
  }
}
