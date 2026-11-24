// Covers the shared package: the FileBaseError class used by every SDK package.
import test from "node:test";
import assert from "node:assert/strict";
import { FileBaseError } from "../dist/index.js";

test("FileBaseError is an Error subclass with name and code", () => {
  const error = new FileBaseError("sign_failed", "boom");
  assert.ok(error instanceof Error);
  assert.equal(error.name, "FileBaseError");
  assert.equal(error.code, "sign_failed");
  assert.equal(error.message, "boom");
});

test("FileBaseError keeps status and details", () => {
  const error = new FileBaseError("upload_failed", "bad", {
    status: 413,
    details: { error: { code: "too_large" } },
  });
  assert.equal(error.status, 413);
  assert.deepEqual(error.details, { error: { code: "too_large" } });
});

test("FileBaseError defaults status, details and cause to undefined", () => {
  const error = new FileBaseError("unknown", "x");
  assert.equal(error.status, undefined);
  assert.equal(error.details, undefined);
  assert.equal("cause" in error, false);
});

test("FileBaseError stores cause when provided, including falsy values", () => {
  const cause = new Error("root");
  assert.equal(new FileBaseError("network_error", "x", { cause }).cause, cause);
  assert.equal(new FileBaseError("network_error", "x", { cause: 0 }).cause, 0);
});
