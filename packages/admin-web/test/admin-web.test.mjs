import assert from "node:assert/strict";
import test from "node:test";

import {
  ADMIN_WEB_TOOLCHAIN,
  assertAdministratorWebToolchain,
  createAdministratorApiClient,
} from "../dist/index.js";

const BASE_URL = "https://console.example/";
const TOKEN_A = "A".repeat(43);
const TOKEN_B = `${"B".repeat(42)}E`;
const SESSION = {
  authenticated: true,
  user_id: "018f1f4b-7a5d-7b5f-8d31-123456789abc",
  username: "admin",
  role: "admin",
  csrf_token: TOKEN_A,
};

function json(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json" },
  });
}

test("admin client uses exact endpoints, validates sessions, and owns CSRF in memory", async () => {
  const requests = [];
  const client = createAdministratorApiClient({
    baseUrl: BASE_URL,
    fetchImpl: async (url, init) => {
      requests.push({ url, init });
      if (new URL(url).pathname.endsWith("/auth/logout")) {
        return new Response(null, { status: 204 });
      }
      return json(SESSION);
    },
  });
  const observed = [];
  const unsubscribe = client.subscribe((session) => observed.push(session));
  assert.deepEqual(
    await client.login(" Admin ", "correct horse battery staple"),
    SESSION,
  );
  assert.deepEqual(await client.restore(), SESSION);
  await client.request("/api/v2/private", (value) => value?.authenticated === true, {
    method: "POST",
    body: "{}",
  });
  await client.logout();
  unsubscribe();

  assert.deepEqual(
    requests.map(({ url }) => new URL(url).pathname),
    ["/api/v2/auth/login", "/api/v2/auth/session", "/api/v2/private", "/api/v2/auth/logout"],
  );
  assert.equal(new Headers(requests[0].init.headers).has("x-csrf-token"), false);
  assert.deepEqual(JSON.parse(requests[0].init.body), {
    username: " Admin ",
    password: "correct horse battery staple",
  });
  assert.equal(Object.hasOwn(JSON.parse(requests[0].init.body), "email"), false);
  assert.equal(new Headers(requests[2].init.headers).get("x-csrf-token"), SESSION.csrf_token);
  assert.deepEqual(requests.map(({ init }) => init.cache), ["no-store", "no-store", "no-store", "no-store"]);
  assert.equal(Object.isFrozen(observed[0]), true);
  assert.equal(client.currentSession(), null);
  assert.deepEqual(observed, [SESSION, SESSION, null]);
});

test("admin client rejects non-admin roles, path escapes, and malformed credentials", async () => {
  let calls = 0;
  const client = createAdministratorApiClient({
    baseUrl: BASE_URL,
    fetchImpl: async () => {
      calls += 1;
      return json({ ...SESSION, role: "viewer" });
    },
  });
  await assert.rejects(client.login("admin", "correct horse battery staple"), (error) => {
    return error.code === "invalid_response_shape";
  });
  await assert.rejects(client.login("admin", ""), TypeError);
  await assert.rejects(client.login("管理员", "correct horse battery staple"), TypeError);
  await assert.rejects(client.login("a".repeat(65), "correct horse battery staple"), TypeError);
  await assert.rejects(client.request("https://evil.example/api/v2/x", () => true), TypeError);
  await assert.rejects(client.request("/api/v1/x", () => true), TypeError);
  await assert.rejects(
    client.request("/api/v2/x", () => true, { credentials: "omit" }),
    /same-origin credentials/,
  );
  await assert.rejects(
    client.request("/api/v2/x", () => true, { cache: "reload" }),
    /cache: no-store/,
  );
  assert.equal(calls, 1);
});

test("admin client never sends CSRF data to a different browser origin", () => {
  const previous = Object.getOwnPropertyDescriptor(globalThis, "location");
  Object.defineProperty(globalThis, "location", {
    configurable: true,
    value: { href: "https://console.example/application" },
  });
  try {
    assert.throws(
      () => createAdministratorApiClient({ baseUrl: "https://other.example/" }),
      /match the browser runtime origin/,
    );
    assert.doesNotThrow(
      () => createAdministratorApiClient({ baseUrl: "https://console.example/" }),
    );
  } finally {
    if (previous === undefined) delete globalThis.location;
    else Object.defineProperty(globalThis, "location", previous);
  }
});

test("logout prevents an older session restore from republishing authentication", async () => {
  let completeRestore;
  const client = createAdministratorApiClient({
    baseUrl: BASE_URL,
    fetchImpl: async (url) => {
      const path = new URL(url).pathname;
      if (path.endsWith("/auth/session")) {
        return new Promise((resolve) => {
          completeRestore = () => resolve(json(SESSION));
        });
      }
      if (path.endsWith("/auth/logout")) {
        return new Response(null, { status: 204 });
      }
      throw new Error(`unexpected request ${path}`);
    },
  });

  const restoring = client.restore();
  await Promise.resolve();
  await client.logout();
  completeRestore();
  await assert.rejects(restoring, (error) => {
    return error.code === "auth_operation_superseded";
  });
  assert.equal(client.currentSession(), null);
});

test("a stale 401 cannot erase a newer administrator login", async () => {
  let rejectOldRequest;
  let unauthorizedCallbacks = 0;
  const newerSession = {
    ...SESSION,
    user_id: "018f1f4b-7a5d-7b5f-8d31-000000000002",
    csrf_token: TOKEN_B,
  };
  const client = createAdministratorApiClient({
    baseUrl: BASE_URL,
    onUnauthorized: () => {
      unauthorizedCallbacks += 1;
    },
    fetchImpl: async (url) => {
      const path = new URL(url).pathname;
      if (path === "/api/v2/private") {
        return new Promise((resolve) => {
          rejectOldRequest = () => resolve(json({
            code: "unauthorized",
            message: "session expired",
            retryable: false,
          }, 401));
        });
      }
      if (path.endsWith("/auth/login")) return json(newerSession);
      if (path.endsWith("/auth/session")) return json(SESSION);
      throw new Error(`unexpected request ${path}`);
    },
  });

  await client.restore();
  const oldRequest = client.request("/api/v2/private", () => true);
  await client.login("admin", "correct horse battery staple");
  rejectOldRequest();
  await assert.rejects(oldRequest, (error) => error.code === "unauthorized");
  assert.deepEqual(client.currentSession(), newerSession);
  assert.equal(unauthorizedCallbacks, 0);
});

test("restore fails closed when a server emits a noncanonical session shape", async () => {
  let restores = 0;
  const client = createAdministratorApiClient({
    baseUrl: BASE_URL,
    fetchImpl: async () => {
      restores += 1;
      return restores === 1
        ? json(SESSION)
        : json({ ...SESSION, csrf_token: "noncanonical-token-shape" });
    },
  });

  await client.restore();
  await assert.rejects(
    client.restore(),
    (error) => error.code === "invalid_response_shape",
  );
  assert.equal(client.currentSession(), null);
});

test("logout clears local state immediately and owns the outgoing CSRF header", async () => {
  let finishLogout;
  let logoutHeaders;
  const client = createAdministratorApiClient({
    baseUrl: BASE_URL,
    fetchImpl: async (url, init) => {
      const path = new URL(url).pathname;
      if (path.endsWith("/auth/session")) return json(SESSION);
      if (path.endsWith("/auth/logout")) {
        logoutHeaders = new Headers(init.headers);
        return new Promise((resolve) => {
          finishLogout = () => resolve(new Response(null, { status: 204 }));
        });
      }
      throw new Error(`unexpected request ${path}`);
    },
  });

  await client.restore();
  const loggingOut = client.logout();
  assert.equal(client.currentSession(), null);
  await Promise.resolve();
  assert.equal(logoutHeaders.get("x-csrf-token"), SESSION.csrf_token);
  finishLogout();
  await loggingOut;
  await assert.rejects(
    client.request("/api/v2/private", () => true, {
      headers: { "x-csrf-token": "caller-controlled" },
    }),
    /owned by the administrator session client/,
  );
});

test("overlapping login then logout is serialized and ends anonymous", async () => {
  const requests = [];
  const client = createAdministratorApiClient({
    baseUrl: BASE_URL,
    fetchImpl: async (url, init) => {
      const path = new URL(url).pathname;
      requests.push({ path, headers: new Headers(init.headers) });
      return path.endsWith("/auth/login")
        ? json(SESSION)
        : new Response(null, { status: 204 });
    },
  });

  const loggingIn = client.login("admin", "correct horse battery staple");
  const loggingOut = client.logout();
  await assert.rejects(loggingIn, (error) => error.code === "auth_operation_superseded");
  await loggingOut;
  assert.deepEqual(requests.map(({ path }) => path), [
    "/api/v2/auth/login",
    "/api/v2/auth/logout",
  ]);
  assert.equal(requests[1].headers.get("x-csrf-token"), SESSION.csrf_token);
  assert.equal(client.currentSession(), null);
});

test("toolchain contract is exact and range-free", () => {
  assert.equal(Object.isFrozen(ADMIN_WEB_TOOLCHAIN), true);
  assert.deepEqual(ADMIN_WEB_TOOLCHAIN, {
    node: "26.7.0",
    react: "19.2.8",
    reactDom: "19.2.8",
    vite: "7.3.6",
    viteReactPlugin: "4.7.0",
    typescript: "5.8.3",
    typesReact: "19.2.18",
    typesReactDom: "19.2.5",
  });
  for (const version of Object.values(ADMIN_WEB_TOOLCHAIN)) {
    assert.match(version, /^\d+\.\d+\.\d+$/);
  }
});

test("consumer toolchain assertion rejects ranges and Node drift", () => {
  const manifest = {
    engines: { node: ">=26.7.0 <27" },
    dependencies: { react: "19.2.8", "react-dom": "19.2.8" },
    devDependencies: {
      "@types/react": "19.2.18",
      "@types/react-dom": "19.2.5",
      "@vitejs/plugin-react": "4.7.0",
      typescript: "5.8.3",
      vite: "7.3.6",
    },
  };
  assert.doesNotThrow(() => assertAdministratorWebToolchain(manifest, "26.7.0\n"));
  assert.throws(
    () => assertAdministratorWebToolchain({
      ...manifest,
      dependencies: { ...manifest.dependencies, react: "^19.2.8" },
    }, "26.7.0"),
    /dependencies\.react must be exactly 19\.2\.8/,
  );
  assert.throws(
    () => assertAdministratorWebToolchain(manifest, "26.4.0"),
    /\.node-version must be exactly 26\.7\.0/,
  );
  assert.throws(
    () => assertAdministratorWebToolchain(manifest, " 26.7.0\n"),
    /\.node-version must be exactly 26\.7\.0/,
  );
  assert.throws(
    () => assertAdministratorWebToolchain({
      ...manifest,
      peerDependencies: { react: ">=19" },
    }, "26.7.0"),
    /peerDependencies\.react must be exactly 19\.2\.8/,
  );
});
