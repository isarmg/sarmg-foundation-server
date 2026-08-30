export const MAX_ERROR_CODE_BYTES = 128;

export type ErrorCode = string;

export type ErrorEnvelope = {
  code: ErrorCode;
  message: string;
  request_id?: string;
  retryable: boolean;
  details?: Record<string, unknown>;
};

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
