/** Current platform password policy for browser form admission, measured in UTF-8 bytes. */
export const ADMINISTRATOR_PASSWORD_MIN_BYTES = 12;
export const ADMINISTRATOR_PASSWORD_MAX_BYTES = 1024;

export function isAdministratorPassword(value: unknown): value is string {
  if (typeof value !== "string" || value.length > ADMINISTRATOR_PASSWORD_MAX_BYTES || /[\u0000-\u001f\u007f\ud800-\udfff]/u.test(value)) return false;
  const bytes = new TextEncoder().encode(value).byteLength;
  return bytes >= ADMINISTRATOR_PASSWORD_MIN_BYTES && bytes <= ADMINISTRATOR_PASSWORD_MAX_BYTES;
}
