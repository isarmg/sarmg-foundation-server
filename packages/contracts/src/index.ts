export const MAX_ERROR_CODE_BYTES = 128;
export const MAX_REQUEST_ID_BYTES = 128;
export const MAX_SAFE_JSON_INTEGER = Number.MAX_SAFE_INTEGER;
export const ADMINISTRATOR_USERNAME_MIN_BYTES = 3;
export const ADMINISTRATOR_USERNAME_MAX_BYTES = 64;
export const AUTHENTICATION_TOKEN_ENCODED_BYTES = 43;
export const ADMINISTRATOR_ROLE = "admin" as const;
export const ADMIN_AUTH_PATHS = Object.freeze({
  login: "/api/v2/auth/login",
  session: "/api/v2/auth/session",
  logout: "/api/v2/auth/logout",
} as const);

export type ErrorCode = string;
export type RequestId = string;

export type AdministratorRole = typeof ADMINISTRATOR_ROLE;

// Login fields are bounded untrusted candidates. The server-side
// sarmg-admin-auth primitive owns canonicalization and password verification;
// accepting a candidate here is not acceptance of a noncurrent credential.
export type AdministratorLoginRequest = {
  username: string;
  password: string;
};

export type AdministratorSession = {
  readonly authenticated: true;
  readonly user_id: string;
  readonly username: string;
  readonly role: AdministratorRole;
  readonly csrf_token: string;
};

export type ErrorEnvelope = {
  code: ErrorCode;
  message: string;
  request_id?: string;
  retryable: boolean;
  details?: Record<string, unknown>;
};

export type SchemaIdentity = {
  revision: number;
  sha256: string;
};

export type StateResourceKind =
  | "sqlite"
  | "configuration"
  | "data-tree"
  | "recordings"
  | "companion-contract";

export type StateResource = {
  name: string;
  kind: StateResourceKind;
  required: boolean;
};

export type StateExternalRequirement = {
  kind: string;
  kid: string;
  algorithm: string;
  envelope_version: number;
};

export type CompanionContract = {
  name: string;
  version: string;
  platform: string;
  sha256: string;
};

export type StateContract = {
  contract_version: 1;
  application: string;
  application_version: string;
  source_revision: string;
  schema: SchemaIdentity | null;
  maintenance_locks: string[];
  resources: StateResource[];
  external_requirements: StateExternalRequirement[];
  companion_contracts: CompanionContract[];
};

export type ReleaseIdentity = {
  product: string;
  version: string;
  source_revision: string;
  target: string;
  state_contract_sha256: string;
};

export type BackupSchemaIdentity = {
  application: string;
  application_version: string;
  schema_revision: number;
  schema_sha256: string;
};

export type BackupExternalRequirement = {
  kind: string;
  kid: string;
  sha256: string;
  algorithm: string;
  envelope_version: number;
};

export type BackupResource = {
  name: string;
  kind: StateResourceKind;
  path: string;
  bytes: number;
  files: number;
  sha256: string;
};

export type BackupManifest = {
  manifest_version: 2;
  tool_version: string;
  product: string;
  application_version: string;
  schema_identity: BackupSchemaIdentity | null;
  created_at_epoch_seconds: number;
  external_requirements: BackupExternalRequirement[];
  resources: BackupResource[];
};

const SHA256_PATTERN = /^[0-9a-f]{64}$/;
const IDENTIFIER_PATTERN = /^[A-Za-z0-9._:-]{1,128}$/;
const SOURCE_REVISION_PATTERN = /^[0-9a-f]{40}$/;
const ERROR_CODE_PATTERN = /^[a-z][a-z0-9._-]*$/;
const CANONICAL_ADMINISTRATOR_USERNAME_PATTERN =
  /^[a-z0-9](?:[a-z0-9._-]*[a-z0-9])$/;
const ADMINISTRATOR_USERNAME_CANDIDATE_PATTERN =
  /^[\x20-\x7e]+$/;
// 32 bytes produce 42 complete base64url sextets plus four payload bits;
// canonical unpadded encoding therefore restricts the final character.
const AUTHENTICATION_TOKEN_PATTERN =
  /^[A-Za-z0-9_-]{42}[AEIMQUYcgkosw048]$/;
const STATE_RESOURCE_KINDS: readonly StateResourceKind[] = [
  "sqlite",
  "configuration",
  "data-tree",
  "recordings",
  "companion-contract",
];

export function isAdministratorLoginRequest(
  value: unknown,
): value is AdministratorLoginRequest {
  return (
    isRecord(value) &&
    hasExactKeys(value, ["username", "password"]) &&
    typeof value.username === "string" &&
    value.username.length <= ADMINISTRATOR_USERNAME_MAX_BYTES &&
    ADMINISTRATOR_USERNAME_CANDIDATE_PATTERN.test(value.username) &&
    isBoundedCredentialText(value.password, 1_024)
  );
}

export function isAdministratorSession(value: unknown): value is AdministratorSession {
  return (
    isRecord(value) &&
    hasExactKeys(value, [
      "authenticated",
      "user_id",
      "username",
      "role",
      "csrf_token",
    ]) &&
    value.authenticated === true &&
    isIdentifier(value.user_id) &&
    isCanonicalAdministratorUsername(value.username) &&
    value.role === ADMINISTRATOR_ROLE &&
    isAuthenticationToken(value.csrf_token)
  );
}

export function isCanonicalAdministratorUsername(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length >= ADMINISTRATOR_USERNAME_MIN_BYTES &&
    value.length <= ADMINISTRATOR_USERNAME_MAX_BYTES &&
    CANONICAL_ADMINISTRATOR_USERNAME_PATTERN.test(value)
  );
}

export function isAuthenticationToken(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length === AUTHENTICATION_TOKEN_ENCODED_BYTES &&
    AUTHENTICATION_TOKEN_PATTERN.test(value)
  );
}

export function isErrorCode(value: unknown): value is ErrorCode {
  return (
    typeof value === "string" &&
    value.length <= MAX_ERROR_CODE_BYTES &&
    ERROR_CODE_PATTERN.test(value)
  );
}

export function isRequestId(value: unknown): value is RequestId {
  return typeof value === "string" && IDENTIFIER_PATTERN.test(value);
}

export function isErrorEnvelope(value: unknown): value is ErrorEnvelope {
  if (
    !isRecord(value) ||
    !hasRequiredAndAllowedKeys(
      value,
      ["code", "message", "retryable"],
      ["request_id", "details"],
    )
  ) return false;
  return (
    isErrorCode(value.code) &&
    typeof value.message === "string" &&
    typeof value.retryable === "boolean" &&
    (!Object.hasOwn(value, "request_id") || isRequestId(value.request_id)) &&
    (!Object.hasOwn(value, "details") || isRecord(value.details))
  );
}

export function isStateContract(value: unknown): value is StateContract {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "contract_version",
      "application",
      "application_version",
      "source_revision",
      "schema",
      "maintenance_locks",
      "resources",
      "external_requirements",
      "companion_contracts",
    ]) ||
    value.contract_version !== 1
  ) return false;
  if (
    !isIdentifier(value.application) ||
    !isIdentifier(value.application_version) ||
    !isSourceRevision(value.source_revision) ||
    !isUniqueArray(value.maintenance_locks, isIdentifier) ||
    !isArrayOf(value.resources, isStateResource) ||
    !isArrayOf(value.external_requirements, isStateExternalRequirement) ||
    !isArrayOf(value.companion_contracts, isCompanionContract)
  ) return false;
  return value.schema === null || isSchemaIdentity(value.schema);
}

export function isReleaseIdentity(value: unknown): value is ReleaseIdentity {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "product",
      "version",
      "source_revision",
      "target",
      "state_contract_sha256",
    ])
  ) return false;
  return (
    isIdentifier(value.product) &&
    isIdentifier(value.version) &&
    isSourceRevision(value.source_revision) &&
    isIdentifier(value.target) &&
    isSha256(value.state_contract_sha256)
  );
}

export function isBackupManifest(value: unknown): value is BackupManifest {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "manifest_version",
      "tool_version",
      "product",
      "application_version",
      "schema_identity",
      "created_at_epoch_seconds",
      "external_requirements",
      "resources",
    ]) ||
    value.manifest_version !== 2
  ) return false;
  return (
    isIdentifier(value.tool_version) &&
    isIdentifier(value.product) &&
    isIdentifier(value.application_version) &&
    (value.schema_identity === null || isBackupSchemaIdentity(value.schema_identity)) &&
    isNonNegativeInteger(value.created_at_epoch_seconds) &&
    isArrayOf(value.external_requirements, isBackupExternalRequirement) &&
    Array.isArray(value.resources) &&
    value.resources.length >= 1 &&
    value.resources.every(isBackupResource)
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isIdentifier(value: unknown): value is string {
  return typeof value === "string" && IDENTIFIER_PATTERN.test(value);
}

function isBoundedCredentialText(value: unknown, maximumCodePoints: number): value is string {
  if (typeof value !== "string" || /[\u0000-\u001f\u007f]/.test(value)) return false;
  const length = [...value].length;
  return length >= 1 && length <= maximumCodePoints;
}

function isSourceRevision(value: unknown): value is string {
  return typeof value === "string" && SOURCE_REVISION_PATTERN.test(value);
}

function isSha256(value: unknown): value is string {
  return typeof value === "string" && SHA256_PATTERN.test(value);
}

function isNonNegativeInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && Number(value) >= 0;
}

function isPositiveInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && Number(value) >= 1;
}

function isSchemaIdentity(value: unknown): value is SchemaIdentity {
  return (
    isRecord(value) &&
    hasExactKeys(value, ["revision", "sha256"]) &&
    isNonNegativeInteger(value.revision) &&
    isSha256(value.sha256)
  );
}

function isStateResource(value: unknown): value is StateResource {
  return (
    isRecord(value) &&
    hasExactKeys(value, ["name", "kind", "required"]) &&
    isIdentifier(value.name) &&
    isStateResourceKind(value.kind) &&
    typeof value.required === "boolean"
  );
}

function isStateResourceKind(value: unknown): value is StateResourceKind {
  return typeof value === "string" && STATE_RESOURCE_KINDS.includes(value as StateResourceKind);
}

function isStateExternalRequirement(value: unknown): value is StateExternalRequirement {
  return (
    isRecord(value) &&
    hasExactKeys(value, ["kind", "kid", "algorithm", "envelope_version"]) &&
    isIdentifier(value.kind) &&
    isIdentifier(value.kid) &&
    isIdentifier(value.algorithm) &&
    isPositiveInteger(value.envelope_version)
  );
}

function isCompanionContract(value: unknown): value is CompanionContract {
  return (
    isRecord(value) &&
    hasExactKeys(value, ["name", "version", "platform", "sha256"]) &&
    isIdentifier(value.name) &&
    isIdentifier(value.version) &&
    isIdentifier(value.platform) &&
    isSha256(value.sha256)
  );
}

function isBackupSchemaIdentity(value: unknown): value is BackupSchemaIdentity {
  return (
    isRecord(value) &&
    hasExactKeys(value, [
      "application",
      "application_version",
      "schema_revision",
      "schema_sha256",
    ]) &&
    isIdentifier(value.application) &&
    isIdentifier(value.application_version) &&
    isNonNegativeInteger(value.schema_revision) &&
    isSha256(value.schema_sha256)
  );
}

function isBackupExternalRequirement(value: unknown): value is BackupExternalRequirement {
  return (
    isRecord(value) &&
    hasExactKeys(value, ["kind", "kid", "sha256", "algorithm", "envelope_version"]) &&
    isIdentifier(value.kind) &&
    isIdentifier(value.kid) &&
    isSha256(value.sha256) &&
    isIdentifier(value.algorithm) &&
    isPositiveInteger(value.envelope_version)
  );
}

function isBackupResource(value: unknown): value is BackupResource {
  return (
    isRecord(value) &&
    hasExactKeys(value, ["name", "kind", "path", "bytes", "files", "sha256"]) &&
    isIdentifier(value.name) &&
    isStateResourceKind(value.kind) &&
    typeof value.path === "string" &&
    value.path.length >= 1 &&
    isNonNegativeInteger(value.bytes) &&
    isPositiveInteger(value.files) &&
    isSha256(value.sha256)
  );
}

function isArrayOf<T>(
  value: unknown,
  predicate: (entry: unknown) => entry is T,
): value is T[] {
  return Array.isArray(value) && value.every(predicate);
}

function isUniqueArray<T>(
  value: unknown,
  predicate: (entry: unknown) => entry is T,
): value is T[] {
  return (
    Array.isArray(value) &&
    value.every(predicate) &&
    new Set(value).size === value.length
  );
}

function hasExactKeys(value: Record<string, unknown>, expected: readonly string[]): boolean {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  return actual.length === wanted.length && actual.every((key, index) => key === wanted[index]);
}

function hasRequiredAndAllowedKeys(
  value: Record<string, unknown>,
  required: readonly string[],
  optional: readonly string[],
): boolean {
  const actual = Object.keys(value);
  const allowed = new Set([...required, ...optional]);
  return required.every((key) => Object.hasOwn(value, key)) &&
    actual.every((key) => allowed.has(key));
}
