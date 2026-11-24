// Covers the server-side FileBase client: API key auth, request building,
// snake_case conversion, file CRUD, direct upload (plain and progress stream) and error mapping.
import test, { afterEach } from "node:test";
import assert from "node:assert/strict";
import { FileBase, FileBaseError } from "../dist/index.js";

const originalFetch = globalThis.fetch;
afterEach(() => {
  globalThis.fetch = originalFetch;
});

const json = (body, status = 200) =>
  new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });

function makeClient(respond, options = {}) {
  const calls = [];
  const fetch = async (url, init) => {
    calls.push({ url, init });
    return respond(url, init);
  };
  const client = new FileBase({
    gatewayUrl: "https://api.test/",
    apiKey: "fb_test_key",
    fetch,
    ...options,
  });
  return { client, calls };
}

async function rejection(promise) {
  try {
    await promise;
  } catch (error) {
    return error;
  }
  assert.fail("expected promise to reject");
}

test("constructor validates apiKey, gatewayUrl and fetch availability", () => {
  assert.throws(
    () => new FileBase({ gatewayUrl: "x", apiKey: "" }),
    (e) => e.code === "validation_error" && /apiKey/.test(e.message),
  );
  assert.throws(
    () => new FileBase({ gatewayUrl: "", apiKey: "k" }),
    (e) => e.code === "validation_error" && /gatewayUrl/.test(e.message),
  );
  globalThis.fetch = undefined;
  assert.throws(
    () => new FileBase({ gatewayUrl: "x", apiKey: "k" }),
    (e) => e.code === "network_error",
  );
});

test("constructor strips trailing slashes from gatewayUrl and uses globalThis.fetch by default", async () => {
  let requested;
  globalThis.fetch = async (url) => {
    requested = url;
    return json({ data: [] });
  };
  const client = new FileBase({
    gatewayUrl: "https://api.test///",
    apiKey: "k",
  });
  assert.equal(client.gatewayUrl, "https://api.test");
  await client.listFiles();
  assert.equal(requested, "https://api.test/files");
});

test("every request carries bearer and x-api-key auth headers", async () => {
  const { client, calls } = makeClient(() => json({ data: [] }));
  await client.listFiles();
  assert.equal(calls[0].init.headers.authorization, "Bearer fb_test_key");
  assert.equal(calls[0].init.headers["x-api-key"], "fb_test_key");
  assert.equal(calls[0].init.headers.accept, "application/json");
  assert.equal(calls[0].init.method, "GET");
});

test("createUploadSession posts snake_case JSON and drops undefined fields", async () => {
  const session = { id: "s", uploadUrl: "u", token: "t" };
  const { client, calls } = makeClient(() => json({ data: session }));
  const result = await client.createUploadSession({
    presetId: "pre",
    projectId: "p",
    expiresInSeconds: 30,
    preset: undefined,
  });
  assert.deepEqual(result, session);
  assert.equal(calls[0].url, "https://api.test/uploads/sign");
  assert.equal(calls[0].init.method, "POST");
  assert.equal(calls[0].init.headers["content-type"], "application/json");
  assert.deepEqual(JSON.parse(calls[0].init.body), {
    preset_id: "pre",
    project_id: "p",
    expires_in_seconds: 30,
  });
});

test("createUploadSession with no options sends an empty JSON object", async () => {
  const { client, calls } = makeClient(() => json({ data: {} }));
  await client.createUploadSession();
  assert.equal(calls[0].init.body, "{}");
});

test("listFiles builds a snake_case query string and skips empty values", async () => {
  const { client, calls } = makeClient(() => json({ data: [{ id: "1" }] }));
  const files = await client.listFiles({
    projectId: "p 1",
    mimeType: "image/png",
    search: "",
    from: "2024-01-01",
  });
  assert.deepEqual(files, [{ id: "1" }]);
  const url = new URL(calls[0].url);
  assert.equal(url.pathname, "/files");
  assert.equal(url.searchParams.get("project_id"), "p 1");
  assert.equal(url.searchParams.get("mime_type"), "image/png");
  assert.equal(url.searchParams.get("from"), "2024-01-01");
  assert.equal(url.searchParams.has("search"), false);
});

test("getFile and deleteFile URL-encode the file id", async () => {
  const { client, calls } = makeClient((url, init) =>
    init.method === "DELETE"
      ? new Response(null, { status: 204 })
      : json({ data: { id: "a/b" } }),
  );
  assert.deepEqual(await client.getFile("a/b?x"), { id: "a/b" });
  assert.equal(calls[0].url, "https://api.test/files/a%2Fb%3Fx");
  assert.equal(await client.deleteFile("a/b"), undefined);
  assert.equal(calls[1].url, "https://api.test/files/a%2Fb");
  assert.equal(calls[1].init.method, "DELETE");
});

test("API error responses map to upload_failed carrying status and the { error } body", async () => {
  const apiError = { error: { code: "not_found", message: "no such file" } };
  const { client } = makeClient(() => json(apiError, 404));
  const error = await rejection(client.getFile("x"));
  assert.ok(error instanceof FileBaseError);
  assert.equal(error.code, "upload_failed");
  assert.equal(error.status, 404);
  assert.deepEqual(error.details, apiError);
  assert.equal((await rejection(client.deleteFile("x"))).status, 404);
});

test("non-JSON and empty error bodies are preserved as text / null", async () => {
  assert.equal(
    (
      await rejection(
        makeClient(() => new Response("oops", { status: 500 })).client.getFile(
          "x",
        ),
      )
    ).details,
    "oops",
  );
  assert.equal(
    (
      await rejection(
        makeClient(() => new Response(null, { status: 500 })).client.getFile(
          "x",
        ),
      )
    ).details,
    null,
  );
});

test("success responses without a data field are rejected", async () => {
  const { client } = makeClient(() => json({ ok: true }));
  const error = await rejection(client.getFile("x"));
  assert.equal(error.code, "upload_failed");
  assert.match(error.message, /no data field/);
  assert.equal(
    (await rejection(makeClient(() => new Response("nope")).client.listFiles()))
      .code,
    "upload_failed",
  );
});

test("fetch failures become network_error, or aborted when the signal fired", async () => {
  const cause = new Error("dns");
  const { client } = makeClient(() => {
    throw cause;
  });
  const error = await rejection(client.getFile("x"));
  assert.equal(error.code, "network_error");
  assert.equal(error.cause, cause);

  const controller = new AbortController();
  const aborting = makeClient(() => {
    controller.abort();
    throw new Error("abort");
  });
  const abortError = await rejection(
    aborting.client.uploadFile(new Uint8Array([1]), {
      signal: controller.signal,
    }),
  );
  assert.equal(abortError.code, "aborted");
});

test("uploadFile sends multipart FormData with preset fields", async () => {
  const record = { id: "f1" };
  const { client, calls } = makeClient(() => json({ data: record }));
  const result = await client.uploadFile(
    new Blob(["abc"], { type: "text/plain" }),
    {
      filename: "a.txt",
      preset: "docs",
      presetId: "pid",
      projectId: "proj",
    },
  );
  assert.deepEqual(result, record);
  assert.equal(calls[0].url, "https://api.test/uploads");
  assert.equal(calls[0].init.method, "POST");
  const form = calls[0].init.body;
  assert.ok(form instanceof FormData);
  assert.equal(form.get("file").name, "a.txt");
  assert.equal(await form.get("file").text(), "abc");
  assert.equal(form.get("preset"), "docs");
  assert.equal(form.get("preset_id"), "pid");
  assert.equal(form.get("project_id"), "proj");
  assert.equal(calls[0].init.headers["content-type"], undefined);
});

test("uploadFile accepts Uint8Array and ArrayBuffer, defaulting name and content type", async () => {
  const { client, calls } = makeClient(() => json({ data: {} }));
  await client.uploadFile(new Uint8Array([1, 2, 3]));
  const first = calls[0].init.body.get("file");
  assert.equal(first.name, "upload.bin");
  assert.equal(first.type, "application/octet-stream");
  assert.equal(first.size, 3);

  await client.uploadFile(new Uint8Array([1, 2]).buffer, {
    contentType: "image/png",
  });
  assert.equal(calls[1].init.body.get("file").type, "image/png");
  assert.equal(calls[1].init.body.get("file").size, 2);
});

test("uploadFile with a Uint8Array view copies only the viewed bytes", async () => {
  const { client, calls } = makeClient(() => json({ data: {} }));
  const view = new Uint8Array([9, 9, 1, 2, 9]).subarray(2, 4);
  await client.uploadFile(view);
  assert.deepEqual(
    [...new Uint8Array(await calls[0].init.body.get("file").arrayBuffer())],
    [1, 2],
  );
});

test("uploadFile with onProgress streams a multipart body and reports progress", async () => {
  let seenBody;
  const { client, calls } = makeClient(async (url, init) => {
    // Drain the stream like a real fetch would so progress callbacks fire.
    seenBody = Buffer.from(await new Response(init.body).arrayBuffer());
    return json({ data: { id: "f" } });
  });
  const payload = new Uint8Array(150 * 1024).fill(65);
  const events = [];
  await client.uploadFile(payload, {
    filename: 'we"ird.bin',
    preset: "p",
    onProgress: (p) => events.push(p),
  });
  const init = calls[0].init;
  assert.equal(init.duplex, "half");
  assert.match(
    init.headers["content-type"],
    /^multipart\/form-data; boundary=----FileBaseBoundary/,
  );
  assert.equal(Number(init.headers["content-length"]), seenBody.byteLength);
  const text = seenBody.toString("latin1");
  assert.match(text, /name="preset"\r\n\r\np\r\n/);
  assert.match(text, /filename="we%22ird\.bin"/);
  assert.ok(events.length >= 3);
  assert.equal(events.at(-1).loaded, events.at(-1).total);
  assert.equal(events.at(-1).fraction, 1);
  assert.ok(events.every((e, i) => i === 0 || e.loaded > events[i - 1].loaded));
  assert.equal(events[0].total, seenBody.byteLength);
});
