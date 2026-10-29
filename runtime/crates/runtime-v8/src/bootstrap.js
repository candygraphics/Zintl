(() => {
  "use strict";
  const evaluationDone = globalThis.__zintlEvaluationDone;
  const evaluationFailed = globalThis.__zintlEvaluationFailed;
  const invokeNative = globalThis.__zintlInvoke;
  const sleepNative = globalThis.__zintlSleep;
  const readFileNative = globalThis.__zintlReadFile;
  const mountOperationNative = globalThis.__zintlMountOperation;
  const consoleNative = globalThis.__zintlConsole;
  for (const name of [
    "__zintlEvaluationDone",
    "__zintlEvaluationFailed",
    "__zintlInvoke",
    "__zintlSleep",
    "__zintlReadFile",
    "__zintlMountOperation",
    "__zintlConsole",
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
  const fromCodePoint = String.fromCodePoint;
  const assign = Object.assign;
  const freeze = Object.freeze;
  const defineProperty = Object.defineProperty;

  const newPromise = (executor) => new PromiseConstructor(executor);
  const then = (promise, onFulfilled, onRejected) =>
    apply(promiseThen, promise, [onFulfilled, onRejected]);
  const error = (message, code) => assign(new ErrorConstructor(message), { code });
  const invoke = (name, input = new ByteArray()) =>
    then(
      newPromise((resolve, reject) => {
        if (typeof name !== "string" || !(input instanceof ByteArray)) {
          return reject(error("Invalid input", "InvalidRequest"));
        }
        invokeNative(name, arrayFrom(input), resolve, reject);
      }),
      (bytes) => new ByteArray(bytes),
    );
  const sleep = (ms) =>
    newPromise((resolve, reject) => sleepNative(ms, resolve, reject));

  const decodeUtf8 = (bytes) => {
    let output = "";
    let codePoints = [];
    const append = (value) => {
      codePoints.push(value);
      if (codePoints.length === 1024) {
        output += apply(fromCodePoint, undefined, codePoints);
        codePoints = [];
      }
    };
    const continuation = (value) => (value & 0xc0) === 0x80;
    for (let index = 0; index < bytes.length; ) {
      const first = bytes[index++];
      if (first <= 0x7f) {
        append(first);
        continue;
      }
      if (first >= 0xc2 && first <= 0xdf && index < bytes.length) {
        const second = bytes[index++];
        if (continuation(second)) {
          append(((first & 0x1f) << 6) | (second & 0x3f));
          continue;
        }
      } else if (first >= 0xe0 && first <= 0xef && index + 1 < bytes.length) {
        const second = bytes[index++];
        const third = bytes[index++];
        const validSecond =
          continuation(second) &&
          (first !== 0xe0 || second >= 0xa0) &&
          (first !== 0xed || second <= 0x9f);
        if (validSecond && continuation(third)) {
          append(((first & 0x0f) << 12) | ((second & 0x3f) << 6) | (third & 0x3f));
          continue;
        }
      } else if (first >= 0xf0 && first <= 0xf4 && index + 2 < bytes.length) {
        const second = bytes[index++];
        const third = bytes[index++];
        const fourth = bytes[index++];
        const validSecond =
          continuation(second) &&
          (first !== 0xf0 || second >= 0x90) &&
          (first !== 0xf4 || second <= 0x8f);
        if (validSecond && continuation(third) && continuation(fourth)) {
          append(
            ((first & 0x07) << 18) |
              ((second & 0x3f) << 12) |
              ((third & 0x3f) << 6) |
              (fourth & 0x3f),
          );
          continue;
        }
      }
      throw error("Invalid UTF-8", "InvalidData");
    }
    if (codePoints.length) output += apply(fromCodePoint, undefined, codePoints);
    return output;
  };

  const encodeUtf8 = (value) => {
    const output = [];
    for (const character of value) {
      let point = character.codePointAt(0);
      // Lone surrogates have no UTF-8 form; encode them as U+FFFD like TextEncoder.
      if (point >= 0xd800 && point <= 0xdfff) point = 0xfffd;
      if (point <= 0x7f) output.push(point);
      else if (point <= 0x7ff)
        output.push(0xc0 | (point >> 6), 0x80 | (point & 0x3f));
      else if (point <= 0xffff)
        output.push(
          0xe0 | (point >> 12),
          0x80 | ((point >> 6) & 0x3f),
          0x80 | (point & 0x3f),
        );
      else
        output.push(
          0xf0 | (point >> 18),
          0x80 | ((point >> 12) & 0x3f),
          0x80 | ((point >> 6) & 0x3f),
          0x80 | (point & 0x3f),
        );
    }
    return new ByteArray(output);
  };

  const readFile = (url, encoding) =>
    then(
      newPromise((resolve, reject) => {
        if (typeof url !== "string" || (encoding !== undefined && encoding !== "utf8")) {
          return reject(error("Invalid mount URI or encoding", "InvalidRequest"));
        }
        readFileNative(url, 4 * 1024 * 1024, resolve, reject);
      }),
      (bytes) => {
        const value = new ByteArray(bytes);
        return encoding === "utf8" ? decodeUtf8(value) : value;
      },
    );
  const mutateMount = (kind, first, second = null) =>
    newPromise((resolve, reject) => {
      if (typeof first !== "string") {
        return reject(error("Invalid mount URI", "InvalidRequest"));
      }
      mountOperationNative(kind, first, second, resolve, reject);
    });
  const rejected = (reason) => newPromise((_, reject) => reject(reason));
  const writeFile = (url, data) => {
    let bytes;
    if (typeof data === "string") bytes = encodeUtf8(data);
    else if (data instanceof ByteArray) bytes = data;
    else return rejected(error("Expected string or Uint8Array", "InvalidRequest"));
    return mutateMount(11, url, arrayFrom(bytes));
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
