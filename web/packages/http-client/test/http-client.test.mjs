import assert from "node:assert/strict";
import test from "node:test";

import {
  ApiClientError,
  isApiClientError,
  requestJson,
} from "../dist/index.js";

function jsonResponse(value, init = {}) {
  const headers = new Headers(init.headers);
  headers.set("content-type", "application/json; charset=utf-8");
  return new Response(JSON.stringify(value), { ...init, headers });
}

test("successful requests use same-origin credentials, JSON accept and session CSRF", async () => {
  let captured;
  const value = await requestJson("https://console.example/api/items", {
    method: "POST",
    body: "{}",
    headers: { "x-custom": "kept" },
    csrfToken: "csrf-token",
    fetchImpl: async (_url, init) => {
      captured = init;
      return jsonResponse({ ok: true });
    },
  });
  assert.deepEqual(value, { ok: true });
  assert.equal(captured.credentials, "same-origin");
  const headers = new Headers(captured.headers);
  assert.equal(headers.get("accept"), "application/json");
  assert.equal(headers.get("x-custom"), "kept");
  assert.equal(headers.get("x-csrf-token"), "csrf-token");

  await requestJson("https://console.example/api/items", {
    csrfToken: "must-not-be-sent-on-get",
    fetchImpl: async (_url, init) => {
      assert.equal(new Headers(init.headers).has("x-csrf-token"), false);
      return jsonResponse({ ok: true });
    },
  });
});

test("204 responses return undefined without requiring a content type", async () => {
  const value = await requestJson("https://console.example/api/logout", {
    method: "POST",
    fetchImpl: async () => new Response(null, { status: 204 }),
  });
  assert.equal(value, undefined);
});

test("error envelopes preserve status, code, request id and Retry-After", async () => {
  await assert.rejects(
    requestJson("https://console.example/api/items", {
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

test("401 invokes session invalidation without replacing the API error", async () => {
  let invalidations = 0;
  await assert.rejects(
    requestJson("https://console.example/api/session", {
      fetchImpl: async () =>
        jsonResponse(
          { code: "unauthorized", message: "sign in", retryable: false },
          { status: 401 },
        ),
      onUnauthorized: async (error) => {
        invalidations += 1;
        assert.equal(error.code, "unauthorized");
        throw new Error("local cleanup failed");
      },
    }),
    (error) => error instanceof ApiClientError && error.code === "unauthorized",
  );
  assert.equal(invalidations, 1);
});

test("invalid error responses do not expose arbitrary response text", async () => {
  await assert.rejects(
    requestJson("https://console.example/api/items", {
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
});

test("success responses require JSON and enforce a streaming byte limit", async () => {
  await assert.rejects(
    requestJson("https://console.example/api/items", {
      fetchImpl: async () =>
        new Response("ok", { headers: { "content-type": "text/plain" } }),
    }),
    (error) => error.code === "invalid_content_type",
  );
  await assert.rejects(
    requestJson("https://console.example/api/items", {
      maxResponseBytes: 4,
      fetchImpl: async () => jsonResponse({ longer: true }),
    }),
    (error) => error.code === "response_too_large",
  );

  const problem = await requestJson("https://console.example/api/items", {
    fetchImpl: async () =>
      new Response('{"ok":true}', {
        headers: { "content-type": "application/problem+json" },
      }),
  });
  assert.deepEqual(problem, { ok: true });
});

test("timeout and caller cancellation have distinct typed errors", async () => {
  const waitForAbort = async (_url, init) =>
    new Promise((_resolve, reject) => {
      if (init.signal.aborted) {
        reject(init.signal.reason);
        return;
      }
      init.signal.addEventListener("abort", () => reject(init.signal.reason), {
        once: true,
      });
    });
  await assert.rejects(
    requestJson("https://console.example/api/slow", {
      timeoutMs: 20,
      fetchImpl: waitForAbort,
    }),
    (error) => error.code === "request_timeout" && error.retryable === true,
  );

  const controller = new AbortController();
  controller.abort(new DOMException("cancelled", "AbortError"));
  await assert.rejects(
    requestJson("https://console.example/api/cancelled", {
      signal: controller.signal,
      fetchImpl: waitForAbort,
    }),
    (error) => error.code === "request_aborted" && error.retryable === false,
  );
});

test("invalid budgets and CSRF values fail before network I/O", async () => {
  let called = false;
  const fetchImpl = async () => {
    called = true;
    return jsonResponse({ ok: true });
  };
  await assert.rejects(
    requestJson("https://console.example/api/items", { timeoutMs: 0, fetchImpl }),
    RangeError,
  );
  await assert.rejects(
    requestJson("https://console.example/api/items", {
      method: "POST",
      csrfToken: "bad\nvalue",
      fetchImpl,
    }),
    TypeError,
  );
  assert.equal(called, false);
});
