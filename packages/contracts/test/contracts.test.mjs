import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { isDeepStrictEqual } from "node:util";

import {
  ADMIN_AUTH_PATHS,
  isBackupManifest,
  isAdministratorLoginRequest,
  isAdministratorSession,
  isAuthenticationToken,
  isCanonicalAdministratorUsername,
  isErrorCode,
  isErrorEnvelope,
  isReleaseIdentity,
  isRequestId,
  isStateContract,
} from "../dist/index.js";

test("administrator paths are one immutable current-only contract", () => {
  assert.equal(Object.isFrozen(ADMIN_AUTH_PATHS), true);
  assert.deepEqual(ADMIN_AUTH_PATHS, {
    login: "/api/v2/auth/login",
    session: "/api/v2/auth/session",
    logout: "/api/v2/auth/logout",
  });
});

test("generated administrator identity and token shapes are current-only", () => {
  for (const username of [
    "admin",
    "admin.operations",
    "admin_ops-2",
    "admin..operations",
    "admin__operations",
    "admin--operations",
    `a${"b".repeat(62)}z`,
  ]) {
    assert.equal(isCanonicalAdministratorUsername(username), true, username);
  }
  for (const username of [
    "ad",
    "Admin",
    ".admin",
    "admin-",
    "admin@example.com",
    "admin+operations",
    "admin operations",
    "管理员",
    `${"a".repeat(65)}`,
  ]) {
    assert.equal(isCanonicalAdministratorUsername(username), false, username);
  }
  assert.equal(isAuthenticationToken("A".repeat(43)), true);
  assert.equal(isAuthenticationToken(`${"B".repeat(42)}E`), true);
  assert.equal(isAuthenticationToken("B".repeat(43)), false);
  assert.equal(isAuthenticationToken("A".repeat(42)), false);
  assert.equal(isAuthenticationToken(`${"A".repeat(42)}=`), false);
});

test("administrator login username remains a bounded printable-ASCII candidate", () => {
  for (const username of ["A", " Admin ", "admin@example.test", "A".repeat(64)]) {
    assert.equal(
      isAdministratorLoginRequest({ username, password: "bounded candidate" }),
      true,
      username,
    );
  }
  for (const username of ["", "A".repeat(65), "admin\nops", "admin\u007fops", "管理员"]) {
    assert.equal(
      isAdministratorLoginRequest({ username, password: "bounded candidate" }),
      false,
      JSON.stringify(username),
    );
  }
  assert.equal(
    isAdministratorLoginRequest({
      email: "admin@example.test",
      password: "bounded candidate",
    }),
    false,
  );
});

const CONTRACT_CASES = [
  {
    name: "administrator-auth",
    fixture: "administrator-auth.fixtures.json",
    schema: "administrator-auth.schema.json",
    guard: (value) => isAdministratorLoginRequest(value) || isAdministratorSession(value),
  },
  {
    name: "error-envelope",
    fixture: "error-envelope.fixtures.json",
    schema: "error-envelope.schema.json",
    guard: isErrorEnvelope,
  },
  {
    name: "state-contract",
    fixture: "state-contract.fixtures.json",
    schema: "state-contract.schema.json",
    guard: isStateContract,
  },
  {
    name: "release",
    fixture: "release.fixtures.json",
    schema: "release.schema.json",
    guard: isReleaseIdentity,
  },
  {
    name: "backup-manifest",
    fixture: "backup-manifest.fixtures.json",
    schema: "backup-manifest.schema.json",
    guard: isBackupManifest,
  },
];

test("declarations expose each authoritative current contract and no extra alias", () => {
  const declarations = readFileSync(
    new URL("../dist/index.d.ts", import.meta.url),
    "utf8",
  );
  for (const name of [
    "ErrorEnvelope",
    "AdministratorLoginRequest",
    "AdministratorSession",
    "StateContract",
    "ReleaseIdentity",
    "BackupManifest",
  ]) {
    assert.match(declarations, new RegExp(`export type ${name}\\b`));
  }
  assert.doesNotMatch(declarations, /\bApiError\b/);
});

test("shared fixtures keep runtime guards and JSON Schemas in lockstep", async (t) => {
  for (const contract of CONTRACT_CASES) {
    await t.test(contract.name, () => {
      const fixtures = readJson(
        new URL(`../fixtures/${contract.fixture}`, import.meta.url),
      );
      fixtures.valid.forEach((value, index) => {
        assert.equal(contract.guard(value), true, `guard rejected valid fixture ${index}`);
        assert.equal(
          schemaAccepts(contract.schema, value),
          true,
          `schema rejected valid fixture ${index}`,
        );
      });
      fixtures.invalid.forEach((value, index) => {
        assert.equal(contract.guard(value), false, `guard accepted invalid fixture ${index}`);
        assert.equal(
          schemaAccepts(contract.schema, value),
          false,
          `schema accepted invalid fixture ${index}`,
        );
      });
    });
  }
});

test("every published Schema uses the canonical sarmg.org identifier", () => {
  for (const { schema } of CONTRACT_CASES) {
    const value = loadSchema(schema);
    assert.match(value.$id, /^https:\/\/sarmg\.org\/schemas\//);
    assert.equal(value.$id.includes("isarmg.org"), false);
  }
});

test("error codes match the Rust wire contract at both length boundaries", () => {
  for (const value of [
    "bad_request",
    "media.upload_conflict",
    "host-monitor.rate-limited",
  ]) {
    assert.equal(isErrorCode(value), true);
  }
  for (const value of ["", "BadRequest", "1bad", "has space", "échec"]) {
    assert.equal(isErrorCode(value), false);
  }
  assert.equal(isErrorCode("a".repeat(128)), true);
  assert.equal(isErrorCode("a".repeat(129)), false);
});

test("request IDs use the same bounded safe ASCII identifier in every wire implementation", () => {
  for (const value of ["request-1", "host:request_2", "a".repeat(128)]) {
    assert.equal(isRequestId(value), true);
  }
  for (const value of ["", "request id", "échec", "a".repeat(129)]) {
    assert.equal(isRequestId(value), false);
  }
});

const schemaCache = new Map();

function readJson(url) {
  return JSON.parse(readFileSync(url, "utf8"));
}

function loadSchema(name) {
  let schema = schemaCache.get(name);
  if (schema === undefined) {
    schema = readJson(new URL(`../schemas/${name}`, import.meta.url));
    schemaCache.set(name, schema);
  }
  return schema;
}

// This intentionally implements only the draft-2020-12 keywords used by this
// package. It keeps fixture tests dependency-free while exercising the shipped
// Schema documents themselves, including cross-document references.
function schemaAccepts(schemaName, value) {
  return validateSchema(loadSchema(schemaName), value, schemaName);
}

function validateSchema(schema, value, schemaName) {
  if (schema.$ref !== undefined) {
    const { target, targetName } = resolveReference(schema.$ref, schemaName);
    return validateSchema(target, value, targetName);
  }
  if (schema.oneOf !== undefined) {
    return schema.oneOf.filter((choice) => validateSchema(choice, value, schemaName)).length === 1;
  }
  if (schema.const !== undefined && !isDeepStrictEqual(value, schema.const)) return false;
  if (schema.enum !== undefined && !schema.enum.some((entry) => isDeepStrictEqual(value, entry))) {
    return false;
  }

  if (schema.type === "null" && value !== null) return false;
  if (schema.type === "boolean" && typeof value !== "boolean") return false;
  if (schema.type === "string") {
    if (typeof value !== "string") return false;
    const length = [...value].length;
    if (schema.minLength !== undefined && length < schema.minLength) return false;
    if (schema.maxLength !== undefined && length > schema.maxLength) return false;
    if (schema.pattern !== undefined && !new RegExp(schema.pattern, "u").test(value)) return false;
  }
  if (schema.type === "integer") {
    if (!Number.isInteger(value)) return false;
    if (schema.minimum !== undefined && value < schema.minimum) return false;
    if (schema.maximum !== undefined && value > schema.maximum) return false;
  }
  if (schema.type === "array") {
    if (!Array.isArray(value)) return false;
    if (schema.minItems !== undefined && value.length < schema.minItems) return false;
    if (schema.maxItems !== undefined && value.length > schema.maxItems) return false;
    if (
      schema.uniqueItems === true &&
      value.some((entry, index) => value.slice(0, index).some((prior) => isDeepStrictEqual(prior, entry)))
    ) return false;
    if (
      schema.items !== undefined &&
      !value.every((entry) => validateSchema(schema.items, entry, schemaName))
    ) return false;
  }
  if (schema.type === "object") {
    if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
    if (
      schema.required !== undefined &&
      !schema.required.every((key) => Object.hasOwn(value, key))
    ) return false;
    if (
      schema.additionalProperties === false &&
      Object.keys(value).some((key) => !Object.hasOwn(schema.properties ?? {}, key))
    ) return false;
    if (
      schema.properties !== undefined &&
      !Object.entries(schema.properties).every(
        ([key, propertySchema]) =>
          !Object.hasOwn(value, key) || validateSchema(propertySchema, value[key], schemaName),
      )
    ) return false;
  }
  return true;
}

function resolveReference(reference, currentName) {
  const hashAt = reference.indexOf("#");
  const document = hashAt === -1 ? reference : reference.slice(0, hashAt);
  const fragment = hashAt === -1 ? "" : reference.slice(hashAt + 1);
  const targetName = document === "" ? currentName : document.split("/").at(-1);
  let target = loadSchema(targetName);
  if (fragment !== "") {
    assert.ok(fragment.startsWith("/"), `unsupported Schema reference ${reference}`);
    for (const encodedPart of fragment.slice(1).split("/")) {
      const part = decodeURIComponent(encodedPart).replaceAll("~1", "/").replaceAll("~0", "~");
      target = target[part];
      assert.notEqual(target, undefined, `unresolved Schema reference ${reference}`);
    }
  }
  return { target, targetName };
}
