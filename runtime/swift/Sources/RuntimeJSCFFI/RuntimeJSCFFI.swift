import Foundation
import JavaScriptCore
import RuntimeJSCShim

private let ok: UInt32 = 0
private let empty: UInt32 = 1
private let bufferTooSmall: UInt32 = 2
private let invalidArgument: UInt32 = 3
private let invalidState: UInt32 = 4
private let quotaExceeded: UInt32 = 5
private let backendError: UInt32 = 255

private let maximumSafeInteger: UInt64 = (1 << 53) - 1

public typealias NotifyFunction = @convention(c) (UnsafeMutableRawPointer?) -> Void

private struct PendingPromise {
  let resolve: JSValue
  let reject: JSValue
}

/// One JavaScriptCore context behind the C ABI.
///
/// Rust calls every entry point on the engine's owning thread, and JavaScript
/// callbacks run inside those calls, so the state needs no locks. Engine-
/// independent request limits are enforced by Rust after decoding; this side
/// checks only what the wire format itself needs.
private final class JSCFFIEngine {
  init(
    maximumEvaluations: Int,
    maximumHostRequests: Int,
    maximumSourceBytes: Int,
    maximumEventBytes: Int,
    notify: NotifyFunction?,
    userData: UnsafeMutableRawPointer?
  ) {
    self.maximumEvaluations = maximumEvaluations
    self.maximumHostRequests = maximumHostRequests
    self.maximumSourceBytes = maximumSourceBytes
    self.maximumEventBytes = maximumEventBytes
    self.notify = notify
    self.userData = userData
    runtimeID = UInt64.random(in: 1...UInt64.max)
  }

  func start(bootstrap: String) -> UInt32 {
    guard context == nil, !shuttingDown else { return invalidState }
    guard let context = JSContext() else { return backendError }
    self.context = context
    installCallbacks(in: context)
    guard let bridge = context.evaluateScript(bootstrap), context.exception == nil else {
      context.exception = nil
      self.context = nil
      return backendError
    }
    self.bridge = bridge
    return ok
  }

  func submit(evaluationID: UInt64, source: Data) -> UInt32 {
    guard evaluationID != 0, evaluationID <= maximumSafeInteger,
      source.count <= maximumSourceBytes,
      let source = String(data: source, encoding: .utf8)
    else { return invalidArgument }
    guard !shuttingDown else { return invalidState }
    guard !pendingEvaluations.values.contains(evaluationID) else { return invalidArgument }
    guard terminalEvents.count < maximumEvaluations,
      pendingEvaluations.count < maximumEvaluations,
      nextEvaluationToken <= maximumSafeInteger
    else { return quotaExceeded }
    let token = nextEvaluationToken
    nextEvaluationToken += 1
    pendingEvaluations[token] = evaluationID
    evaluate(token: token, source: source)
    return ok
  }

  func nextEvent(output: UnsafeMutablePointer<UInt8>?, capacity: Int, required: inout Int) -> UInt32
  {
    let terminal = events.isEmpty ? terminalEvents.first : nil
    guard let event = events.first ?? terminal else {
      required = 0
      return empty
    }
    required = event.count
    guard capacity >= event.count, let output else { return bufferTooSmall }
    event.copyBytes(to: output, count: event.count)
    if terminal != nil {
      terminalEvents.removeFirst()
    } else {
      events.removeFirst()
      queuedEventBytes -= event.count
    }
    return ok
  }

  func complete(
    requestID: UInt64,
    kind: UInt32,
    objectID: UInt64,
    payload: Data
  ) -> UInt32 {
    guard !shuttingDown, let promise = promises.removeValue(forKey: requestID) else {
      return invalidArgument
    }
    settle(promise, kind: kind, objectID: objectID, payload: payload)
    return ok
  }

  func microtaskCheckpoint() -> UInt32 {
    guard let context, !shuttingDown else { return invalidState }
    // JavaScriptCore drains Promise jobs at the end of a host entry. A no-op
    // entry gives jobs queued by native Promise settlement an explicit checkpoint.
    _ = context.evaluateScript("void 0")
    if context.exception != nil {
      context.exception = nil
      return backendError
    }
    return ok
  }

  func cancel(evaluationID: UInt64) -> UInt32 {
    guard let token = pendingEvaluations.first(where: { $0.value == evaluationID })?.key else {
      return invalidArgument
    }
    settle(token: token, kind: 3, payload: Data())
    return ok
  }

  func shutdown() -> UInt32 {
    guard !shuttingDown else { return ok }
    shuttingDown = true
    for promise in promises.values {
      reject(promise.reject, "Runtime is shutting down", code: "ShuttingDown")
    }
    promises.removeAll()
    pendingEvaluations.removeAll()
    bridge = nil
    context = nil
    return ok
  }

  private func installCallbacks(in context: JSContext) {
    typealias EvaluationDone = @convention(block) (Double, NSString) -> Void
    let evaluationDone: EvaluationDone = { [weak self] token, value in
      guard let self, let token = Self.exactUInt64(token) else { return }
      settle(token: token, kind: 1, payload: Data((value as String).utf8))
    }
    typealias EvaluationFailed = @convention(block) (Double, NSString, NSString) -> Void
    let evaluationFailed: EvaluationFailed = { [weak self] token, name, message in
      guard let self, let token = Self.exactUInt64(token) else { return }
      fail(token: token, name: name as String, message: message as String)
    }
    typealias Invoke = @convention(block) (NSString, JSValue, JSValue, JSValue) -> Void
    let invoke: Invoke = { [weak self] name, input, resolve, reject in
      guard let self else { return }
      submitHost(
        operation: 1, payload: invokePayload(name: name as String, input: input),
        invalid: "Invalid operation input", resolve: resolve, reject: reject)
    }
    typealias Sleep = @convention(block) (Double, JSValue, JSValue) -> Void
    let sleep: Sleep = { [weak self] milliseconds, resolve, reject in
      guard let self else { return }
      submitHost(
        operation: 2, payload: sleepPayload(milliseconds: milliseconds),
        invalid: "Invalid timer delay", resolve: resolve, reject: reject)
    }
    typealias ReadFile = @convention(block) (NSString, Double, JSValue, JSValue) -> Void
    let readFile: ReadFile = { [weak self] url, maximum, resolve, reject in
      guard let self else { return }
      submitHost(
        operation: 10, payload: readFilePayload(url: url as String, maximum: maximum),
        invalid: "Invalid mount read request", resolve: resolve, reject: reject)
    }
    typealias MountOperation =
      @convention(block) (Double, NSString, JSValue, JSValue, JSValue) ->
      Void
    let mountOperation: MountOperation = { [weak self] kind, url, operand, resolve, reject in
      guard let self else { return }
      let operation = Self.exactUInt64(kind).flatMap { UInt16(exactly: $0) } ?? 0
      submitHost(
        operation: operation,
        payload: mountPayload(operation: operation, url: url as String, operand: operand),
        invalid: "Invalid mount operation", resolve: resolve, reject: reject)
    }
    typealias Console = @convention(block) (NSString) -> Void
    let console: Console = { [weak self] message in
      self?.emitConsole(message as String)
    }
    // Lone surrogates become U+FFFD when the string is converted to UTF-8.
    typealias EncodeUtf8 = @convention(block) (NSString) -> JSValue?
    let encodeUtf8: EncodeUtf8 = { [weak self] text in
      self?.uint8Array(Data((text as String).utf8))
    }
    // Returning nil yields `undefined` for invalid UTF-8.
    typealias DecodeUtf8 = @convention(block) (JSValue) -> NSString?
    let decodeUtf8: DecodeUtf8 = { [weak self] bytes in
      guard let data = self?.bytes(bytes, maximum: .max) else { return nil }
      return String(data: data, encoding: .utf8) as NSString?
    }
    let callbacks: [(String, Any)] = [
      ("__zintlEvaluationDone", evaluationDone),
      ("__zintlEvaluationFailed", evaluationFailed),
      ("__zintlInvoke", invoke),
      ("__zintlSleep", sleep),
      ("__zintlReadFile", readFile),
      ("__zintlMountOperation", mountOperation),
      ("__zintlConsole", console),
      ("__zintlEncodeUtf8", encodeUtf8),
      ("__zintlDecodeUtf8", decodeUtf8),
    ]
    for (name, callback) in callbacks {
      context.setObject(callback, forKeyedSubscript: name as NSString)
    }
  }

  private func emitConsole(_ message: String) {
    enqueue(Self.event(kind: 5, id: 0, payload: Data(message.utf8)))
  }

  /// Runs the bridge with a private settlement token, never the public evaluation ID.
  private func evaluate(token: UInt64, source: String) {
    guard let bridge, !shuttingDown else {
      fail(token: token, name: "Error", message: "Runtime is shutting down")
      return
    }
    _ = bridge.invokeMethod("evaluate", withArguments: [Double(token), source])
    if context?.exception != nil {
      context?.exception = nil
      fail(token: token, name: "Error", message: "JavaScript evaluation failed")
    }
  }

  private func fail(token: UInt64, name: String, message: String) {
    settle(token: token, kind: 2, payload: Self.exceptionPayload(name: name, message: message))
  }

  /// Emits one settlement event for `token`; stale tokens are ignored.
  private func settle(token: UInt64, kind: UInt16, payload: Data) {
    guard let evaluationID = pendingEvaluations.removeValue(forKey: token) else { return }
    if !enqueue(Self.event(kind: kind, id: evaluationID, payload: payload)) {
      let fallback = Self.event(
        kind: 2, id: evaluationID,
        payload: Self.exceptionPayload(
          name: "QuotaExceeded", message: "Engine event queue exhausted"))
      if terminalEvents.count < maximumEvaluations { terminalEvents.append(fallback) }
      notify?(userData)
    }
  }

  private func invokePayload(name: String, input: JSValue) -> Data? {
    guard let length = UInt16(exactly: name.utf8.count),
      let input = bytes(input, maximum: maximumEventBytes)
    else { return nil }
    var payload = Data()
    Self.append(UInt32(1), to: &payload)
    Self.append(length, to: &payload)
    payload.append(contentsOf: name.utf8)
    payload.append(input)
    return payload
  }

  private func sleepPayload(milliseconds: Double) -> Data? {
    guard let milliseconds = Self.exactUInt64(milliseconds) else { return nil }
    let (nanoseconds, overflow) = milliseconds.multipliedReportingOverflow(by: 1_000_000)
    guard !overflow else { return nil }
    var payload = Data()
    Self.append(nanoseconds, to: &payload)
    return payload
  }

  private func readFilePayload(url: String, maximum: Double) -> Data? {
    guard let maximum = Self.exactUInt64(maximum) else { return nil }
    var payload = Data()
    Self.append(maximum, to: &payload)
    payload.append(contentsOf: url.utf8)
    return payload
  }

  private func mountPayload(operation: UInt16, url: String, operand: JSValue) -> Data? {
    var payload = Data()
    switch operation {
    case 11:
      guard let length = UInt16(exactly: url.utf8.count),
        let bytes = bytes(operand, maximum: maximumEventBytes)
      else { return nil }
      Self.append(length, to: &payload)
      payload.append(contentsOf: url.utf8)
      payload.append(bytes)
    case 12...14:
      payload.append(contentsOf: url.utf8)
    case 15:
      guard operand.isString, let destination = operand.toString(),
        let length = UInt16(exactly: url.utf8.count)
      else { return nil }
      Self.append(length, to: &payload)
      payload.append(contentsOf: url.utf8)
      payload.append(contentsOf: destination.utf8)
    default:
      return nil
    }
    return payload
  }

  /// Queues one host request, or rejects at once when `payload` could not be built.
  private func submitHost(
    operation: UInt16, payload: Data?, invalid: String, resolve: JSValue, reject: JSValue
  ) {
    guard let payload else {
      self.reject(reject, invalid, code: "InvalidRequest")
      return
    }
    guard !shuttingDown, promises.count < maximumHostRequests, nextHostID <= maximumSafeInteger
    else {
      self.reject(reject, "Host request limit exceeded", code: "QuotaExceeded")
      return
    }
    let requestID = nextHostID
    nextHostID += 1
    guard enqueue(Self.event(kind: 4, id: requestID, operation: operation, payload: payload)) else {
      self.reject(reject, "Engine event queue exhausted", code: "QuotaExceeded")
      return
    }
    promises[requestID] = PendingPromise(resolve: resolve, reject: reject)
  }

  private func settle(_ promise: PendingPromise, kind: UInt32, objectID: UInt64, payload: Data) {
    switch kind {
    case 0:
      _ = promise.resolve.call(withArguments: [])
    case 1:
      guard let bytes = uint8Array(payload) else {
        reject(promise.reject, "Host operation failed", code: "OperationFailed")
        return
      }
      _ = promise.resolve.call(withArguments: [bytes])
    case 2, 3:
      // Completion kinds 2 and 3 carry Directory and File objects; the shim brands them 1 and 2.
      guard let context,
        let object = rtjsc_host_object_make(
          context.jsGlobalContextRef, runtimeID, objectID, kind - 1),
        let value = JSValue(jsValueRef: object, in: context)
      else {
        reject(promise.reject, "Host operation failed", code: "OperationFailed")
        return
      }
      _ = promise.resolve.call(withArguments: [value])
    default:
      let (code, message) =
        Self.failure(payload) ?? ("OperationFailed", "Host operation failed")
      reject(promise.reject, message, code: code)
    }
  }

  /// Copies a `Uint8Array` of at most `maximum` bytes, honoring its byte offset.
  private func bytes(_ value: JSValue, maximum: Int) -> Data? {
    guard let context else { return nil }
    let ctx = context.jsGlobalContextRef
    guard JSValueGetTypedArrayType(ctx, value.jsValueRef, nil) == kJSTypedArrayTypeUint8Array,
      let view = JSValueToObject(ctx, value.jsValueRef, nil)
    else { return nil }
    let length = JSObjectGetTypedArrayByteLength(ctx, view, nil)
    guard length <= maximum else { return nil }
    guard length > 0 else { return Data() }
    let offset = JSObjectGetTypedArrayByteOffset(ctx, view, nil)
    guard let buffer = JSObjectGetTypedArrayBuffer(ctx, view, nil),
      let base = JSObjectGetArrayBufferBytesPtr(ctx, buffer, nil)
    else { return nil }
    return Data(bytes: base + offset, count: length)
  }

  private func uint8Array(_ data: Data) -> JSValue? {
    guard let context else { return nil }
    let ctx = context.jsGlobalContextRef
    guard let array = JSObjectMakeTypedArray(ctx, kJSTypedArrayTypeUint8Array, data.count, nil)
    else { return nil }
    if !data.isEmpty {
      guard let target = JSObjectGetTypedArrayBytesPtr(ctx, array, nil) else { return nil }
      data.copyBytes(to: target.assumingMemoryBound(to: UInt8.self), count: data.count)
    }
    return JSValue(jsValueRef: array, in: context)
  }

  @discardableResult
  private func enqueue(_ event: Data) -> Bool {
    let accepted =
      terminalEvents.isEmpty
      && event.count <= maximumEventBytes
      && queuedEventBytes <= maximumEventBytes - event.count
    if accepted {
      events.append(event)
      queuedEventBytes += event.count
      notify?(userData)
    }
    return accepted
  }

  private func reject(_ reject: JSValue, _ message: String, code: String) {
    guard let context, let error = JSValue(newErrorFromMessage: message, in: context) else {
      _ = reject.call(withArguments: [message])
      return
    }
    error.setValue(code, forProperty: "code")
    _ = reject.call(withArguments: [error])
  }

  /// Builds one `ZJE1` event; host requests also carry their operation code.
  private static func event(kind: UInt16, id: UInt64, operation: UInt16? = nil, payload: Data)
    -> Data
  {
    var event = Data("ZJE1".utf8)
    append(UInt16(1), to: &event)
    append(kind, to: &event)
    append(id, to: &event)
    if let operation { append(operation, to: &event) }
    append(UInt32(payload.count), to: &event)
    event.append(payload)
    return event
  }

  private static func exceptionPayload(name: String, message: String) -> Data {
    let name = name.utf8.count <= Int(UInt16.max) ? name : "Error"
    var payload = Data()
    append(UInt16(name.utf8.count), to: &payload)
    payload.append(contentsOf: name.utf8)
    payload.append(contentsOf: message.utf8)
    return payload
  }

  /// Splits a failure completion into its code and message.
  private static func failure(_ payload: Data) -> (String, String)? {
    let bytes = [UInt8](payload)
    guard bytes.count >= 2 else { return nil }
    let codeEnd = 2 + (Int(bytes[0]) << 8 | Int(bytes[1]))
    guard bytes.count >= codeEnd,
      let code = String(bytes: bytes[2..<codeEnd], encoding: .utf8),
      let message = String(bytes: bytes[codeEnd...], encoding: .utf8)
    else { return nil }
    return (code, message)
  }

  private static func exactUInt64(_ value: Double) -> UInt64? {
    guard value.isFinite, value >= 0, value <= Double(maximumSafeInteger),
      value.rounded(.down) == value
    else { return nil }
    return UInt64(value)
  }

  private static func append<T: FixedWidthInteger>(_ value: T, to data: inout Data) {
    withUnsafeBytes(of: value.bigEndian) { data.append(contentsOf: $0) }
  }

  private let maximumEvaluations: Int
  private let maximumHostRequests: Int
  private let maximumSourceBytes: Int
  private let maximumEventBytes: Int
  private let notify: NotifyFunction?
  private let userData: UnsafeMutableRawPointer?
  private let runtimeID: UInt64
  private var context: JSContext?
  private var bridge: JSValue?
  private var promises: [UInt64: PendingPromise] = [:]
  /// Maps private settlement tokens to public evaluation IDs.
  private var pendingEvaluations: [UInt64: UInt64] = [:]
  private var nextEvaluationToken: UInt64 = 1
  private var nextHostID: UInt64 = 1
  private var shuttingDown = false
  private var events: [Data] = []
  private var terminalEvents: [Data] = []
  private var queuedEventBytes = 0
}

private func engine(_ pointer: UnsafeMutableRawPointer?) -> JSCFFIEngine? {
  guard let pointer else { return nil }
  return Unmanaged<JSCFFIEngine>.fromOpaque(pointer).takeUnretainedValue()
}

@_cdecl("zjsc_engine_new")
public func zjscEngineNew(
  _ maximumEvaluations: UInt32, _ maximumHostRequests: UInt32,
  _ maximumSourceBytes: UInt32, _ maximumEventBytes: UInt32,
  _ notify: NotifyFunction?, _ userData: UnsafeMutableRawPointer?,
  _ output: UnsafeMutablePointer<UnsafeMutableRawPointer?>?
) -> UInt32 {
  guard maximumEvaluations > 0, maximumHostRequests > 0, maximumSourceBytes > 0,
    maximumEventBytes > 0, let output
  else { return invalidArgument }
  let value = JSCFFIEngine(
    maximumEvaluations: Int(maximumEvaluations), maximumHostRequests: Int(maximumHostRequests),
    maximumSourceBytes: Int(maximumSourceBytes), maximumEventBytes: Int(maximumEventBytes),
    notify: notify, userData: userData)
  output.pointee = Unmanaged.passRetained(value).toOpaque()
  return ok
}

@_cdecl("zjsc_engine_start")
public func zjscEngineStart(
  _ pointer: UnsafeMutableRawPointer?, _ script: UnsafePointer<UInt8>?, _ length: Int
) -> UInt32 {
  guard let engine = engine(pointer), let script, length > 0,
    let bootstrap = String(
      bytes: UnsafeBufferPointer(start: script, count: length), encoding: .utf8)
  else { return invalidArgument }
  return engine.start(bootstrap: bootstrap)
}

@_cdecl("zjsc_engine_submit")
public func zjscEngineSubmit(
  _ pointer: UnsafeMutableRawPointer?, _ evaluationID: UInt64,
  _ source: UnsafePointer<UInt8>?, _ length: Int
) -> UInt32 {
  guard let engine = engine(pointer), length >= 0, length == 0 || source != nil else {
    return invalidArgument
  }
  let data = length == 0 ? Data() : Data(bytes: source!, count: length)
  return engine.submit(evaluationID: evaluationID, source: data)
}

@_cdecl("zjsc_engine_next_event")
public func zjscEngineNextEvent(
  _ pointer: UnsafeMutableRawPointer?, _ output: UnsafeMutablePointer<UInt8>?,
  _ capacity: Int, _ required: UnsafeMutablePointer<Int>?
) -> UInt32 {
  guard let engine = engine(pointer), capacity >= 0, let required else { return invalidArgument }
  var size = 0
  let status = engine.nextEvent(output: output, capacity: capacity, required: &size)
  required.pointee = size
  return status
}

@_cdecl("zjsc_engine_complete")
public func zjscEngineComplete(
  _ pointer: UnsafeMutableRawPointer?, _ requestID: UInt64, _ kind: UInt32,
  _ objectID: UInt64, _ payload: UnsafePointer<UInt8>?, _ length: Int
) -> UInt32 {
  guard let engine = engine(pointer), requestID != 0, length >= 0, length == 0 || payload != nil
  else { return invalidArgument }
  let data = length == 0 ? Data() : Data(bytes: payload!, count: length)
  return engine.complete(requestID: requestID, kind: kind, objectID: objectID, payload: data)
}

@_cdecl("zjsc_engine_cancel")
public func zjscEngineCancel(_ pointer: UnsafeMutableRawPointer?, _ evaluationID: UInt64) -> UInt32
{
  engine(pointer)?.cancel(evaluationID: evaluationID) ?? invalidArgument
}

@_cdecl("zjsc_engine_microtask_checkpoint")
public func zjscEngineMicrotaskCheckpoint(_ pointer: UnsafeMutableRawPointer?) -> UInt32 {
  engine(pointer)?.microtaskCheckpoint() ?? invalidArgument
}

@_cdecl("zjsc_engine_shutdown")
public func zjscEngineShutdown(_ pointer: UnsafeMutableRawPointer?) -> UInt32 {
  engine(pointer)?.shutdown() ?? invalidArgument
}

@_cdecl("zjsc_engine_free")
public func zjscEngineFree(_ pointer: UnsafeMutableRawPointer?) {
  guard let pointer else { return }
  let engine = Unmanaged<JSCFFIEngine>.fromOpaque(pointer).takeRetainedValue()
  _ = engine.shutdown()
}
