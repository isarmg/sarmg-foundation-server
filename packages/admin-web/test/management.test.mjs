import assert from "node:assert/strict";
import test from "node:test";
import { createAdministratorApiClient } from "../dist/index.js";

test("self-service waits for restore, uses its session CSRF and revokes local authorization", async () => {
  const session = { authenticated: true, user_id: "A".repeat(43), username: "admin", role: "admin", csrf_token: "A".repeat(43) };
  let releaseRestore, restores = 0, mutations = 0;
  const client = createAdministratorApiClient({ baseUrl: "http://127.0.0.1", fetchImpl: async (url, init) => {
    if (String(url).endsWith("/session")) {
      if (++restores === 1) return Response.json(session);
      return new Promise(resolve => { releaseRestore = () => resolve(Response.json(session)); });
    }
    mutations++;
    assert.equal(String(url), "http://127.0.0.1/api/v2/platform/administrators/self");
    assert.equal(new Headers(init.headers).get("x-csrf-token"), session.csrf_token);
    assert.deepEqual(JSON.parse(init.body), { username: "renamed", current_password: "correct horse battery" });
    return new Response(null, { status: 204 });
  } });
  await client.restore();
  const restoring = client.restore();
  await Promise.resolve();
  const updating = client.updateAccount({ username: "renamed", current_password: "correct horse battery" });
  assert.equal(mutations, 0);
  releaseRestore();
  await restoring;
  await updating;
  assert.equal(mutations, 1);
  assert.equal(client.currentSession(), null);
  await assert.rejects(client.updateAccount({ username: "admin", current_password: "secret\ntext" }));
});
