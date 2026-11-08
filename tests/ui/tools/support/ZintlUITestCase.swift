import Carbon
import XCTest

@MainActor
class ZintlUITestCase: XCTestCase {
  private(set) var app: XCUIApplication!

  override func setUpWithError() throws {
    continueAfterFailure = false
    useASCIIKeyboard()
  }

  /// Selects an ASCII keyboard for the test and restores the previous input source afterwards.
  ///
  /// An input method such as Japanese changes typed characters and shows a mode
  /// indicator that XCUITest treats as an interrupting dialog.
  private func useASCIIKeyboard() {
    let previous = TISCopyCurrentKeyboardInputSource().takeRetainedValue()
    let ascii = TISCopyCurrentASCIICapableKeyboardLayoutInputSource().takeRetainedValue()
    TISSelectInputSource(ascii)
    addTeardownBlock { @MainActor in
      TISSelectInputSource(previous)
    }
  }

  /// Launches the fixture app with `scenario` and waits for its main window.
  ///
  /// Pass `exitsOnItsOwn` when the test ends with the app quitting by itself.
  /// XCTest then waits about 20 seconds for a crash report if it launched the
  /// app, so such tests open the app through Launch Services and attach to it.
  func launch(
    scenario: String,
    environment: [String: String] = [:],
    exitsOnItsOwn: Bool = false,
    file: StaticString = #filePath,
    line: UInt = #line
  ) throws {
    let url = URL(
      fileURLWithPath: ProcessInfo.processInfo.environment["ZINTL_UI_TEST_APP"] ?? defaultAppPath)
    let arguments = ["--scenario", scenario]

    // A sandboxed runner drops launch arguments, so the app also reads them from here.
    let pasteboard = NSPasteboard(name: .ruler)
    let options = ["scenario=\(scenario)"] + environment.map { "env:\($0.key)=\($0.value)" }
    pasteboard.clearContents()
    pasteboard.setString(options.joined(separator: "\n"), forType: .string)
    addTeardownBlock { @MainActor in
      pasteboard.clearContents()
    }

    if exitsOnItsOwn {
      app = try openAndAttach(url, arguments: arguments, environment: environment)
    } else {
      app = XCUIApplication(url: url)
      app.launchArguments = arguments
      app.launchEnvironment.merge(environment) { _, value in value }
      app.launch()
    }
    let launchedApp = app!
    addTeardownBlock { @MainActor in
      launchedApp.terminate()
    }

    XCTAssertTrue(
      app.windows["main-window"].waitToExist(timeout: 5),
      "The main window did not appear",
      file: file,
      line: line
    )
  }

  /// Opens the app at `url` through Launch Services and returns an XCUIApplication attached to it.
  private func openAndAttach(
    _ url: URL,
    arguments: [String],
    environment: [String: String]
  ) throws -> XCUIApplication {
    let bundleIdentifier = try XCTUnwrap(
      Bundle(url: url)?.bundleIdentifier, "The app at \(url.path) has no bundle identifier")
    let attached = XCUIApplication(bundleIdentifier: bundleIdentifier)
    XCTAssertEqual(attached.state, .notRunning, "A previous app instance is still running")

    let configuration = NSWorkspace.OpenConfiguration()
    configuration.arguments = arguments
    configuration.environment = environment
    configuration.createsNewApplicationInstance = true
    NSWorkspace.shared.openApplication(at: url, configuration: configuration)

    XCTAssertTrue(
      waitUntil(timeout: 5) { attached.state != .notRunning }, "The app did not start")
    attached.activate()
    return attached
  }

  /// Waits until `element` reports `value`.
  func waitForValue(_ element: XCUIElement, _ value: String, timeout: TimeInterval = 2) -> Bool {
    waitUntil(timeout: timeout) { element.currentValue == value }
  }

  /// Returns the element with the accessibility identifier `id`, whatever its type.
  func element(_ id: String) -> XCUIElement {
    app.descendants(matching: .any)[id]
  }

  /// Asserts that the label `id` shows `value`, waiting for the app to update it.
  ///
  /// Scenarios echo their Stores in labels, so this proves native input reached Rust.
  func assertLabel(
    _ id: String,
    _ value: String,
    timeout: TimeInterval = 2,
    file: StaticString = #filePath,
    line: UInt = #line
  ) {
    let label = app.staticTexts[id]
    XCTAssertTrue(
      waitForValue(label, value, timeout: timeout),
      "Label \(id) shows \(label.currentValue ?? "nothing") instead of \(value)",
      file: file,
      line: line
    )
  }

  /// Polls the UI until `condition` holds or `timeout` passes, and returns whether it held.
  func waitUntil(timeout: TimeInterval = 2, _ condition: () -> Bool) -> Bool {
    Self.poll(timeout: timeout, condition)
  }

  /// Checks `condition` every 50 ms until it holds or `timeout` passes.
  ///
  /// XCTest's own waits recheck only once per second, so each miss costs a full
  /// second; every wait in these tests goes through this instead.
  static func poll(timeout: TimeInterval, _ condition: () -> Bool) -> Bool {
    let deadline = Date().addingTimeInterval(timeout)
    while Date() < deadline {
      if condition() {
        return true
      }
      RunLoop.current.run(until: Date().addingTimeInterval(0.05))
    }
    return condition()
  }

  /// Presses `element` at the normalized point `start` and drags it by `offset` points.
  func drag(
    _ element: XCUIElement,
    from start: CGVector = CGVector(dx: 0.5, dy: 0.5),
    by offset: CGVector
  ) {
    let origin = element.coordinate(withNormalizedOffset: start)
    origin.click(forDuration: 0.2, thenDragTo: origin.withOffset(offset))
  }

  /// Counts the pixels in a screenshot of `element` whose sRGB color satisfies `matches`.
  func pixelCount(in element: XCUIElement, where matches: (NSColor) -> Bool) -> Int {
    guard let data = element.screenshot().image.tiffRepresentation,
      let bitmap = NSBitmapImageRep(data: data)
    else {
      return 0
    }
    var count = 0
    for y in 0..<bitmap.pixelsHigh {
      for x in 0..<bitmap.pixelsWide {
        guard let color = bitmap.colorAt(x: x, y: y)?.usingColorSpace(.sRGB) else { continue }
        if matches(color) {
          count += 1
        }
      }
    }
    return count
  }

  /// Replaces the text of the focused `field` with `text` through the pasteboard.
  ///
  /// Typing goes through the input source, which may change characters such as
  /// `/`; pasting does not. The previous pasteboard text comes back afterwards.
  func paste(_ text: String, into field: XCUIElement) {
    let pasteboard = NSPasteboard.general
    let previous = pasteboard.string(forType: .string)
    addTeardownBlock { @MainActor in
      pasteboard.clearContents()
      if let previous {
        pasteboard.setString(previous, forType: .string)
      }
    }
    pasteboard.clearContents()
    pasteboard.setString(text, forType: .string)
    field.typeKey("a", modifierFlags: .command)
    field.typeKey("v", modifierFlags: .command)
  }

  /// Returns the sRGB color that a screenshot of `element` shows at the screen point `point`.
  func pixelColor(in element: XCUIElement, at point: CGPoint) -> NSColor? {
    let frame = element.frame
    guard let data = element.screenshot().image.tiffRepresentation,
      let bitmap = NSBitmapImageRep(data: data), frame.width > 0, frame.height > 0
    else {
      return nil
    }
    let x = Int((point.x - frame.minX) * CGFloat(bitmap.pixelsWide) / frame.width)
    let y = Int((point.y - frame.minY) * CGFloat(bitmap.pixelsHigh) / frame.height)
    guard (0..<bitmap.pixelsWide).contains(x), (0..<bitmap.pixelsHigh).contains(y) else {
      return nil
    }
    return bitmap.colorAt(x: x, y: y)?.usingColorSpace(.sRGB)
  }

  /// Creates an empty directory for files the app writes; the test removes it afterwards.
  func makeTemporaryDirectory() throws -> URL {
    let directory = FileManager.default.temporaryDirectory
      .appendingPathComponent("zintl-ui-\(UUID().uuidString)")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    addTeardownBlock {
      try? FileManager.default.removeItem(at: directory)
    }
    return directory
  }

  private var defaultAppPath: String {
    URL(fileURLWithPath: #filePath)
      .deletingLastPathComponent()
      .deletingLastPathComponent()
      .deletingLastPathComponent()
      .appendingPathComponent("target/ZintlUITestApp.app")
      .path
  }
}

@MainActor
extension XCUIElement {
  /// Waits until the element exists, without `waitForExistence`'s one-second polling.
  func waitToExist(timeout: TimeInterval) -> Bool {
    ZintlUITestCase.poll(timeout: timeout) { exists }
  }

  /// Waits until the element is gone, without `waitForNonExistence`'s one-second polling.
  func waitToDisappear(timeout: TimeInterval) -> Bool {
    ZintlUITestCase.poll(timeout: timeout) { !exists }
  }

  /// Waits until the element exists and can be clicked.
  func waitToBeHittable(timeout: TimeInterval) -> Bool {
    ZintlUITestCase.poll(timeout: timeout) { exists && isHittable }
  }

  /// The element's value as text, or nil when it is missing; unlike `value`, never fails the test.
  var currentValue: String? {
    (try? snapshot())?.value as? String
  }
}
