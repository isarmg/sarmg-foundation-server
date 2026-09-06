// Reviewed source snapshot of the current platform composition; no diagnostic HTTP route.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
const source = await readFile(new URL('../rust/crates/sarmg-server-runtime/src/lib.rs', import.meta.url), 'utf8');
const start = source.indexOf('#[derive(Clone)]\nstruct PlatformState');
const end = source.indexOf('pub fn install_panic_hook()', start);
if (start < 0 || end < 0 || process.argv.length < 3) throw new Error('Pass explicit consumer roots; source layout must match');
const header = '// Generated from sarmg-foundation-server. Update via sync-platform-router.mjs.\nuse axum::{Json, Router, extract::{DefaultBodyLimit, Request, State as AxumState}, http::{HeaderValue, StatusCode}, middleware::{self, Next}, response::{IntoResponse, Response}, routing::get};\nuse futures_util::FutureExt;\nuse std::sync::Arc;\nuse sarmg_server_runtime::{RuntimeHandle, new_request_id, LIVENESS_PATH, READINESS_PATH, DEFAULT_REQUEST_BODY_BYTES};\n';
const bytes = header + source.slice(start, end);
if (bytes.includes('/platform/diagnostics') || bytes.includes('get(diagnostics')) throw new Error('Diagnostics must not be distributed');
for (const root of process.argv.slice(2)) {
  const manifest = await readFile(resolve(root, 'Cargo.toml'), 'utf8');
  const edition = /^edition\s*=\s*"(2021|2024)"\s*$/m.exec(manifest)?.[1];
  if (!edition) throw new Error(root + ': expected an explicit supported Cargo edition');
  const folder = resolve(root, 'foundation'); await mkdir(folder, { recursive: true });
  await writeFile(join(folder, 'platform_router.rs'), bytes);
  execFileSync('rustfmt', ['--edition', edition, join(folder, 'platform_router.rs')], {timeout:10000});
  const formatted = await readFile(join(folder, 'platform_router.rs'));
  await writeFile(join(folder, 'platform-router.json'), JSON.stringify({source:'sarmg-foundation-server/rust/crates/sarmg-server-runtime/src/lib.rs',license:'Apache-2.0',diagnostics:false,sha256:createHash('sha256').update(formatted).digest('hex')},null,2)+'\n');
  console.log(root + ': diagnostics-free platform router synchronized');
}
