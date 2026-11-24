// Covers FileBaseClient: sign requests, direct upload (fetch and XHR progress),
// chunked/resumable upload, abort handling and error mapping.
import test, { afterEach } from "node:test";
import assert from "node:assert/strict";
import { FileBaseClient, FileBaseError } from "../dist/index.js";

const session = {
  id: "sess_1",
  projectId: "p1",
  presetId: "pre1",
  uploadUrl: "https://api.test/uploads/sess_1",
  token: "tok_abc",
  expiresAt: "2030-01-01T00:00:00Z",
};
const fileRecord = { id: "f1", originalName: "a.txt" };

const originalFetch = globalThis.fetch;
const originalXhr = globalThis.XMLHttpRequest;
afterEach(() => {
  globalThis.fetch = originalFetch;
  if (originalXhr === undefined) delete globalThis.XMLHttpRequest;
  else globalThis.XMLHttpRequest = originalXhr;
});

const json = (body, status = 200) =>
  new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });

/** Builds a client whose fetch records calls and answers via `respond(url, init, callIndex)`. */
function makeClient(respond, extra = {}) {
  const calls = [];
  const fetch = async (url, init) => {
    calls.push({ url, init });
    return respond(url, init, calls.length - 1);
  };
  const client = new FileBaseClient({
    signEndpoint: "/api/sign",
    fetch,
    ...extra,
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

test("constructor requires signEndpoint", () => {
  assert.throws(
    () => new FileBaseClient({ signEndpoint: "" }),
    (e) => e instanceof FileBaseError && e.code === "validation_error",
  );
});

test("createSession POSTs JSON with default headers and credentials", async () => {
  const { client, calls } = makeClient(() => json({ data: session }));
  const result = await client.createSession({
    preset: "avatars",
    expiresInSeconds: 60,
  });
  assert.deepEqual(result, session);
  assert.equal(calls[0].url, "/api/sign");
  assert.equal(calls[0].init.method, "POST");
  assert.equal(calls[0].init.headers["content-type"], "application/json");
  assert.equal(calls[0].init.credentials, "same-origin");
  assert.deepEqual(JSON.parse(calls[0].init.body), {
    preset: "avatars",
    expiresInSeconds: 60,
  });
});

test("createSession sends custom signHeaders and credentials", async () => {
  const { client, calls } = makeClient(() => json({ data: session }), {
    signHeaders: { "x-csrf": "1", accept: "text/plain" },
    signCredentials: "include",
  });
  await client.createSession();
  assert.equal(calls[0].init.headers["x-csrf"], "1");
  assert.equal(calls[0].init.headers.accept, "text/plain");
  assert.equal(calls[0].init.credentials, "include");
  assert.equal(calls[0].init.body, "{}");
});

test("createSession maps API error shape to sign_failed with status and details", async () => {
  const apiError = { error: { code: "forbidden", message: "nope" } };
  const { client } = makeClient(() => json(apiError, 403));
  const error = await rejection(client.createSession());
  assert.ok(error instanceof FileBaseError);
  assert.equal(error.code, "sign_failed");
  assert.equal(error.status, 403);
  assert.deepEqual(error.details, apiError);
});

test("createSession keeps non-JSON error bodies as raw text and empty bodies as null", async () => {
  const { client } = makeClient(
    () => new Response("gateway down", { status: 502 }),
  );
  assert.equal(
    (await rejection(client.createSession())).details,
    "gateway down",
  );
  const empty = makeClient(() => new Response(null, { status: 500 }));
  assert.equal((await rejection(empty.client.createSession())).details, null);
});

test("createSession rejects invalid session payloads", async () => {
  for (const body of [
    { data: { uploadUrl: "u" } },
    { data: { token: "t" } },
    {},
  ]) {
    const { client } = makeClient(() => json(body));
    const error = await rejection(client.createSession());
    assert.equal(error.code, "sign_failed");
    assert.match(error.message, /invalid session/);
  }
  const { client } = makeClient(
    () => new Response("not json", { status: 200 }),
  );
  assert.equal((await rejection(client.createSession())).details, null);
});

test("createSession wraps fetch failures as network_error with cause", async () => {
  const cause = new TypeError("offline");
  const { client } = makeClient(() => {
    throw cause;
  });
  const error = await rejection(client.createSession());
  assert.equal(error.code, "network_error");
  assert.equal(error.cause, cause);
});

test("createSession falls back to globalThis.fetch and errors when none exists", async () => {
  globalThis.fetch = async () => json({ data: session });
  const client = new FileBaseClient({ signEndpoint: "/s" });
  assert.deepEqual(await client.createSession(), session);

  globalThis.fetch = undefined;
  const error = await rejection(client.createSession());
  assert.equal(error.code, "network_error");
  assert.match(error.message, /fetch is not available/);
});

test("uploadToSession sends bearer token and multipart form via fetch", async () => {
  const { client, calls } = makeClient(() => json({ data: fileRecord }));
  const blob = new Blob(["hello"], { type: "text/plain" });
  const result = await client.uploadToSession(session, blob, {
    filename: "greeting.txt",
    fields: { folder: "docs" },
  });
  assert.deepEqual(result, fileRecord);
  assert.equal(calls[0].url, session.uploadUrl);
  assert.equal(calls[0].init.method, "POST");
  assert.equal(calls[0].init.headers.authorization, "Bearer tok_abc");
  const form = calls[0].init.body;
  assert.ok(form instanceof FormData);
  assert.equal(form.get("file").name, "greeting.txt");
  assert.equal(await form.get("file").text(), "hello");
  assert.equal(form.get("folder"), "docs");
});

test("uploadToSession guesses filename from blob type and honors contentType override", async () => {
  const { client, calls } = makeClient(() => json({ data: fileRecord }));
  await client.uploadToSession(session, new Blob(["x"], { type: "image/png" }));
  assert.equal(calls[0].init.body.get("file").name, "upload.png");

  await client.uploadToSession(session, new Blob(["x"]), {
    contentType: "text/csv",
  });
  assert.equal(calls[1].init.body.get("file").name, "upload.octet-stream");
  assert.equal(calls[1].init.body.get("file").type, "text/csv");
});

test("uploadToSession uses File name when given a File", async () => {
  const { client, calls } = makeClient(() => json({ data: fileRecord }));
  await client.uploadToSession(
    session,
    new File(["x"], "photo.jpg", { type: "image/jpeg" }),
  );
  assert.equal(calls[0].init.body.get("file").name, "photo.jpg");
});

test("uploadToSession maps non-2xx to upload_failed with status and API error details", async () => {
  const apiError = { error: { code: "file_too_large", message: "too big" } };
  const { client } = makeClient(() => json(apiError, 413));
  const error = await rejection(
    client.uploadToSession(session, new Blob(["x"])),
  );
  assert.equal(error.code, "upload_failed");
  assert.equal(error.status, 413);
  assert.deepEqual(error.details.error, apiError.error);
});

test("uploadToSession rejects 200 responses without data", async () => {
  const { client } = makeClient(() => json({ ok: true }));
  const error = await rejection(
    client.uploadToSession(session, new Blob(["x"])),
  );
  assert.equal(error.code, "upload_failed");
  assert.match(error.message, /invalid response/);
});

test("uploadToSession distinguishes network_error from aborted", async () => {
  const { client } = makeClient(() => {
    throw new Error("socket hang up");
  });
  assert.equal(
    (await rejection(client.uploadToSession(session, new Blob(["x"])))).code,
    "network_error",
  );

  const controller = new AbortController();
  const aborting = makeClient(() => {
    controller.abort();
    throw new DOMException("aborted", "AbortError");
  });
  const error = await rejection(
    aborting.client.uploadToSession(session, new Blob(["x"]), {
      signal: controller.signal,
    }),
  );
  assert.equal(error.code, "aborted");
});

test("uploadToSession passes the abort signal to fetch", async () => {
  const controller = new AbortController();
  const { client, calls } = makeClient(() => json({ data: fileRecord }));
  await client.uploadToSession(session, new Blob(["x"]), {
    signal: controller.signal,
  });
  assert.equal(calls[0].init.signal, controller.signal);
});

test("upload signs first, stripping upload-only options from the sign request", async () => {
  const { client, calls } = makeClient((url) =>
    url === "/api/sign" ? json({ data: session }) : json({ data: fileRecord }),
  );
  const result = await client.upload(new Blob(["x"]), {
    preset: "docs",
    projectId: "p1",
    filename: "x.txt",
    fields: { a: "b" },
  });
  assert.deepEqual(result, fileRecord);
  assert.deepEqual(JSON.parse(calls[0].init.body), {
    preset: "docs",
    projectId: "p1",
  });
  assert.equal(calls[1].url, session.uploadUrl);
  assert.equal(calls[1].init.body.get("a"), "b");
});

test("upload does not attempt the upload when signing fails", async () => {
  const { client, calls } = makeClient(() =>
    json({ error: { code: "x", message: "y" } }, 401),
  );
  await rejection(client.upload(new Blob(["x"])));
  assert.equal(calls.length, 1);
});

/** Minimal XMLHttpRequest stand-in that lets tests drive progress and completion. */
class FakeXhr {
  static last;
  upload = {};
  headers = {};
  status = 0;
  responseText = "";
  constructor() {
    FakeXhr.last = this;
  }
  open(method, url) {
    this.method = method;
    this.url = url;
  }
  setRequestHeader(name, value) {
    this.headers[name] = value;
  }
  send(body) {
    this.body = body;
  }
  abort() {
    this.onabort?.();
  }
  finish(status, text) {
    this.status = status;
    this.responseText = text;
    this.onload();
  }
}

test("uploadToSession uses XHR when onProgress is given and reports progress", async () => {
  globalThis.XMLHttpRequest = FakeXhr;
  const { client, calls } = makeClient(() => json({}));
  const events = [];
  const promise = client.uploadToSession(session, new Blob(["x"]), {
    onProgress: (p) => events.push(p),
  });
  const xhr = FakeXhr.last;
  assert.equal(xhr.method, "POST");
  assert.equal(xhr.url, session.uploadUrl);
  assert.equal(xhr.headers.authorization, "Bearer tok_abc");
  xhr.upload.onprogress({ loaded: 25, total: 100, lengthComputable: true });
  xhr.upload.onprogress({ loaded: 5, total: 0, lengthComputable: false });
  xhr.finish(200, JSON.stringify({ data: fileRecord }));
  assert.deepEqual(await promise, fileRecord);
  assert.deepEqual(events, [
    { loaded: 25, total: 100, fraction: 0.25 },
    { loaded: 5, total: 0, fraction: null },
  ]);
  assert.equal(calls.length, 0);
});

test("XHR upload maps error status, invalid body, network error and abort", async () => {
  globalThis.XMLHttpRequest = FakeXhr;
  const { client } = makeClient(() => json({}));
  const start = (extra = {}) =>
    client.uploadToSession(session, new Blob(["x"]), {
      onProgress() {},
      ...extra,
    });

  let promise = start();
  FakeXhr.last.finish(
    422,
    JSON.stringify({ error: { code: "bad", message: "m" } }),
  );
  let error = await rejection(promise);
  assert.equal(error.code, "upload_failed");
  assert.equal(error.status, 422);
  assert.equal(error.details.error.code, "bad");

  promise = start();
  FakeXhr.last.finish(500, "plain text");
  assert.equal((await rejection(promise)).details, "plain text");

  promise = start();
  FakeXhr.last.finish(200, "{}");
  assert.match((await rejection(promise)).message, /invalid response/);

  promise = start();
  FakeXhr.last.onerror();
  assert.equal((await rejection(promise)).code, "network_error");

  const controller = new AbortController();
  promise = start({ signal: controller.signal });
  controller.abort();
  assert.equal((await rejection(promise)).code, "aborted");

  const preAborted = new AbortController();
  preAborted.abort();
  assert.equal(
    (await rejection(start({ signal: preAborted.signal }))).code,
    "aborted",
  );
});

test("chunkStatus requests /chunks with bearer token and normalizes snake_case data", async () => {
  const { client, calls } = makeClient(() =>
    json({
      data: {
        session_id: "sess_1",
        received_chunks: [0, 2],
        received_bytes: "10",
        chunk_size: 65536,
        max_file_size: 1000,
        used: 0,
        expires_at: "2030",
      },
    }),
  );
  const status = await client.chunkStatus(session);
  assert.equal(calls[0].url, `${session.uploadUrl}/chunks`);
  assert.equal(calls[0].init.method, "GET");
  assert.equal(calls[0].init.headers.authorization, "Bearer tok_abc");
  assert.deepEqual(status, {
    sessionId: "sess_1",
    receivedChunks: [0, 2],
    receivedBytes: 10,
    chunkSize: 65536,
    maxFileSize: 1000,
    used: false,
    expiresAt: "2030",
  });
});

test("chunkStatus applies defaults for missing fields and rejects errors", async () => {
  const sparse = makeClient(() => json({ data: {} }));
  assert.deepEqual(await sparse.client.chunkStatus(session), {
    sessionId: "sess_1",
    receivedChunks: [],
    receivedBytes: 0,
    chunkSize: 0,
    maxFileSize: 0,
    used: false,
    expiresAt: "",
  });

  const failing = makeClient(() =>
    json({ error: { code: "expired", message: "m" } }, 410),
  );
  const error = await rejection(failing.client.chunkStatus(session));
  assert.equal(error.code, "upload_failed");
  assert.equal(error.status, 410);

  const invalid = makeClient(() => json({}));
  assert.match(
    (await rejection(invalid.client.chunkStatus(session))).message,
    /invalid response/,
  );
});

test("chunkStatus reports aborted vs network_error", async () => {
  const controller = new AbortController();
  controller.abort();
  const { client } = makeClient(() => {
    throw new Error("x");
  });
  assert.equal(
    (await rejection(client.chunkStatus(session, controller.signal))).code,
    "aborted",
  );
  assert.equal(
    (await rejection(client.chunkStatus(session))).code,
    "network_error",
  );
});

const KIB = 1024;
const chunkSize = 64 * KIB; // smallest size the client allows

function chunkedResponder({ received = [], statusFails = false } = {}) {
  return (url, init) => {
    if (url.endsWith("/chunks") && init.method === "GET") {
      return statusFails
        ? json({}, 500)
        : json({ data: { received_chunks: received } });
    }
    if (url.endsWith("/chunks")) return json({ data: {} });
    return json({ data: fileRecord });
  };
}

test("chunked upload splits the file, posts each chunk, then completes", async () => {
  const { client, calls } = makeClient(chunkedResponder());
  const file = new Blob([new Uint8Array(chunkSize * 2 + 10)], {
    type: "video/mp4",
  });
  const events = [];
  const result = await client.uploadChunkedToSession(session, file, {
    chunkSize,
    filename: "v.mp4",
    fields: { folder: "f" },
    onProgress: (p) => events.push(p),
  });
  assert.deepEqual(result, fileRecord);

  const posts = calls.filter(
    (c) => c.url.endsWith("/chunks") && c.init.method === "POST",
  );
  assert.equal(posts.length, 3);
  assert.deepEqual(
    posts.map((c) => c.init.body.get("chunk_index")),
    ["0", "1", "2"],
  );
  assert.deepEqual(
    posts.map((c) => c.init.body.get("chunk").size),
    [chunkSize, chunkSize, 10],
  );
  assert.equal(posts[0].init.body.get("chunk").name, "v.mp4.part0");
  assert.equal(posts[0].init.body.get("folder"), "f");
  assert.equal(posts[0].init.headers.authorization, "Bearer tok_abc");

  const complete = calls.at(-1);
  assert.equal(complete.url, `${session.uploadUrl}/complete`);
  assert.deepEqual(JSON.parse(complete.init.body), {
    filename: "v.mp4",
    contentType: "video/mp4",
  });
  assert.deepEqual(
    events.map((p) => p.loaded),
    [0, chunkSize, chunkSize * 2, chunkSize * 2 + 10],
  );
  assert.equal(events.at(-1).fraction, 1);
});

test("chunked upload resumes: skips chunks the server already has and counts them as loaded", async () => {
  const { client, calls } = makeClient(chunkedResponder({ received: [0, 1] }));
  const file = new Blob([new Uint8Array(chunkSize * 3)]);
  const events = [];
  await client.uploadChunkedToSession(session, file, {
    chunkSize,
    onProgress: (p) => events.push(p),
  });
  const posts = calls.filter(
    (c) => c.init.method === "POST" && c.url.endsWith("/chunks"),
  );
  assert.deepEqual(
    posts.map((c) => c.init.body.get("chunk_index")),
    ["2"],
  );
  assert.equal(events[0].loaded, chunkSize * 2);
});

test("chunked upload with resume:false does not query chunk status", async () => {
  const { client, calls } = makeClient(chunkedResponder({ received: [0] }));
  await client.uploadChunkedToSession(
    session,
    new Blob([new Uint8Array(chunkSize)]),
    {
      chunkSize,
      resume: false,
    },
  );
  assert.equal(calls.filter((c) => c.init.method === "GET").length, 0);
  assert.equal(calls.filter((c) => c.url.endsWith("/chunks")).length, 1);
});

test("chunked upload ignores a failing status lookup and uploads everything", async () => {
  const { client, calls } = makeClient(chunkedResponder({ statusFails: true }));
  await client.uploadChunkedToSession(
    session,
    new Blob([new Uint8Array(chunkSize)]),
    { chunkSize },
  );
  assert.equal(
    calls.filter((c) => c.init.method === "POST" && c.url.endsWith("/chunks"))
      .length,
    1,
  );
});

test("chunked upload clamps chunkSize to a 64 KiB minimum", async () => {
  const { client, calls } = makeClient(chunkedResponder());
  await client.uploadChunkedToSession(
    session,
    new Blob([new Uint8Array(chunkSize + 1)]),
    {
      chunkSize: 10,
      resume: false,
    },
  );
  assert.equal(
    calls.filter((c) => c.init.method === "POST" && c.url.endsWith("/chunks"))
      .length,
    2,
  );
});

test("chunked upload of an empty file sends one empty chunk and reports null fraction", async () => {
  const { client, calls } = makeClient(chunkedResponder());
  const events = [];
  await client.uploadChunkedToSession(session, new Blob([]), {
    resume: false,
    onProgress: (p) => events.push(p),
  });
  assert.equal(
    calls.filter((c) => c.init.method === "POST" && c.url.endsWith("/chunks"))
      .length,
    1,
  );
  assert.deepEqual(events[0], { loaded: 0, total: 0, fraction: null });
});

test("chunked upload stops with upload_failed naming the failing chunk", async () => {
  const { client, calls } = makeClient((url, init) => {
    if (url.endsWith("/chunks") && init.method === "POST") {
      const index = init.body.get("chunk_index");
      return index === "1"
        ? json({ error: { code: "disk_full", message: "m" } }, 507)
        : json({ data: {} });
    }
    return json({ data: { received_chunks: [] } });
  });
  const error = await rejection(
    client.uploadChunkedToSession(
      session,
      new Blob([new Uint8Array(chunkSize * 3)]),
      { chunkSize },
    ),
  );
  assert.equal(error.code, "upload_failed");
  assert.match(error.message, /chunk 1/);
  assert.equal(error.status, 507);
  assert.equal(calls.filter((c) => c.init.method === "POST").length, 2);
});

test("chunked upload maps completion errors and invalid completion payloads", async () => {
  const failing = makeClient((url, init) =>
    url.endsWith("/complete")
      ? json({ error: { code: "x", message: "m" } }, 409)
      : chunkedResponder()(url, init),
  );
  let error = await rejection(
    failing.client.uploadChunkedToSession(session, new Blob(["x"]), {
      resume: false,
    }),
  );
  assert.equal(error.status, 409);
  assert.match(error.message, /completion/);

  const invalid = makeClient((url, init) =>
    url.endsWith("/complete") ? json({}) : chunkedResponder()(url, init),
  );
  error = await rejection(
    invalid.client.uploadChunkedToSession(session, new Blob(["x"]), {
      resume: false,
    }),
  );
  assert.match(error.message, /invalid response/);
});

test("chunked upload maps network and abort errors on chunk POST", async () => {
  const network = makeClient((url, init) => {
    if (init.method === "POST") throw new Error("down");
    return json({ data: {} });
  });
  assert.equal(
    (
      await rejection(
        network.client.uploadChunkedToSession(session, new Blob(["x"]), {
          resume: false,
        }),
      )
    ).code,
    "network_error",
  );

  const controller = new AbortController();
  const aborting = makeClient(() => {
    controller.abort();
    throw new Error("aborted");
  });
  assert.equal(
    (
      await rejection(
        aborting.client.uploadChunkedToSession(session, new Blob(["x"]), {
          resume: false,
          signal: controller.signal,
        }),
      )
    ).code,
    "aborted",
  );
});

test("upload with strategy chunked and uploadChunked sign then use chunk endpoints", async () => {
  const respond = (url, init, i) =>
    url === "/api/sign"
      ? json({ data: session })
      : chunkedResponder()(url, init, i);
  for (const run of [
    (c) =>
      c.upload(new Blob(["x"]), {
        strategy: "chunked",
        preset: "p",
        resume: false,
      }),
    (c) => c.uploadChunked(new Blob(["x"]), { preset: "p", resume: false }),
  ]) {
    const { client, calls } = makeClient(respond);
    assert.deepEqual(await run(client), fileRecord);
    assert.deepEqual(JSON.parse(calls[0].init.body), { preset: "p" });
    assert.ok(calls.at(-1).url.endsWith("/complete"));
  }
});
