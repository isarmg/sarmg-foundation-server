import {
  isErrorEnvelope,
  type ErrorEnvelope,
} from "@isarmg/contracts";

export type { ErrorEnvelope } from "@isarmg/contracts";

export const DEFAULT_TIMEOUT_MS = 10_000;
export const DEFAULT_MAX_RESPONSE_BYTES = 2 * 1024 * 1024;
export const MAX_RESPONSE_BYTES = 64 * 1024 * 1024;

const MAX_ERROR_RESPONSE_BYTES = 64 * 1024;
const MAX_RETRY_AFTER_SECONDS = 24 * 60 * 60;
const CSRF_HEADER = "x-csrf-token";

export type RequestJsonOptions = Omit<RequestInit, "signal"> & {
  signal?: AbortSignal;
  timeoutMs?: number;
  maxResponseBytes?: number;
  csrfToken?: string;
  fetchImpl?: typeof fetch;
  onUnauthorized?: (error: ApiClientError) => void | Promise<void>;
};

export class ApiClientError extends Error {
  readonly status: number;
  readonly code: string;
  readonly requestId?: string;
  readonly retryable: boolean;
  readonly retryAfterSeconds?: number;
  readonly details?: Record<string, unknown>;
  readonly envelope?: ErrorEnvelope;

  constructor(options: {
    message: string;
    status?: number;
    code: string;
    requestId?: string;
    retryable?: boolean;
    retryAfterSeconds?: number;
    details?: Record<string, unknown>;
    envelope?: ErrorEnvelope;
    cause?: unknown;
  }) {
    super(options.message, { cause: options.cause });
    this.name = "ApiClientError";
    this.status = options.status ?? 0;
    this.code = options.code;
    this.requestId = options.requestId;
    this.retryable = options.retryable ?? false;
    this.retryAfterSeconds = options.retryAfterSeconds;
    this.details = options.details;
    this.envelope = options.envelope;
  }
}

export function isApiClientError(error: unknown): error is ApiClientError {
  return error instanceof ApiClientError;
}

export async function requestJson<T>(
  url: string | URL,
  options: RequestJsonOptions = {},
): Promise<T> {
  const {
    timeoutMs = DEFAULT_TIMEOUT_MS,
    maxResponseBytes = DEFAULT_MAX_RESPONSE_BYTES,
    csrfToken,
    fetchImpl = globalThis.fetch,
    onUnauthorized,
    signal: callerSignal,
    ...requestInit
  } = options;
  validateBudget("timeoutMs", timeoutMs, 1, 120_000);
  validateBudget("maxResponseBytes", maxResponseBytes, 1, MAX_RESPONSE_BYTES);
  if (typeof fetchImpl !== "function") {
    throw new TypeError("fetchImpl must be a function");
  }

  const headers = new Headers(requestInit.headers);
  if (!headers.has("accept")) headers.set("accept", "application/json");
  const method = (requestInit.method ?? "GET").toUpperCase();
  if (csrfToken !== undefined && isUnsafeMethod(method)) {
    if (csrfToken.length === 0 || /[\r\n]/.test(csrfToken)) {
      throw new TypeError("csrfToken must be a non-empty single-line value");
    }
    headers.set(CSRF_HEADER, csrfToken);
  }

  const controller = new AbortController();
  let timedOut = false;
  const timeout = setTimeout(() => {
    timedOut = true;
    controller.abort(new DOMException("request timed out", "TimeoutError"));
  }, timeoutMs);
  const abortFromCaller = () => controller.abort(callerSignal?.reason);
  if (callerSignal?.aborted) abortFromCaller();
  else callerSignal?.addEventListener("abort", abortFromCaller, { once: true });

  try {
    let response: Response;
    try {
      response = await fetchImpl(url, {
        ...requestInit,
        credentials: requestInit.credentials ?? "same-origin",
        headers,
        signal: controller.signal,
      });
    } catch (cause) {
      if (timedOut) {
        throw localError("request_timeout", "Request timed out", true, cause);
      }
      if (callerSignal?.aborted) {
        throw localError("request_aborted", "Request was aborted", false, cause);
      }
      throw localError("network_error", "Network request failed", true, cause);
    }

    const requestId = safeRequestId(response.headers.get("x-request-id"));
    if (!response.ok) {
      const error = await responseError(response, requestId, controller.signal);
      if (response.status === 401 && onUnauthorized) {
        try {
          void Promise.resolve(onUnauthorized(error)).catch(() => undefined);
        } catch {
          // Session cleanup must not replace the authoritative API error.
        }
      }
      throw error;
    }

    if (
      method === "HEAD" ||
      response.status === 204 ||
      response.status === 205 ||
      response.headers.get("content-length") === "0"
    ) {
      return undefined as T;
    }
    if (!isJsonContentType(response.headers.get("content-type"))) {
      throw new ApiClientError({
        status: response.status,
        code: "invalid_content_type",
        message: "Server response is not JSON",
        requestId,
      });
    }
    try {
      const text = await readBoundedText(response, maxResponseBytes, requestId);
      return JSON.parse(text) as T;
    } catch (cause) {
      if (isApiClientError(cause) || controller.signal.aborted) throw cause;
      throw new ApiClientError({
        status: response.status,
        code: "invalid_json_response",
        message: "Server returned invalid JSON",
        requestId,
        cause,
      });
    }
  } catch (cause) {
    if (isApiClientError(cause)) throw cause;
    if (timedOut) {
      throw localError("request_timeout", "Request timed out", true, cause);
    }
    if (callerSignal?.aborted) {
      throw localError("request_aborted", "Request was aborted", false, cause);
    }
    throw localError("network_error", "Network request failed", true, cause);
  } finally {
    clearTimeout(timeout);
    callerSignal?.removeEventListener("abort", abortFromCaller);
  }
}

async function responseError(
  response: Response,
  headerRequestId?: string,
  signal?: AbortSignal,
): Promise<ApiClientError> {
  const retryAfterSeconds = parseRetryAfter(response.headers.get("retry-after"));
  if (!isJsonContentType(response.headers.get("content-type"))) {
    return new ApiClientError({
      status: response.status,
      code: "invalid_error_response",
      message: "Server returned an invalid error response",
      requestId: headerRequestId,
      retryAfterSeconds,
    });
  }
  let value: unknown;
  try {
    value = JSON.parse(
      await readBoundedText(response, MAX_ERROR_RESPONSE_BYTES, headerRequestId),
    ) as unknown;
  } catch (cause) {
    if (signal?.aborted) throw cause;
    if (isApiClientError(cause)) return cause;
    return new ApiClientError({
      status: response.status,
      code: "invalid_error_response",
      message: "Server returned an invalid error response",
      requestId: headerRequestId,
      retryAfterSeconds,
      cause,
    });
  }
  if (!isErrorEnvelope(value)) {
    return new ApiClientError({
      status: response.status,
      code: "invalid_error_response",
      message: "Server returned an invalid error response",
      requestId: headerRequestId,
      retryAfterSeconds,
    });
  }
  return new ApiClientError({
    status: response.status,
    code: value.code,
    message: value.message,
    requestId: safeRequestId(value.request_id) ?? headerRequestId,
    retryable: value.retryable,
    retryAfterSeconds,
    details: value.details,
    envelope: value,
  });
}

async function readBoundedText(
  response: Response,
  maximumBytes: number,
  requestId?: string,
): Promise<string> {
  const declared = response.headers.get("content-length");
  if (declared !== null && /^\d+$/.test(declared) && Number(declared) > maximumBytes) {
    throw responseTooLarge(response.status, requestId);
  }
  if (!response.body) return "";

  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let length = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      length += value.byteLength;
      if (length > maximumBytes) {
        try {
          await reader.cancel();
        } catch {
          // The size violation remains authoritative even if cancellation fails.
        }
        throw responseTooLarge(response.status, requestId);
      }
      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }
  const body = new Uint8Array(length);
  let offset = 0;
  for (const chunk of chunks) {
    body.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return new TextDecoder("utf-8", { fatal: true }).decode(body);
}

function responseTooLarge(status: number, requestId?: string): ApiClientError {
  return new ApiClientError({
    status,
    code: "response_too_large",
    message: "Server response exceeds the configured size limit",
    requestId,
  });
}

function localError(
  code: string,
  message: string,
  retryable: boolean,
  cause: unknown,
): ApiClientError {
  return new ApiClientError({ code, message, retryable, cause });
}

function isUnsafeMethod(method: string): boolean {
  return !["GET", "HEAD", "OPTIONS", "TRACE"].includes(method);
}

function isJsonContentType(value: string | null): boolean {
  if (value === null) return false;
  const mediaType = value.split(";", 1)[0]?.trim().toLowerCase();
  return mediaType === "application/json" || mediaType?.endsWith("+json") === true;
}

function safeRequestId(value: string | undefined | null): string | undefined {
  if (value === undefined || value === null || value.length === 0 || value.length > 256) {
    return undefined;
  }
  return /[\r\n\u0000-\u001f\u007f]/.test(value) ? undefined : value;
}

function parseRetryAfter(value: string | null, now = Date.now()): number | undefined {
  if (value === null) return undefined;
  const trimmed = value.trim();
  if (/^\d+$/.test(trimmed)) {
    const seconds = Number(trimmed);
    return Number.isSafeInteger(seconds)
      ? Math.min(seconds, MAX_RETRY_AFTER_SECONDS)
      : undefined;
  }
  const date = Date.parse(trimmed);
  if (!Number.isFinite(date)) return undefined;
  return Math.min(
    Math.max(0, Math.ceil((date - now) / 1_000)),
    MAX_RETRY_AFTER_SECONDS,
  );
}

function validateBudget(name: string, value: number, minimum: number, maximum: number): void {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new RangeError(`${name} must be an integer between ${minimum} and ${maximum}`);
  }
}
