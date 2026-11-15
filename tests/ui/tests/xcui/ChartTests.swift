import XCTest

@MainActor
final class ChartTests: ZintlUITestCase {
  /// Returns the bar labeled `label` in the first chart, an image element named after its data.
  private func breakdownBar(_ label: String) -> XCUIElement {
    app.groups["breakdown"].images[label]
  }

  /// Returns the bar with the ID `id`.
  private func bar(_ id: String) -> XCUIElement {
    app.images[id]
  }

  /// Verifies each bar is an accessibility element with its label and formatted value.
  func testBarsReportTheirLabelsAndValues() throws {
    try launch(scenario: "chart")

    for (label, value) in [("Food", "3480"), ("Transport", "1200"), ("Rent", "2400")] {
      let element = breakdownBar(label)
      XCTAssertTrue(element.waitToExist(timeout: 2), label)
      XCTAssertEqual(element.label, label)
      XCTAssertEqual(element.value as? String, value, label)
    }
  }

  /// Verifies bar lengths follow their values, and a doubled value doubles its bar.
  func testDoublingAValueDoublesItsBar() throws {
    try launch(scenario: "chart")

    let food = breakdownBar("Food")
    let transport = breakdownBar("Transport")
    XCTAssertTrue(transport.waitToExist(timeout: 2))
    let before = transport.frame.width
    XCTAssertEqual(before / food.frame.width, 1200.0 / 3480.0, accuracy: 0.02)

    app.buttons["double-transport"].click()
    XCTAssertTrue(waitForValue(transport, "2400"))
    XCTAssertEqual(transport.frame.width, before * 2, accuracy: 1)
    XCTAssertEqual(transport.frame.width, breakdownBar("Rent").frame.width, accuracy: 1)
  }

  /// Verifies short Japanese labels get columns as wide as their text, and bars share one start.
  func testShortJapaneseLabelsAreNotTruncated() throws {
    try launch(scenario: "chart")

    let group = app.groups["cjk"]
    let utilities = bar("cjk-utilities")
    XCTAssertTrue(utilities.waitToExist(timeout: 2))
    let font = NSFont.systemFont(ofSize: 13)
    let textWidth = ("水道光熱費" as NSString).size(withAttributes: [.font: font]).width
    // The label column ends 8 points before the track.
    let column = utilities.frame.minX - group.frame.minX - 8
    XCTAssertGreaterThanOrEqual(column, textWidth.rounded(.up))
    XCTAssertEqual(bar("cjk-salary").frame.minX, utilities.frame.minX, accuracy: 0.5)
  }

  /// Verifies bars with the same label in two charts are told apart by ID, inside their groups.
  func testBarsAreFoundByIDInsideTheirChart() throws {
    try launch(scenario: "chart")

    let expensesOther = bar("expenses-other")
    let incomeOther = bar("income-other")
    XCTAssertTrue(expensesOther.waitToExist(timeout: 2))
    XCTAssertEqual(expensesOther.value as? String, "300")
    XCTAssertEqual(incomeOther.value as? String, "600")
    XCTAssertNotEqual(expensesOther.frame, incomeOther.frame)
    XCTAssertEqual(app.groups["expenses"].images.count, 3)
    XCTAssertEqual(app.groups["income"].images.count, 3)
  }

  /// Verifies a bar at half the chart's maximum fills half the track.
  func testMaximumSetsTheValueThatFillsTheTrack() throws {
    try launch(scenario: "chart")

    let rent = bar("expenses-rent")
    XCTAssertTrue(rent.waitToExist(timeout: 2))
    XCTAssertEqual(bar("expenses-food").frame.width, rent.frame.width / 2, accuracy: 1)
  }
}
