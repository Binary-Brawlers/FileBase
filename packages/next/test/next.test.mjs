// Covers createFileBaseRoute: preset allow-list, authorize hook, JSON parsing,
// and mapping of upstream FileBaseError / unexpected failures to JSON responses.
import test, { afterEach } from "node:test";
import assert from "node:assert/strict";
import { createFileBaseRoute } from "../dist/index.js";

const session = { id: "s1", uploadUrl: "https://api.test/u/s1", token: "t" };
const originalFetch = globalThis.fetch;
afterEach(() => {
  globalThis.fetch = originalFetch;
});

/** Creates a route whose upstream gateway fetch is stubbed. */
function makeRoute(
  options = {},
  upstream = () => Response.json({ data: session }),
) {
  const calls = [];
  const route = createFileBaseRoute({
    apiKey: "fb_test_key",
    gatewayUrl: "https://api.test",
    fetch: async (url, init) => {
      calls.push({ url, init });
      return upstream(url, init);
    },
    ...options,
  });
  return { route, calls };
}

const post = (body, headers = { "content-type": "application/json" }) =>
  new Request("https://app.test/api/sign", {
    method: "POST",
    headers,
    body:
      body === undefined
        ? undefined
        : typeof body === "string"
          ? body
          : JSON.stringify(body),
  });

test("exchanges the API key for a session and returns it under data", async () => {
  const { route, calls } = makeRoute();
  const response = await route(post({ presetId: "pre", projectId: "p" }));
  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { data: session });
  assert.equal(calls[0].url, "https://api.test/uploads/sign");
  assert.equal(calls[0].init.headers.authorization, "Bearer fb_test_key");
  assert.deepEqual(JSON.parse(calls[0].init.body), {
    preset_id: "pre",
    project_id: "p",
  });
});

test("treats missing or malformed JSON bodies as an empty sign request", async () => {
  const { route, calls } = makeRoute();
  await route(post(undefined, {}));
  await route(post("{not json"));
  assert.equal(calls[0].init.body, "{}");
  assert.equal(calls[1].init.body, "{}");
});

test("allowedPresets rejects missing and unlisted presets with 403", async () => {
  const { route, calls } = makeRoute({ allowedPresets: ["avatars"] });
  for (const body of [{}, { preset: "other" }, { presetId: "other" }]) {
    const response = await route(post(body));
    assert.equal(response.status, 403);
    assert.deepEqual(await response.json(), {
      error: { message: "preset is not allowed", code: "request_failed" },
    });
  }
  assert.equal(calls.length, 0);
});

test("allowedPresets accepts either preset or presetId that is listed", async () => {
  const { route } = makeRoute({ allowedPresets: ["avatars"] });
  assert.equal((await route(post({ preset: "avatars" }))).status, 200);
  assert.equal((await route(post({ presetId: "avatars" }))).status, 200);
});

test("authorize returning false yields 401 without calling upstream", async () => {
  const { route, calls } = makeRoute({ authorize: async () => false });
  const response = await route(post({}));
  assert.equal(response.status, 401);
  assert.equal((await response.json()).error.message, "not authorized");
  assert.equal(calls.length, 0);
});

test("authorize receives the request and parsed body, and its result overrides client fields", async () => {
  let seen;
  const { route, calls } = makeRoute({
    authorize: (request, sign) => {
      seen = { url: request.url, sign };
      return { projectId: "forced" };
    },
  });
  await route(post({ projectId: "client-chosen", preset: "p" }));
  assert.equal(seen.url, "https://app.test/api/sign");
  assert.deepEqual(seen.sign, { projectId: "client-chosen", preset: "p" });
  assert.deepEqual(JSON.parse(calls[0].init.body), {
    project_id: "forced",
    preset: "p",
  });
});

test("authorize returning true or undefined leaves the body untouched", async () => {
  for (const result of [true, undefined]) {
    const { route, calls } = makeRoute({ authorize: () => result });
    await route(post({ preset: "p" }));
    assert.deepEqual(JSON.parse(calls[0].init.body), { preset: "p" });
  }
});

test("authorize runs after the allow-list check", async () => {
  let called = false;
  const { route } = makeRoute({
    allowedPresets: ["a"],
    authorize: () => {
      called = true;
    },
  });
  await route(post({ preset: "b" }));
  assert.equal(called, false);
});

test("upstream API errors keep their status and report the FileBaseError code", async () => {
  const { route } = makeRoute({}, () =>
    Response.json(
      { error: { code: "invalid_key", message: "bad key" } },
      { status: 401 },
    ),
  );
  const response = await route(post({}));
  assert.equal(response.status, 401);
  assert.deepEqual(await response.json(), {
    error: {
      message: "request to /uploads/sign failed",
      code: "upload_failed",
    },
  });
});

test("network failures to the gateway surface as 502 network_error", async () => {
  const { route } = makeRoute({}, () => {
    throw new Error("connect ECONNREFUSED");
  });
  const response = await route(post({}));
  assert.equal(response.status, 502);
  assert.equal((await response.json()).error.code, "network_error");
});

test("a throwing authorize hook propagates instead of being swallowed", async () => {
  const { route } = makeRoute({
    authorize: () => {
      throw new Error("session lookup failed");
    },
  });
  await assert.rejects(route(post({})), /session lookup failed/);
});

test("constructing a route with invalid options throws immediately", () => {
  assert.throws(
    () => createFileBaseRoute({ apiKey: "", gatewayUrl: "x" }),
    /apiKey/,
  );
});
