import assert from "node:assert/strict";
import test from "node:test";
import { isAdministratorPassword, createAdministratorApiClient } from "../dist/index.js";

test("browser password policy enforces UTF-8 bytes and rejects non-scalar text", () => {
  for (const value of ["a".repeat(12), "a".repeat(1024), "é".repeat(512), "🔐".repeat(256)]) assert.equal(isAdministratorPassword(value), true);
  for (const value of ["a".repeat(11), "a".repeat(1025), `a${"é".repeat(512)}`, "🔐".repeat(257), `valid password\n`, `valid password\ud800`, null, {}]) assert.equal(isAdministratorPassword(value), false);
});
test("out-of-policy credentials do not dispatch a login request", async () => {
  let calls = 0;
  const client = createAdministratorApiClient({ baseUrl: "https://example.invalid", fetchImpl: async () => { calls++; throw new Error("must not dispatch"); } });
  await assert.rejects(client.login("admin", `a${"é".repeat(512)}`), TypeError);
  await assert.rejects(client.login("admin", "short"), TypeError);
  assert.equal(calls, 0);
});
