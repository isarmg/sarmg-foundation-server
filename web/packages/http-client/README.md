# `@isarmg/http-client`

Experimental same-origin JSON client for ISArmg product consoles. It defaults to a 10-second
timeout and a 2 MiB success-response limit, validates JSON content types and the shared
`ErrorEnvelope`, and never includes arbitrary non-JSON error bodies in thrown messages.

```ts
import { requestJson } from "@isarmg/http-client";

const host = await requestJson<Host>("/api/hosts/1", {
  csrfToken: session.csrfToken,
  onUnauthorized: () => session.clear(),
});
```

Callers should branch on `ApiClientError.code` or `status`, never on display text. Passing a CSRF
token adds it only to unsafe methods. This package remains 0.x and must be integration-tested in
each product before replacing a mature local client. Import wire types such as `ErrorEnvelope`
directly from `@isarmg/contracts`; this package does not re-export contract types.
