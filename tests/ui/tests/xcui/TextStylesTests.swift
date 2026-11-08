import XCTest

@MainActor
final class TextStylesTests: ZintlUITestCase {
  /// Counts the pixels of `id` that differ clearly from its top-left background pixel.
  private func inkPixels(_ id: String) -> Int {
    let label = app.staticTexts[id]
    guard
      let background = pixelColor(
        in: label, at: CGPoint(x: label.frame.minX + 1, y: label.frame.minY + 1))
    else {
      return 0
    }
    return pixelCount(in: label) { color in
      abs(color.brightnessComponent - background.brightnessComponent) > 0.3
    }
  }

  /// Verifies a larger font makes a label taller and bold draws heavier strokes.
  func testFontSizeAndWeightChangeTheLabel() throws {
    try launch(scenario: "text-styles")

    let small = app.staticTexts["small"].frame
    let large = app.staticTexts["large"].frame
    let bold = app.staticTexts["large-bold"].frame
    XCTAssertGreaterThan(large.height, small.height * 1.5)
    XCTAssertEqual(bold.height, large.height, accuracy: 2)
    // Labels stretch to the window width, so weight shows only in the drawn pixels.
    XCTAssertGreaterThan(inkPixels("large"), inkPixels("small") * 2)
    XCTAssertGreaterThan(Double(inkPixels("large-bold")), Double(inkPixels("large")) * 1.2)
  }

  /// Verifies a custom text color reaches the screen.
  func testCustomColorIsDrawn() throws {
    try launch(scenario: "text-styles")

    let isRed = { (color: NSColor) in
      color.redComponent > 0.8 && color.greenComponent < 0.3 && color.blueComponent < 0.3
    }
    XCTAssertGreaterThan(pixelCount(in: app.staticTexts["red"], where: isRed), 20)
    XCTAssertEqual(pixelCount(in: app.staticTexts["large-bold"], where: isRed), 0)
  }

  /// Verifies line_limit(2) wraps onto two lines while line_limit(1) keeps one.
  func testLineLimitWrapsUpToTheLimit() throws {
    try launch(scenario: "text-styles")

    let one = app.staticTexts["one-line"].frame
    let two = app.staticTexts["two-lines"].frame
    XCTAssertGreaterThan(two.height, one.height * 1.5)
    XCTAssertLessThan(two.height, one.height * 2.5)
    XCTAssertEqual(two.width, one.width, accuracy: 1)
  }
}
