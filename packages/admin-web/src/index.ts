import {
  ADMIN_AUTH_PATHS,
  isAdministratorLoginRequest,
  isAdministratorSession,
  type AdministratorSession,
} from "@sarmg/contracts";
import {
  ApiClientError,
  requestJson,
  type RequestJsonOptions,
} from "@sarmg/http-client";

export const ADMIN_WEB_TOOLCHAIN = Object.freeze({
  node: "26.7.0",
  react: "19.2.8",
  reactDom: "19.2.8",
  vite: "7.3.6",
  viteReactPlugin: "4.7.0",
  typescript: "5.8.3",
  typesReact: "19.2.18",
  typesReactDom: "19.2.5",
} as const);

export type JsonGuard<T> = (value: unknown) => value is T;
export type AdministratorSessionListener = (session: AdministratorSession | null) => void;

export type AdministratorApiClientOptions = {
  baseUrl?: string | URL;
  fetchImpl?: typeof fetch;
  onUnauthorized?: () => void | Promise<void>;
};

export type AdministratorApiClient = {
  login(username: string, password: string): Promise<AdministratorSession>;
  restore(): Promise<AdministratorSession>;
  logout(): Promise<void>;
  request<T>(path: string, guard: JsonGuard<T>, init?: RequestInit): Promise<T>;
  currentSession(): AdministratorSession | null;
  subscribe(listener: AdministratorSessionListener): () => void;
};

type AuthenticationRequestContext = {
  generation: number;
  csrfToken: string | null;
  sessionAtDispatch: AdministratorSession | null;
};

/**
 * Fail a consumer build when its React/Vite or Node baseline drifts from the
 * exact current Foundation release. Version ranges are intentionally invalid.
 */
export function assertAdministratorWebToolchain(
  manifest: unknown,
  nodeVersionFile: string,
): void {
  if (!isRecord(manifest)) {
    throw new TypeError("Web package manifest must be an object");
  }
  const expectedEngine = `>=${ADMIN_WEB_TOOLCHAIN.node} <27`;
  const engines = requireRecord(manifest.engines, "engines");
  if (engines.node !== expectedEngine) {
    throw new TypeError(`engines.node must be exactly ${expectedEngine}`);
  }
  if (
    nodeVersionFile !== ADMIN_WEB_TOOLCHAIN.node &&
    nodeVersionFile !== `${ADMIN_WEB_TOOLCHAIN.node}\n`
  ) {
    throw new TypeError(`.node-version must be exactly ${ADMIN_WEB_TOOLCHAIN.node}`);
  }

  const dependencies = requireRecord(manifest.dependencies, "dependencies");
  const development = requireRecord(manifest.devDependencies, "devDependencies");
  for (const [section, values, dependency, expected] of [
    ["dependencies", dependencies, "react", ADMIN_WEB_TOOLCHAIN.react],
    ["dependencies", dependencies, "react-dom", ADMIN_WEB_TOOLCHAIN.reactDom],
    ["devDependencies", development, "vite", ADMIN_WEB_TOOLCHAIN.vite],
    [
      "devDependencies",
      development,
      "@vitejs/plugin-react",
      ADMIN_WEB_TOOLCHAIN.viteReactPlugin,
    ],
    ["devDependencies", development, "typescript", ADMIN_WEB_TOOLCHAIN.typescript],
    ["devDependencies", development, "@types/react", ADMIN_WEB_TOOLCHAIN.typesReact],
    [
      "devDependencies",
      development,
      "@types/react-dom",
      ADMIN_WEB_TOOLCHAIN.typesReactDom,
    ],
  ] as const) {
    if (values[dependency] !== expected) {
      throw new TypeError(`${section}.${dependency} must be exactly ${expected}`);
    }
  }

  const expectedVersions: Readonly<Record<string, string>> = {
    react: ADMIN_WEB_TOOLCHAIN.react,
    "react-dom": ADMIN_WEB_TOOLCHAIN.reactDom,
    vite: ADMIN_WEB_TOOLCHAIN.vite,
    "@vitejs/plugin-react": ADMIN_WEB_TOOLCHAIN.viteReactPlugin,
    typescript: ADMIN_WEB_TOOLCHAIN.typescript,
    "@types/react": ADMIN_WEB_TOOLCHAIN.typesReact,
    "@types/react-dom": ADMIN_WEB_TOOLCHAIN.typesReactDom,
  };
  for (const section of [
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
  ] as const) {
    const raw = manifest[section];
    if (raw === undefined) continue;
    const values = requireRecord(raw, section);
    for (const [dependency, expected] of Object.entries(expectedVersions)) {
      if (Object.hasOwn(values, dependency) && values[dependency] !== expected) {
        throw new TypeError(`${section}.${dependency} must be exactly ${expected}`);
      }
    }
  }
}

/**
 * Create one application-wide administrator API client. Authentication data
 * stays in closure state and is never persisted in localStorage/sessionStorage.
 */
export function createAdministratorApiClient(
  options: AdministratorApiClientOptions = {},
): AdministratorApiClient {
  const baseUrl = resolveAdministratorBaseUrl(options.baseUrl);
  const listeners = new Set<AdministratorSessionListener>();
  let session: AdministratorSession | null = null;
  // This private snapshot follows serialized Set-Cookie mutations even when
  // their public result has already been superseded. It exists solely so a
  // queued logout can carry the CSRF token issued by a preceding login.
  let transportSession: AdministratorSession | null = null;
  let restorePromise: Promise<AdministratorSession> | null = null;
  let authenticationMutationTail: Promise<void> = Promise.resolve();
  // Every login/logout/401 advances the generation so an older network
  // response cannot resurrect or overwrite newer authentication state.
  let authenticationGeneration = 0;

  const publish = (next: AdministratorSession | null) => {
    session = next;
    for (const listener of listeners) listener(next);
  };

  const invalidate = () => {
    authenticationGeneration += 1;
    transportSession = null;
    publish(null);
  };

  const requireCurrentOperation = (generation: number) => {
    if (generation !== authenticationGeneration) {
      throw new ApiClientError({
        code: "auth_operation_superseded",
        message: "Authentication operation was superseded by newer session state",
      });
    }
  };

  const send = async <T>(
    path: string,
    guard: JsonGuard<T>,
    init: RequestInit = {},
    authenticationContext: AuthenticationRequestContext = {
      generation: authenticationGeneration,
      csrfToken: session?.csrf_token ?? null,
      sessionAtDispatch: session,
    },
  ): Promise<T> => {
    const target = resolveApiPath(path, baseUrl);
    const headers = new Headers(init.headers);
    if (headers.has("x-csrf-token")) {
      throw new TypeError("x-csrf-token is owned by the administrator session client");
    }
    if (init.body !== undefined && !headers.has("content-type")) {
      headers.set("content-type", "application/json");
    }
    if (init.credentials !== undefined && init.credentials !== "same-origin") {
      throw new TypeError("administrator requests require same-origin credentials");
    }
    if (init.cache !== undefined && init.cache !== "no-store") {
      throw new TypeError("administrator requests require cache: no-store");
    }
    const { signal, ...requestInit } = init;
    const requestOptions: RequestJsonOptions = {
      ...requestInit,
      ...(signal === null || signal === undefined ? {} : { signal }),
      baseUrl,
      fetchImpl: options.fetchImpl,
      headers,
      cache: "no-store",
      ...(authenticationContext.csrfToken === null
        ? {}
        : { csrfToken: authenticationContext.csrfToken }),
      onUnauthorized: async () => {
        // A request from an older session must not erase a newer login.
        if (
          authenticationContext.generation !== authenticationGeneration ||
          authenticationContext.sessionAtDispatch !== session
        ) return;
        invalidate();
        await options.onUnauthorized?.();
      },
    };
    const value = await requestJson<unknown>(target, requestOptions);
    if (!guard(value)) {
      throw new ApiClientError({
        code: "invalid_response_shape",
        message: "Server response does not match the current Sarmg contract",
      });
    }
    return value;
  };

  const enqueueAuthenticationMutation = <T>(operation: () => Promise<T>): Promise<T> => {
    const pending = authenticationMutationTail.then(operation, operation);
    authenticationMutationTail = pending.then(
      () => undefined,
      () => undefined,
    );
    return pending;
  };

  const client: AdministratorApiClient = {
    async login(username, password) {
      const credentials: unknown = { username, password };
      if (!isAdministratorLoginRequest(credentials)) {
        throw new TypeError("administrator credentials violate the current contract");
      }
      const generation = authenticationGeneration + 1;
      authenticationGeneration = generation;
      restorePromise = null;
      if (session !== null) publish(null);
      return enqueueAuthenticationMutation(async () => {
        const received = await send(
          ADMIN_AUTH_PATHS.login,
          isAdministratorSession,
          {
            method: "POST",
            body: JSON.stringify(credentials),
          },
          { generation, csrfToken: null, sessionAtDispatch: null },
        );
        const authenticated = freezeAdministratorSession(received);
        // Authentication mutations are serialized, so this is the cookie
        // session a later queued logout needs even if the UI superseded it.
        transportSession = authenticated;
        requireCurrentOperation(generation);
        publish(authenticated);
        return authenticated;
      });
    },

    restore() {
      if (restorePromise) return restorePromise;
      const generation = authenticationGeneration;
      const precedingMutations = authenticationMutationTail;
      const pending = precedingMutations
        .then(() => {
          requireCurrentOperation(generation);
          return send(
            ADMIN_AUTH_PATHS.session,
            isAdministratorSession,
            {},
            {
              generation,
              csrfToken: null,
              sessionAtDispatch: session,
            },
          );
        })
        .then((received) => {
          requireCurrentOperation(generation);
          const authenticated = freezeAdministratorSession(received);
          transportSession = authenticated;
          publish(authenticated);
          return authenticated;
        })
        .catch((error: unknown) => {
          if (
            generation === authenticationGeneration &&
            error instanceof ApiClientError &&
            isInvalidSessionResponse(error.code)
          ) {
            invalidate();
          }
          throw error;
        })
        .finally(() => {
          if (restorePromise === pending) restorePromise = null;
        });
      restorePromise = pending;
      return pending;
    },

    async logout() {
      const generation = authenticationGeneration + 1;
      authenticationGeneration = generation;
      restorePromise = null;
      // Local authorization ends synchronously; the server mutation remains
      // serialized so overlapping login/logout calls finish in call order.
      if (session !== null) publish(null);
      return enqueueAuthenticationMutation(async () => {
        const csrfToken = transportSession?.csrf_token ?? null;
        try {
          await send(
            ADMIN_AUTH_PATHS.logout,
            isUndefined,
            { method: "POST" },
            { generation, csrfToken, sessionAtDispatch: null },
          );
        } finally {
          transportSession = null;
          if (generation === authenticationGeneration && session !== null) {
            publish(null);
          }
        }
      });
    },

    request: send,
    currentSession: () => session,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };

  return client;
}

function resolveAdministratorBaseUrl(configured: string | URL | undefined): URL {
  const locationHref = runtimeLocationHref();
  if (configured === undefined && locationHref === undefined) {
    throw new TypeError("baseUrl is required outside an HTTP(S) browser runtime");
  }
  const base = configured === undefined
    ? new URL("/", locationHref)
    : new URL(configured);
  if (
    !["http:", "https:"].includes(base.protocol) ||
    base.username !== "" ||
    base.password !== ""
  ) {
    throw new TypeError("baseUrl must be an HTTP(S) URL without user information");
  }
  if (locationHref !== undefined) {
    const runtime = new URL(locationHref);
    if (base.origin !== runtime.origin) {
      throw new TypeError("baseUrl must match the browser runtime origin");
    }
  }
  return base;
}

function runtimeLocationHref(): string | undefined {
  try {
    return typeof globalThis.location?.href === "string"
      ? globalThis.location.href
      : undefined;
  } catch {
    return undefined;
  }
}

function resolveApiPath(path: string, baseUrl: string | URL): URL {
  if (
    typeof path !== "string" ||
    !path.startsWith("/api/v2/") ||
    /[\u0000-\u001f\u007f]/.test(path)
  ) {
    throw new TypeError("API path must stay under /api/v2/");
  }
  const base = new URL(baseUrl);
  const target = new URL(path, base);
  if (
    target.origin !== base.origin ||
    !target.pathname.startsWith("/api/v2/") ||
    target.hash !== ""
  ) {
    throw new TypeError("API path must stay under /api/v2/");
  }
  return target;
}

function isUndefined(value: unknown): value is undefined {
  return value === undefined;
}

function isInvalidSessionResponse(code: string): boolean {
  return code === "invalid_response_shape" ||
    code === "invalid_content_type" ||
    code === "invalid_json_response" ||
    code === "response_too_large";
}

function freezeAdministratorSession(
  value: AdministratorSession,
): AdministratorSession {
  return Object.freeze({ ...value });
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function requireRecord(value: unknown, label: string): Record<string, unknown> {
  if (!isRecord(value)) throw new TypeError(`${label} must be an object`);
  return value;
}
