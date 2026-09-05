import assert from "node:assert/strict";
import test from "node:test";
import { isPlatformDiagnostics } from "../dist/index.js";

const current = {
  request_id: "diag-123",
  product: { id: "sentinel", version: "0.2.0", foundation_revision: "a".repeat(40), profile: "server", capabilities: ["operations"] },
  schema_identity: { application: "sentinel", application_version: "0.2.0", schema_revision: 3, schema_sha256: "b".repeat(64) },
  health: { live: true, ready: true, degraded: false },
  checks: { database: true }, tasks: { reconciler: { criticality: "critical", state: "running" } },
  metrics: { audit_backlog: 0, operation_backlog: 2, spool_pending_bytes: null, spool_pending_records: null },
};
test("current redacted diagnostics distinguish unsupported counters from zero", () => {
  assert.equal(isPlatformDiagnostics(current), true);
  assert.equal(isPlatformDiagnostics({ ...current, schema_identity: null }), true);
  assert.equal(isPlatformDiagnostics({ ...current, metrics: { ...current.metrics, operation_backlog: -1 } }), false);
  assert.equal(isPlatformDiagnostics({ ...current, metrics: { ...current.metrics, audit_backlog: Number.MAX_SAFE_INTEGER + 1 } }), false);
});
test("diagnostics reject unknown fields, secrets, wrong schema identity and unknown states", () => {
  for (const mutate of [
    v => { v.database_url = "secret"; },
    v => { v.product.token = "secret"; },
    v => { v.health.error = "internal path"; },
    v => { v.schema_identity.application = "other"; },
    v => { v.tasks.reconciler.state = "unknown"; },
    v => { v.tasks.reconciler.criticality = ["critical"]; },
    v => { v.tasks.reconciler.error = "internal path"; },
    v => { v.request_id = "secret\npath"; },
    v => { delete v.metrics.audit_backlog; },
  ]) {
    const value = structuredClone(current); mutate(value);
    assert.equal(isPlatformDiagnostics(value), false);
  }
});
