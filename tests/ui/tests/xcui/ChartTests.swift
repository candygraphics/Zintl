import XCTest

@MainActor
final class ChartTests: ZintlUITestCase {
  /// Returns the bar labeled `label`, an image element named after its data.
  private func bar(_ label: String) -> XCUIElement {
    app.images[label]
  }

  /// Verifies each bar is an accessibility element with its label and formatted value.
  func testBarsReportTheirLabelsAndValues() throws {
    try launch(scenario: "chart")

    for (label, value) in [("Food", "3480"), ("Transport", "1200"), ("Rent", "2400")] {
      let element = bar(label)
      XCTAssertTrue(element.waitToExist(timeout: 2), label)
      XCTAssertEqual(element.label, label)
      XCTAssertEqual(element.value as? String, value, label)
    }
  }

  /// Verifies bar lengths follow their values, and a doubled value doubles its bar.
  func testDoublingAValueDoublesItsBar() throws {
    try launch(scenario: "chart")

    let food = bar("Food")
    let transport = bar("Transport")
    XCTAssertTrue(transport.waitToExist(timeout: 2))
    let before = transport.frame.width
    XCTAssertEqual(before / food.frame.width, 1200.0 / 3480.0, accuracy: 0.02)

    app.buttons["double-transport"].click()
    XCTAssertTrue(waitForValue(transport, "2400"))
    XCTAssertEqual(transport.frame.width, before * 2, accuracy: 1)
    XCTAssertEqual(transport.frame.width, bar("Rent").frame.width, accuracy: 1)
  }
}
