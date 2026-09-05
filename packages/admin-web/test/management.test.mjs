import assert from "node:assert/strict";
import test from "node:test";
import { createAdministratorApiClient, createAdministratorManagementClient } from "../dist/index.js";

test("management uses the current guarded namespace, shared CSRF and empty mutation responses", async () => {
  const calls = [];
  const session = { authenticated: true, user_id: "A".repeat(43), username: "admin", role: "admin", csrf_token: "A".repeat(43) };
  const client = createAdministratorApiClient({ baseUrl: "http://127.0.0.1", fetchImpl: async (url, init) => {
    calls.push([String(url), init]);
    if (String(url).endsWith("/session")) return Response.json(session);
    if (init.method === "POST") return new Response(null, { status: 204 });
    return Response.json([]);
  } });
  await client.restore();
  const management = createAdministratorManagementClient(client);
  await management.list(10, 20);
  await management.create("secondary", "correct horse battery");
  await management.setPassword("admin:2", "replacement password");
  await management.disable("admin:2");
  assert.equal(calls[1][0], "http://127.0.0.1/api/v2/platform/administrators?limit=10&offset=20");
  assert.equal(calls[3][0], "http://127.0.0.1/api/v2/platform/administrators/admin%3A2/password");
  for (const [, init] of calls.slice(2)) {
    assert.equal(new Headers(init.headers).get("x-csrf-token"), session.csrf_token);
    assert.equal(init.credentials, "same-origin"); assert.equal(init.cache, "no-store");
  }
  assert.equal(calls.at(-1)[1].body, undefined);
  assert.throws(() => management.list(101));
  assert.throws(() => management.list(10, Number.MAX_SAFE_INTEGER + 1));
  await assert.rejects(management.disable("../escape"));
  await assert.rejects(management.create("admin", "secret\ntext"));
});
