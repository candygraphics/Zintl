(() => {
  "use strict";
  const evaluationDone = globalThis.__zintlEvaluationDone;
  const evaluationFailed = globalThis.__zintlEvaluationFailed;
  const invokeNative = globalThis.__zintlInvoke;
  const sleepNative = globalThis.__zintlSleep;
  const readFileNative = globalThis.__zintlReadFile;
  const mountOperationNative = globalThis.__zintlMountOperation;
  const consoleNative = globalThis.__zintlConsole;
  const encodeUtf8 = globalThis.__zintlEncodeUtf8;
  const decodeUtf8Native = globalThis.__zintlDecodeUtf8;
  for (const name of [
    "__zintlEvaluationDone",
    "__zintlEvaluationFailed",
    "__zintlInvoke",
    "__zintlSleep",
    "__zintlReadFile",
    "__zintlMountOperation",
    "__zintlConsole",
    "__zintlEncodeUtf8",
    "__zintlDecodeUtf8",
  ]) delete globalThis[name];

  // Captured intrinsics keep the bridge working after user code replaces globals.
  const PromiseConstructor = Promise;
  const promiseThen = PromiseConstructor.prototype.then;
  const apply = Reflect.apply;
  const indirectEval = eval;
  const stringify = JSON.stringify;
  const arrayFrom = Array.from;
  const ByteArray = Uint8Array;
  const ErrorConstructor = Error;
  const toString = String;
  const assign = Object.assign;
  const freeze = Object.freeze;
  const defineProperty = Object.defineProperty;

  const MAXIMUM_READ_BYTES = 4 * 1024 * 1024;

  const newPromise = (executor) => new PromiseConstructor(executor);
  const then = (promise, onFulfilled, onRejected) =>
    apply(promiseThen, promise, [onFulfilled, onRejected]);
  const rejected = (reason) => newPromise((_, reject) => reject(reason));
  const error = (message, code) => assign(new ErrorConstructor(message), { code });

  const invoke = (name, input = new ByteArray()) =>
    then(
      newPromise((resolve, reject) => {
        if (typeof name !== "string" || !(input instanceof ByteArray)) {
          return reject(error("Invalid input", "InvalidRequest"));
        }
        invokeNative(name, input, resolve, reject);
      }),
      // A unit completion still resolves `invoke` with bytes.
      (value) => (value === undefined ? new ByteArray() : value),
    );
  const sleep = (ms) =>
    newPromise((resolve, reject) => sleepNative(ms, resolve, reject));

  const decodeUtf8 = (bytes) => {
    const text = decodeUtf8Native(bytes);
    if (text === undefined) throw error("Invalid UTF-8", "InvalidData");
    return text;
  };
  const readFile = (url, encoding) =>
    then(
      newPromise((resolve, reject) => {
        if (typeof url !== "string" || (encoding !== undefined && encoding !== "utf8")) {
          return reject(error("Invalid mount URI or encoding", "InvalidRequest"));
        }
        readFileNative(url, MAXIMUM_READ_BYTES, resolve, reject);
      }),
      (bytes) => (encoding === "utf8" ? decodeUtf8(bytes) : bytes),
    );
  const mutateMount = (kind, url, operand = null) =>
    newPromise((resolve, reject) => {
      if (typeof url !== "string") {
        return reject(error("Invalid mount URI", "InvalidRequest"));
      }
      mountOperationNative(kind, url, operand, resolve, reject);
    });
  const writeFile = (url, data) => {
    if (typeof data === "string") return mutateMount(11, url, encodeUtf8(data));
    if (data instanceof ByteArray) return mutateMount(11, url, data);
    return rejected(error("Expected string or Uint8Array", "InvalidRequest"));
  };
  const mkdir = (url) => mutateMount(12, url);
  const removeFile = (url) => mutateMount(13, url);
  const removeDirectory = (url) => mutateMount(14, url);
  const rename = (from, to) => {
    if (typeof to !== "string") {
      return rejected(error("Invalid mount URI", "InvalidRequest"));
    }
    return mutateMount(15, from, to);
  };
  defineProperty(globalThis, "Zintl", {
    value: freeze({
      invoke,
      sleep,
      readFile,
      writeFile,
      mkdir,
      removeFile,
      removeDirectory,
      rename,
    }),
    configurable: false,
  });

  const formatConsoleValue = (value) => {
    if (typeof value === "string") return value;
    try {
      const encoded = stringify(value);
      if (encoded !== undefined) return encoded;
    } catch (_) {}
    try {
      return toString(value);
    } catch (_) {
      return "<unprintable>";
    }
  };
  const emitConsole = (...values) =>
    consoleNative(values.map(formatConsoleValue).join(" "));
  defineProperty(globalThis, "console", {
    value: freeze({
      debug: emitConsole,
      log: emitConsole,
      info: emitConsole,
      warn: emitConsole,
      error: emitConsole,
    }),
    configurable: false,
  });

  const encode = (value) =>
    stringify(
      value instanceof ByteArray
        ? { type: "bytes", value: arrayFrom(value) }
        : { type: "value", value: value === undefined ? null : value },
    );
  // Reads `name` and `message` without letting a hostile thrown value escape.
  const describe = (exception) => {
    try {
      if (
        exception !== null &&
        (typeof exception === "object" || typeof exception === "function")
      ) {
        const name = exception.name;
        const message = exception.message;
        return {
          name: typeof name === "string" ? name : "Error",
          message: typeof message === "string" ? message : toString(exception),
        };
      }
      return { name: "Error", message: toString(exception) };
    } catch (_) {
      return { name: "Error", message: "Uncaught exception" };
    }
  };
  const fail = (token, exception) => {
    const { name, message } = describe(exception);
    evaluationFailed(token, name, message);
  };
  const succeed = (token, value) => {
    let encoded;
    try {
      encoded = encode(value);
    } catch (exception) {
      fail(token, exception);
      return;
    }
    evaluationDone(token, encoded);
  };
  // `token` is a backend-private settlement key, not the public evaluation ID.
  const evaluate = (token, source) => {
    let result;
    try {
      result = indirectEval(source);
    } catch (exception) {
      fail(token, exception);
      return;
    }
    then(
      newPromise((resolve) => resolve(result)),
      (value) => succeed(token, value),
      (exception) => fail(token, exception),
    );
  };
  return freeze({ evaluate });
})()
