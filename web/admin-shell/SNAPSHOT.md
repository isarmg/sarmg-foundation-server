# Administrator shell integration and acceptance

The shared shell provides light/dark icon switching, top navigation, and full-width content. The official xcss package supplies the authentication client, CSRF handling, and administrator contracts.
See [workspace configuration](../../docs/admin-workspace.md) for shared defaults and consumer overrides.

Products install the single `@xcss/web` package from an immutable release archive, pin its integrity digest in the lockfile, and import public subpaths such as `admin-shell` and `admin-ui`.
They use one shared shell context; independent builds require no neighboring xcss source checkout.
Consumers choosing `web-embedded-native` use the shared native Web entrypoint.

Each product's manifests and lockfiles determine its actual version and full source revision. Acceptance results are tied to that product's source commit, CI, and formal release.
