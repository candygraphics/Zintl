import Foundation
import XCTest

@testable import RuntimeJSCFFI

final class RuntimeJSCFFITests: XCTestCase {
  /// Starts `engine` with the bridge script Rust passes in production.
  private func start(_ engine: UnsafeMutableRawPointer?) -> UInt32 {
    let script = URL(fileURLWithPath: #filePath)
      .appendingPathComponent("../../../../crates/runtime-engine/src/bootstrap.js")
      .standardizedFileURL
    let bytes = Array((try? Data(contentsOf: script)) ?? Data())
    return bytes.withUnsafeBufferPointer { zjscEngineStart(engine, $0.baseAddress, $0.count) }
  }

  // Verifies the private C ABI evaluates a Promise and preserves non-consuming size queries.
  func testPromiseEvaluationEvent() throws {
    var engine: UnsafeMutableRawPointer?
    XCTAssertEqual(zjscEngineNew(8, 8, 4096, 65536, nil, nil, &engine), 0)
    defer { zjscEngineFree(engine) }
    XCTAssertEqual(start(engine), 0)
    let source = Data("Promise.resolve(42)".utf8)
    XCTAssertEqual(
      source.withUnsafeBytes { bytes in
        zjscEngineSubmit(
          engine,
          1,
          bytes.bindMemory(to: UInt8.self).baseAddress,
          bytes.count
        )
      },
      0
    )
    // Evaluation runs synchronously, so its event is queued once submit returns.
    var required = 0
    XCTAssertEqual(zjscEngineNextEvent(engine, nil, 0, &required), 2)
    XCTAssertGreaterThan(required, 20)
    var event = [UInt8](repeating: 0, count: required)
    XCTAssertEqual(zjscEngineNextEvent(engine, &event, event.count, &required), 0)
    XCTAssertEqual(Array(event.prefix(4)), Array("ZJE1".utf8))
    XCTAssertEqual(zjscEngineShutdown(engine), 0)
  }

  // Verifies an oversized evaluation result produces a bounded terminal error event.
  func testEvaluationQueueOverflowTerminatesEvaluation() throws {
    var engine: UnsafeMutableRawPointer?
    XCTAssertEqual(zjscEngineNew(2, 2, 4096, 32, nil, nil, &engine), 0)
    defer { zjscEngineFree(engine) }
    XCTAssertEqual(start(engine), 0)
    let source = Data("'x'.repeat(1000)".utf8)
    XCTAssertEqual(
      source.withUnsafeBytes { bytes in
        zjscEngineSubmit(
          engine,
          1,
          bytes.bindMemory(to: UInt8.self).baseAddress,
          bytes.count
        )
      },
      0
    )
    // Evaluation runs synchronously, so its event is queued once submit returns.
    var required = 0
    XCTAssertEqual(zjscEngineNextEvent(engine, nil, 0, &required), 2)
    var event = [UInt8](repeating: 0, count: required)
    XCTAssertEqual(zjscEngineNextEvent(engine, &event, event.count, &required), 0)
    XCTAssertEqual(Array(event.prefix(4)), Array("ZJE1".utf8))
    XCTAssertEqual(Array(event[6...7]), [0, 2])
    XCTAssertTrue(String(decoding: event, as: UTF8.self).contains("QuotaExceeded"))
  }

  // Verifies evaluation identities cannot alias through JavaScript number conversion.
  func testUnsafeIntegerEvaluationIdentityIsRejected() throws {
    var engine: UnsafeMutableRawPointer?
    XCTAssertEqual(zjscEngineNew(2, 2, 4096, 4096, nil, nil, &engine), 0)
    defer { zjscEngineFree(engine) }
    XCTAssertEqual(start(engine), 0)
    let source = Data("42".utf8)
    XCTAssertEqual(
      source.withUnsafeBytes { bytes in
        zjscEngineSubmit(
          engine,
          9_007_199_254_740_992,
          bytes.bindMemory(to: UInt8.self).baseAddress,
          bytes.count
        )
      },
      3
    )
  }
}
