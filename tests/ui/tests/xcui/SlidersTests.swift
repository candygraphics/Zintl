import XCTest

@MainActor
final class SlidersTests: ZintlUITestCase {
  /// Verifies adjusting a slider writes the new value to its Store and runs on_change.
  func testSliderAdjustmentWritesItsStore() throws {
    try launch(scenario: "sliders")

    let slider = app.sliders["free-slider"]
    XCTAssertTrue(slider.waitToExist(timeout: 2))
    slider.adjust(toNormalizedSliderPosition: 0.4)
    // Adjusting lands within a point of the requested position.
    let value = app.staticTexts["free-value"]
    XCTAssertTrue(waitUntil { abs((Double(value.value as? String ?? "") ?? -1) - 40) <= 1 })
    // Clicking the track moves the knob there.
    slider.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5)).click()
    XCTAssertTrue(waitUntil { slider.normalizedSliderPosition > 0.85 })
    assertLabel("changes", "Changed")
  }

  /// Verifies dragging the knob past the end of the track writes the maximum.
  func testDraggingTheKnobReachesTheMaximum() throws {
    try launch(scenario: "sliders")

    let slider = app.sliders["free-slider"]
    XCTAssertTrue(slider.waitToExist(timeout: 2))
    // The knob starts at the left end of the track.
    drag(slider, from: CGVector(dx: 0.03, dy: 0.5), by: CGVector(dx: slider.frame.width, dy: 0))
    assertLabel("free-value", "100")
  }

  /// Verifies a slider with snapping ticks only stores tick values.
  func testSnappingSliderStoresOnlyTickValues() throws {
    try launch(scenario: "sliders")

    let slider = app.sliders["snapping-slider"]
    XCTAssertTrue(slider.waitToExist(timeout: 2))
    // Five ticks over 0...100 sit 25 apart, so 0.4 snaps to 50.
    slider.adjust(toNormalizedSliderPosition: 0.4)
    assertLabel("snapping-value", "50.00")
  }

  /// Verifies writing the Stores from the app moves the native knobs.
  func testStoreChangesMoveTheKnobs() throws {
    try launch(scenario: "sliders")

    let free = app.sliders["free-slider"]
    XCTAssertTrue(free.waitToExist(timeout: 2))
    app.buttons["set-75"].click()
    assertLabel("free-value", "75")
    XCTAssertTrue(waitUntil { free.normalizedSliderPosition > 0.7 })
    XCTAssertEqual(free.normalizedSliderPosition, 0.75, accuracy: 0.02)
    XCTAssertEqual(app.sliders["snapping-slider"].normalizedSliderPosition, 0.75, accuracy: 0.02)
    // An app-side write is not a user change.
    assertLabel("changes", "Unchanged")
  }

  /// Verifies a full-width slider spans the window while a disabled slider ignores input.
  func testFullWidthAndDisabledSliders() throws {
    try launch(scenario: "sliders")

    let window = app.windows["main-window"]
    let wide = app.sliders["wide-slider"]
    let free = app.sliders["free-slider"]
    XCTAssertTrue(wide.waitToExist(timeout: 2))
    XCTAssertEqual(free.frame.width, 200, accuracy: 4)
    XCTAssertEqual(wide.frame.maxX, window.frame.maxX, accuracy: 2)
    XCTAssertGreaterThan(wide.frame.width, free.frame.width)

    let disabled = app.sliders["disabled-slider"]
    XCTAssertFalse(disabled.isEnabled)
    disabled.click()
    XCTAssertEqual(disabled.normalizedSliderPosition, 0, accuracy: 0.01)
  }
}
