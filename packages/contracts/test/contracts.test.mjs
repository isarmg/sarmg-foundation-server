import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { isErrorCode, isErrorEnvelope, isStateContract } from "../dist/index.js";

test("0.2 declarations expose only the authoritative error wire type", () => {
  const declarations = readFileSync(
    new URL("../dist/index.d.ts", import.meta.url),
    "utf8",
  );
  assert.match(declarations, /export type ErrorEnvelope\b/);
  assert.doesNotMatch(declarations, /\bApiError\b/);
});

test("state contracts fail closed on malformed and unknown fields", () => {
  const contract = {
    contract_version: 1,
    application: "media-backup",
    application_version: "0.2.0",
    source_revision: "a".repeat(40),
    schema: { revision: 1, sha256: "b".repeat(64) },
    maintenance_locks: ["database", "data-tree"],
    resources: [
      { name: "database", kind: "sqlite", required: true },
      { name: "blobs", kind: "data-tree", required: true },
    ],
    external_requirements: [],
    companion_contracts: [],
  };
  assert.equal(isStateContract(contract), true);
  assert.equal(isStateContract({ ...contract, compatibility: true }), false);
  assert.equal(isStateContract({ ...contract, source_revision: "unbound" }), false);
});

test("error codes match the Rust wire contract", () => {
  for (const value of [
    "bad_request",
    "photo.upload_conflict",
    "host-agent.rate-limited",
  ]) {
    assert.equal(isErrorCode(value), true);
  }
  for (const value of ["", "BadRequest", "1bad", "has space", "échec"]) {
    assert.equal(isErrorCode(value), false);
  }
  assert.equal(isErrorCode("a".repeat(128)), true);
  assert.equal(isErrorCode("a".repeat(129)), false);
});

test("error envelope requires stable branching fields and object details", () => {
  assert.equal(
    isErrorEnvelope({
      code: "too_many_requests",
      message: "try later",
      request_id: "request-1",
      retryable: true,
      details: { retry_after: 5 },
    }),
    true,
  );
  assert.equal(
    isErrorEnvelope({ code: "bad_request", message: "bad", retryable: false }),
    true,
  );
  assert.equal(isErrorEnvelope({ code: "bad_request", message: "bad" }), false);
  assert.equal(
    isErrorEnvelope({
      code: "bad_request",
      message: "bad",
      retryable: false,
      details: [],
    }),
    false,
  );
});
