import XCTest

@MainActor
final class GridVariantsTests: ZintlUITestCase {
  /// Returns the brightness behind the right end of each row whose IDs start with `prefix`.
  private func rowBrightness(_ prefix: String) -> [CGFloat] {
    let window = app.windows["main-window"]
    return (0..<4).map { row in
      let frame = app.staticTexts["\(prefix)-\(row)"].frame
      let color = pixelColor(in: window, at: CGPoint(x: frame.maxX - 4, y: frame.midY))
      return color?.brightnessComponent ?? -1
    }
  }

  /// Verifies a striped grid shades every other row while a plain grid shades none.
  func testStripedGridShadesAlternateRows() throws {
    try launch(scenario: "grid-variants")

    XCTAssertTrue(app.staticTexts["plain-3"].waitToExist(timeout: 2))
    let striped = rowBrightness("striped")
    XCTAssertGreaterThan(abs(striped[0] - striped[1]), 0.01, "\(striped)")
    XCTAssertEqual(striped[0], striped[2], accuracy: 0.005)
    XCTAssertEqual(striped[1], striped[3], accuracy: 0.005)

    let plain = rowBrightness("plain")
    XCTAssertEqual(plain[0], plain[1], accuracy: 0.005, "\(plain)")
    XCTAssertEqual(plain[0], striped[0], accuracy: 0.005)
  }

  /// Verifies a heading cell spans every column and the columns line up above and below it.
  func testSpanningHeadingCoversEveryColumn() throws {
    try launch(scenario: "grid-variants")

    let heading = app.staticTexts["heading"]
    XCTAssertTrue(heading.waitToExist(timeout: 2))
    let first = app.staticTexts["food-0"]
    let last = app.staticTexts["food-3"]
    XCTAssertEqual(heading.frame.minX, first.frame.minX, accuracy: 0.5)
    XCTAssertEqual(heading.frame.maxX, last.frame.maxX, accuracy: 0.5)
    // The heading sits on its own row between the two groups.
    XCTAssertGreaterThan(heading.frame.minY, first.frame.maxY)
    XCTAssertLessThan(heading.frame.maxY, app.staticTexts["bus-0"].frame.minY)
    for column in 0..<4 {
      XCTAssertEqual(
        app.staticTexts["food-\(column)"].frame.minX,
        app.staticTexts["bus-\(column)"].frame.minX,
        accuracy: 0.5,
        "column \(column)"
      )
    }
  }
}
