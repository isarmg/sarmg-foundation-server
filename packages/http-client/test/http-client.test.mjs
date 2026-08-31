import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  ApiClientError,
  DEFAULT_MAX_RESPONSE_BYTES,
  isApiClientError,
  MAX_RESPONSE_BYTES,
  requestJson,
} from "../dist/index.js";

const BASE_URL = "https://console.example/app/";

test("declarations do not re-export contract types", () => {
  const declarations = readFileSync(
    new URL("../dist/index.d.ts", import.meta.url),
    "utf8",
  );
  assert.doesNotMatch(declarations, /export type\s*\{\s*ErrorEnvelope\s*\}/);
});

function jsonResponse(value, init = {}) {
  const headers = new Headers(init.headers);
  headers.set("content-type", "application/json; charset=utf-8");
  return new Response(JSON.stringify(value), { ...init, headers });
}

function requestAtOrigin(url, options = {}) {
  return requestJson(url, { baseUrl: BASE_URL, ...options });
}

test("relative and same-origin absolute requests are normalized before fetch", async () => {
  const captured = [];
  const fetchImpl = async (url, init) => {
    captured.push({ url, init });
    return jsonResponse({ ok: true });
  };

  const relativeValue = await requestAtOrigin("../api/items?limit=2", {
    method: "POST",
    body: "{}",
    headers: { "x-custom": "kept" },
    csrfToken: "csrf-token",
    fetchImpl,
  });
  const absoluteValue = await requestAtOrigin(
    new URL("https://console.example:443/api/session"),
    { fetchImpl },
  );

  assert.deepEqual(relativeValue, { ok: true });
  assert.deepEqual(absoluteValue, { ok: true });
  assert.equal(captured[0].url, "https://console.example/api/items?limit=2");
  assert.equal(captured[1].url, "https://console.example/api/session");
  assert.equal(captured[0].init.credentials, "same-origin");
  assert.equal(captured[0].init.redirect, "error");
  const headers = new Headers(captured[0].init.headers);
  assert.equal(headers.get("accept"), "application/json");
  assert.equal(headers.get("x-custom"), "kept");
  assert.equal(headers.get("x-csrf-token"), "csrf-token");

  await requestAtOrigin("/api/items", {
    csrfToken: "must-not-be-sent-on-get",
    fetchImpl: async (_url, init) => {
      assert.equal(new Headers(init.headers).has("x-csrf-token"), false);
      return jsonResponse({ ok: true });
    },
  });
});

test("URL policy fails closed before network I/O", async () => {
  let calls = 0;
  const fetchImpl = async () => {
    calls += 1;
    return jsonResponse({ ok: true });
  };
  const invalidCases = [
    { url: "https://attacker.example/api", baseUrl: BASE_URL },
    { url: "//attacker.example/api", baseUrl: BASE_URL },
    { url: "http://console.example/api", baseUrl: BASE_URL },
    { url: "https://console.example:444/api", baseUrl: BASE_URL },
    { url: "https://console.example.evil/api", baseUrl: BASE_URL },
    { url: "https://user:secret@console.example/api", baseUrl: BASE_URL },
    { url: "data:application/json,%7B%7D", baseUrl: BASE_URL },
    { url: "javascript:alert(1)", baseUrl: BASE_URL },
    { url: "/api/\nitems", baseUrl: BASE_URL },
    { url: "", baseUrl: BASE_URL },
    { url: "/api/items", baseUrl: "https://user:secret@console.example/" },
    { url: "/api/items", baseUrl: "file:///tmp/index.html" },
    { url: "/api/items", baseUrl: "/relative-base" },
  ];
  for (const { url, baseUrl } of invalidCases) {
    await assert.rejects(
      requestJson(url, {
        baseUrl,
        method: "POST",
        csrfToken: "must-never-leak",
        fetchImpl,
      }),
      TypeError,
      `accepted ${url}`,
    );
  }
  await assert.rejects(
    requestAtOrigin("/api/items", { redirect: "follow", fetchImpl }),
    TypeError,
  );
  if (globalThis.location === undefined) {
    await assert.rejects(requestJson("/api/items", { fetchImpl }), TypeError);
  }
  assert.equal(calls, 0);
});

test("204, 205 and HEAD responses return undefined without a content type", async () => {
  for (const { method, status } of [
    { method: "POST", status: 204 },
    { method: "POST", status: 205 },
    { method: "HEAD", status: 200 },
  ]) {
    const value = await requestAtOrigin("/api/logout", {
      method,
      fetchImpl: async () => new Response(null, { status }),
    });
    assert.equal(value, undefined);
  }
});

test("error envelopes preserve status, branching fields, request ID and Retry-After", async () => {
  await assert.rejects(
    requestAtOrigin("/api/items", {
      fetchImpl: async () =>
        jsonResponse(
          {
            code: "too_many_requests",
            message: "try later",
            request_id: "body-request-id",
            retryable: true,
            details: { budget: "account" },
          },
          {
            status: 429,
            headers: {
              "retry-after": "7",
              "x-request-id": "header-request-id",
            },
          },
        ),
    }),
    (error) => {
      assert.equal(isApiClientError(error), true);
      assert.equal(error.status, 429);
      assert.equal(error.code, "too_many_requests");
      assert.equal(error.requestId, "body-request-id");
      assert.equal(error.retryable, true);
      assert.equal(error.retryAfterSeconds, 7);
      assert.deepEqual(error.details, { budget: "account" });
      return true;
    },
  );
});

test("request IDs are bounded, sanitized and use body-over-header precedence", async () => {
  await assert.rejects(
    requestAtOrigin("/api/missing-id", {
      fetchImpl: async () =>
        jsonResponse(
          { code: "not_found", message: "missing", retryable: false },
          { status: 404, headers: { "x-request-id": "header-request-id" } },
        ),
    }),
    (error) => error.requestId === "header-request-id",
  );

  await assert.rejects(
    requestAtOrigin("/api/invalid-header-id", {
      fetchImpl: async () =>
        jsonResponse(
          { code: "not_found", message: "missing", retryable: false },
          { status: 404, headers: { "x-request-id": "a".repeat(129) } },
        ),
    }),
    (error) => error.requestId === undefined,
  );

  await assert.rejects(
    requestAtOrigin("/api/invalid-body-id", {
      fetchImpl: async () =>
        jsonResponse(
          {
            code: "not_found",
            message: "missing",
            request_id: "not a safe id",
            retryable: false,
          },
          { status: 404, headers: { "x-request-id": "header-request-id" } },
        ),
    }),
    (error) =>
      error.code === "invalid_error_response" &&
      error.requestId === "header-request-id",
  );
});

test("Retry-After accepts bounded delta-seconds and HTTP dates", async () => {
  const cases = [
    { value: "86401", expected: 86400 },
    { value: "Wed, 21 Oct 2015 07:28:00 GMT", expected: 0 },
    { value: "1.5", expected: undefined },
    { value: "999999999999999999999999999999", expected: undefined },
  ];
  for (const { value, expected } of cases) {
    await assert.rejects(
      requestAtOrigin("/api/retry", {
        fetchImpl: async () =>
          jsonResponse(
            { code: "service_unavailable", message: "retry", retryable: true },
            { status: 503, headers: { "retry-after": value } },
          ),
      }),
      (error) => error.retryAfterSeconds === expected,
      `unexpected Retry-After result for ${value}`,
    );
  }
});

test("401 awaits session invalidation once without replacing the API error", async () => {
  let invalidations = 0;
  let cleanupFinished = false;
  await assert.rejects(
    requestAtOrigin("/api/session", {
      fetchImpl: async () =>
        jsonResponse(
          { code: "unauthorized", message: "sign in", retryable: false },
          { status: 401 },
        ),
      onUnauthorized: async (error) => {
        invalidations += 1;
        assert.equal(error.code, "unauthorized");
        await new Promise((resolve) => setTimeout(resolve, 10));
        cleanupFinished = true;
        throw new Error("local cleanup failed");
      },
    }),
    (error) =>
      cleanupFinished &&
      error instanceof ApiClientError &&
      error.code === "unauthorized",
  );
  assert.equal(invalidations, 1);
  assert.equal(cleanupFinished, true);
});

test("invalid error responses never expose arbitrary response text", async () => {
  await assert.rejects(
    requestAtOrigin("/api/items", {
      fetchImpl: async () =>
        new Response("database password=do-not-leak", {
          status: 500,
          headers: { "content-type": "text/plain", "x-request-id": "request-2" },
        }),
    }),
    (error) => {
      assert.equal(error.code, "invalid_error_response");
      assert.equal(error.requestId, "request-2");
      assert.equal(error.message.includes("do-not-leak"), false);
      return true;
    },
  );

  await assert.rejects(
    requestAtOrigin("/api/unknown-error-field", {
      fetchImpl: async () =>
        jsonResponse(
          {
            code: "internal_error",
            message: "failed",
            retryable: false,
            debug: "must not be accepted",
          },
          { status: 500 },
        ),
    }),
    (error) => error.code === "invalid_error_response",
  );
});

test("success responses require JSON and count bytes at exact boundaries", async () => {
  await assert.rejects(
    requestAtOrigin("/api/items", {
      fetchImpl: async () =>
        new Response("ok", { headers: { "content-type": "text/plain" } }),
    }),
    (error) => error.code === "invalid_content_type",
  );

  const body = '{"ok":true}';
  const byteLength = new TextEncoder().encode(body).byteLength;
  const exact = await requestAtOrigin("/api/items", {
    maxResponseBytes: byteLength,
    fetchImpl: async () =>
      new Response(body, { headers: { "content-type": "application/json" } }),
  });
  assert.deepEqual(exact, { ok: true });
  await assert.rejects(
    requestAtOrigin("/api/items", {
      maxResponseBytes: byteLength - 1,
      fetchImpl: async () =>
        new Response(body, { headers: { "content-type": "application/json" } }),
    }),
    (error) => error.code === "response_too_large",
  );

  const unicodeBody = '"é"';
  assert.ok(new TextEncoder().encode(unicodeBody).byteLength > unicodeBody.length);
  await assert.rejects(
    requestAtOrigin("/api/unicode", {
      maxResponseBytes: unicodeBody.length,
      fetchImpl: async () =>
        new Response(unicodeBody, { headers: { "content-type": "application/json" } }),
    }),
    (error) => error.code === "response_too_large",
  );

  const problem = await requestAtOrigin("/api/items", {
    fetchImpl: async () =>
      new Response('{"ok":true}', {
        headers: { "content-type": "application/problem+json" },
      }),
  });
  assert.deepEqual(problem, { ok: true });
});

test("declared and streaming size violations cancel bodies and remain authoritative", async () => {
  let declaredCancelled = false;
  const declaredStream = new ReadableStream({
    pull(controller) {
      controller.enqueue(new TextEncoder().encode("{}"));
      controller.close();
    },
    cancel() {
      declaredCancelled = true;
    },
  });
  await assert.rejects(
    requestAtOrigin("/api/declared-large", {
      maxResponseBytes: 4,
      fetchImpl: async () =>
        new Response(declaredStream, {
          headers: {
            "content-type": "application/json",
            "content-length": "5",
          },
        }),
    }),
    (error) => error.code === "response_too_large",
  );
  assert.equal(declaredCancelled, true);

  let streamingCancelled = false;
  const streamingBody = new ReadableStream({
    start(controller) {
      controller.enqueue(new Uint8Array([123, 34, 97]));
      controller.enqueue(new Uint8Array([34, 58, 49, 125]));
    },
    cancel() {
      streamingCancelled = true;
    },
  });
  await assert.rejects(
    requestAtOrigin("/api/streaming-large", {
      maxResponseBytes: 4,
      fetchImpl: async () =>
        new Response(streamingBody, { headers: { "content-type": "application/json" } }),
    }),
    (error) => error.code === "response_too_large",
  );
  assert.equal(streamingCancelled, true);

  await assert.rejects(
    requestAtOrigin("/api/error-too-large", {
      fetchImpl: async () =>
        jsonResponse(
          {
            code: "internal_error",
            message: "x".repeat(64 * 1024),
            retryable: false,
          },
          { status: 500, headers: { "x-request-id": "request-large" } },
        ),
    }),
    (error) =>
      error.code === "response_too_large" &&
      error.status === 500 &&
      error.requestId === "request-large",
  );
});

test("malformed JSON and malformed UTF-8 produce typed failures", async () => {
  await assert.rejects(
    requestAtOrigin("/api/bad-json", {
      fetchImpl: async () =>
        new Response("{", { headers: { "content-type": "application/json" } }),
    }),
    (error) => error.code === "invalid_json_response",
  );
  await assert.rejects(
    requestAtOrigin("/api/bad-utf8", {
      fetchImpl: async () =>
        new Response(new Uint8Array([0xff]), {
          headers: { "content-type": "application/json" },
        }),
    }),
    (error) => error.code === "invalid_json_response",
  );
});

test("timeout and caller cancellation have distinct first-wins errors", async () => {
  const rejectAfterAbort = (delayMs) => async (_url, init) =>
    new Promise((_resolve, reject) => {
      const rejectLater = () => setTimeout(() => reject(init.signal.reason), delayMs);
      if (init.signal.aborted) rejectLater();
      else init.signal.addEventListener("abort", rejectLater, { once: true });
    });

  await assert.rejects(
    requestAtOrigin("/api/slow", {
      timeoutMs: 10,
      fetchImpl: rejectAfterAbort(20),
    }),
    (error) => error.code === "request_timeout" && error.retryable === true,
  );

  const alreadyCancelled = new AbortController();
  alreadyCancelled.abort(new DOMException("cancelled", "AbortError"));
  await assert.rejects(
    requestAtOrigin("/api/cancelled", {
      signal: alreadyCancelled.signal,
      timeoutMs: 5,
      fetchImpl: rejectAfterAbort(20),
    }),
    (error) => error.code === "request_aborted" && error.retryable === false,
  );

  const callerWins = new AbortController();
  const callerRequest = requestAtOrigin("/api/caller-wins", {
    signal: callerWins.signal,
    timeoutMs: 10,
    fetchImpl: rejectAfterAbort(20),
  });
  callerWins.abort(new DOMException("cancelled", "AbortError"));
  await assert.rejects(
    callerRequest,
    (error) => error.code === "request_aborted",
  );
});

test("budget and header inputs are checked at both boundaries before fetch", async () => {
  let calls = 0;
  const fetchImpl = async () => {
    calls += 1;
    return jsonResponse({ ok: true });
  };
  for (const options of [
    { timeoutMs: 0 },
    { timeoutMs: 120_001 },
    { timeoutMs: 1.5 },
    { maxResponseBytes: 0 },
    { maxResponseBytes: MAX_RESPONSE_BYTES + 1 },
    { maxResponseBytes: 1.5 },
    { method: "POST", csrfToken: "bad\nvalue" },
    { method: "POST", csrfToken: "" },
    { method: "POST", csrfToken: 42 },
    { method: 42 },
  ]) {
    await assert.rejects(
      requestAtOrigin("/api/items", { ...options, fetchImpl }),
      (error) => error instanceof TypeError || error instanceof RangeError,
    );
  }
  assert.equal(calls, 0);

  await requestAtOrigin("/api/boundaries", {
    timeoutMs: 120_000,
    maxResponseBytes: MAX_RESPONSE_BYTES,
    fetchImpl,
  });
  await requestAtOrigin("/api/default-budget", {
    maxResponseBytes: DEFAULT_MAX_RESPONSE_BYTES,
    fetchImpl,
  });
  assert.equal(calls, 2);
});
