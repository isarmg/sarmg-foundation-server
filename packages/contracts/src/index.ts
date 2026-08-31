export const MAX_ERROR_CODE_BYTES = 128;

export type ErrorCode = string;

export type ErrorEnvelope = {
  code: ErrorCode;
  message: string;
  request_id?: string;
  retryable: boolean;
  details?: Record<string, unknown>;
};

export type StateContract = {
  contract_version: 1;
  application: string;
  application_version: string;
  source_revision: string;
  schema: {
    revision: number;
    sha256: string;
  } | null;
  maintenance_locks: string[];
  resources: Array<{
    name: string;
    kind: "sqlite" | "configuration" | "data-tree" | "recordings" | "companion-contract";
    required: boolean;
  }>;
  external_requirements: Array<{
    kind: string;
    kid: string;
    algorithm: string;
    envelope_version: number;
  }>;
  companion_contracts: Array<{
    name: string;
    version: string;
    platform: string;
    sha256: string;
  }>;
};

const SHA256_PATTERN = /^[0-9a-f]{64}$/;
const IDENTIFIER_PATTERN = /^[A-Za-z0-9._:-]{1,128}$/;

export function isStateContract(value: unknown): value is StateContract {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "contract_version", "application", "application_version", "source_revision",
      "schema", "maintenance_locks", "resources", "external_requirements",
      "companion_contracts",
    ]) ||
    value.contract_version !== 1
  ) return false;
  if (
    !isIdentifier(value.application) ||
    !isIdentifier(value.application_version) ||
    !isSourceRevision(value.source_revision) ||
    !Array.isArray(value.maintenance_locks) ||
    !value.maintenance_locks.every(isIdentifier) ||
    !Array.isArray(value.resources) ||
    !value.resources.every(isStateResource) ||
    !Array.isArray(value.external_requirements) ||
    !value.external_requirements.every(isExternalRequirement) ||
    !Array.isArray(value.companion_contracts) ||
    !value.companion_contracts.every(isCompanionContract)
  ) return false;
  return value.schema === null || isSchemaIdentity(value.schema);
}

const ERROR_CODE_PATTERN = /^[a-z][a-z0-9._-]*$/;

export function isErrorCode(value: unknown): value is ErrorCode {
  return (
    typeof value === "string" &&
    value.length <= MAX_ERROR_CODE_BYTES &&
    ERROR_CODE_PATTERN.test(value)
  );
}

export function isErrorEnvelope(value: unknown): value is ErrorEnvelope {
  if (!isRecord(value)) return false;
  return (
    isErrorCode(value.code) &&
    typeof value.message === "string" &&
    typeof value.retryable === "boolean" &&
    (value.request_id === undefined || typeof value.request_id === "string") &&
    (value.details === undefined || isRecord(value.details))
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isIdentifier(value: unknown): value is string {
  return typeof value === "string" && IDENTIFIER_PATTERN.test(value);
}

function isSourceRevision(value: unknown): value is string {
  return typeof value === "string" && /^[0-9a-f]{40}$/.test(value);
}

function isSchemaIdentity(value: unknown): boolean {
  return isRecord(value) && hasExactKeys(value, ["revision", "sha256"]) &&
    Number.isSafeInteger(value.revision) && Number(value.revision) >= 0 &&
    typeof value.sha256 === "string" && SHA256_PATTERN.test(value.sha256);
}

function isStateResource(value: unknown): boolean {
  return isRecord(value) && hasExactKeys(value, ["name", "kind", "required"]) && isIdentifier(value.name) &&
    ["sqlite", "configuration", "data-tree", "recordings", "companion-contract"].includes(String(value.kind)) &&
    typeof value.required === "boolean";
}

function isExternalRequirement(value: unknown): boolean {
  return isRecord(value) && hasExactKeys(value, ["kind", "kid", "algorithm", "envelope_version"]) && isIdentifier(value.kind) && isIdentifier(value.kid) &&
    isIdentifier(value.algorithm) && Number.isSafeInteger(value.envelope_version) &&
    Number(value.envelope_version) > 0;
}

function isCompanionContract(value: unknown): boolean {
  return isRecord(value) && hasExactKeys(value, ["name", "version", "platform", "sha256"]) && isIdentifier(value.name) && isIdentifier(value.version) &&
    isIdentifier(value.platform) && typeof value.sha256 === "string" &&
    SHA256_PATTERN.test(value.sha256);
}

function hasExactKeys(value: Record<string, unknown>, expected: string[]): boolean {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  return actual.length === wanted.length && actual.every((key, index) => key === wanted[index]);
}
