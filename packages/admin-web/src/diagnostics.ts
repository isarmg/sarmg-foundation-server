export const PLATFORM_DIAGNOSTICS_PATH = "/api/v2/platform/diagnostics";
export type PlatformDiagnostics = {
  request_id: string | null;
  product: { id: string; version: string; foundation_revision: string; profile: string; capabilities: string[] };
  schema_identity: { application: string; application_version: string; schema_revision: number; schema_sha256: string } | null;
  health: { live: boolean; ready: boolean; degraded: boolean };
  checks: Record<string, boolean>;
  tasks: Record<string, { criticality: "critical" | "degrading" | "best_effort"; state: "running" | "stopped" | "completed" | "failed" | "panicked" | "aborted" }>;
  metrics: Record<"audit_backlog" | "operation_backlog" | "spool_pending_bytes" | "spool_pending_records", number | null>;
};

function record(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
function exact(value: Record<string, unknown>, keys: string[]) {
  return Object.keys(value).length === keys.length && keys.every(key => Object.hasOwn(value, key));
}
function label(value: unknown): value is string {
  return typeof value === "string" && /^[A-Za-z0-9._:-]{1,128}$/.test(value);
}
function count(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}
function entries(value: unknown, guard: (item: unknown) => boolean): boolean {
  return record(value) && Object.keys(value).length <= 128
    && Object.entries(value).every(([key, item]) => label(key) && guard(item));
}

/** Reject unknown fields; diagnostics is a closed redacted contract, not arbitrary JSON. */
export function isPlatformDiagnostics(value: unknown): value is PlatformDiagnostics {
  if (!record(value) || !exact(value, ["request_id", "product", "schema_identity", "health", "checks", "tasks", "metrics"])) return false;
  const { product, schema_identity: schema, health, metrics } = value;
  if (!(value.request_id === null || label(value.request_id)) || !record(product)
    || !exact(product, ["id", "version", "foundation_revision", "profile", "capabilities"])
    || !label(product.id) || !label(product.version) || !label(product.profile)
    || typeof product.foundation_revision !== "string" || !/^[a-fA-F0-9]{40}$/.test(product.foundation_revision)
    || !Array.isArray(product.capabilities) || product.capabilities.length > 128 || !product.capabilities.every(label)) return false;
  if (schema !== null && (!record(schema)
    || !exact(schema, ["application", "application_version", "schema_revision", "schema_sha256"])
    || schema.application !== product.id || schema.application_version !== product.version
    || !count(schema.schema_revision) || typeof schema.schema_sha256 !== "string" || !/^[a-f0-9]{64}$/.test(schema.schema_sha256))) return false;
  return record(health) && exact(health, ["live", "ready", "degraded"])
    && Object.values(health).every(item => typeof item === "boolean")
    && entries(value.checks, item => typeof item === "boolean")
    && entries(value.tasks, item => record(item) && exact(item, ["criticality", "state"])
      && typeof item.criticality === "string" && ["critical", "degrading", "best_effort"].includes(item.criticality)
      && typeof item.state === "string" && ["running", "stopped", "completed", "failed", "panicked", "aborted"].includes(item.state))
    && record(metrics) && exact(metrics, ["audit_backlog", "operation_backlog", "spool_pending_bytes", "spool_pending_records"])
    && Object.values(metrics).every(item => item === null || count(item));
}
