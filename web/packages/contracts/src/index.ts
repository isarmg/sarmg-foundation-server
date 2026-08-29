export type ApiError = {
  code: string;
  message: string;
  request_id?: string;
  retryable?: boolean;
  details?: Record<string, unknown>;
};
