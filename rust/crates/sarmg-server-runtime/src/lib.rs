//! Shared server lifecycle and health primitives. Product routers remain outside this crate.

use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Request, State as AxumState},
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use std::{collections::BTreeMap, future::Future, sync::Arc};
use tokio::{
    sync::{RwLock, watch},
    task::JoinSet,
};

pub const LIVENESS_PATH: &str = "/healthz";
pub const READINESS_PATH: &str = "/readyz";
pub const DIAGNOSTICS_PATH: &str = "/api/v2/platform/diagnostics";
pub const DEFAULT_REQUEST_BODY_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProductDescriptor {
    pub id: String,
    pub version: String,
    pub foundation_revision: String,
    pub profile: String,
    pub capabilities: Vec<String>,
}

impl ProductDescriptor {
    pub fn validate(&self) -> Result<(), Error> {
        for (label, value) in [
            ("id", &self.id),
            ("version", &self.version),
            ("profile", &self.profile),
        ] {
            if value.is_empty()
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
            {
                return Err(Error::InvalidDescriptor(label));
            }
        }
        if self.foundation_revision.len() < 7
            || !self
                .foundation_revision
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            return Err(Error::InvalidDescriptor("foundation_revision"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskCriticality {
    Critical,
    Degrading,
    BestEffort,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Running,
    Completed,
    Failed,
    Panicked,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TaskSnapshot {
    pub criticality: TaskCriticality,
    pub state: TaskState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HealthSnapshot {
    pub live: bool,
    pub ready: bool,
    pub degraded: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Diagnostics {
    pub product: ProductDescriptor,
    pub health: HealthSnapshot,
    pub checks: BTreeMap<String, bool>,
    pub tasks: BTreeMap<String, TaskSnapshot>,
}

#[async_trait]
pub trait HealthCheck: Send + Sync {
    async fn check(&self) -> bool;
}

#[derive(Clone)]
pub struct RuntimeHandle {
    state: Arc<RwLock<RuntimeState>>,
    shutdown: watch::Sender<bool>,
}

impl RuntimeHandle {
    pub fn shutdown(&self) {
        let _ = self.shutdown.send(true);
    }
    pub fn shutdown_signal(&self) -> watch::Receiver<bool> {
        self.shutdown.subscribe()
    }
    pub async fn health(&self) -> HealthSnapshot {
        self.state.read().await.health()
    }
    pub async fn diagnostics(&self) -> Diagnostics {
        self.state.read().await.diagnostics()
    }
}

struct RuntimeState {
    product: ProductDescriptor,
    checks: BTreeMap<String, bool>,
    tasks: BTreeMap<String, TaskSnapshot>,
    shutting_down: bool,
}

impl RuntimeState {
    fn health(&self) -> HealthSnapshot {
        let critical_failed = self
            .tasks
            .values()
            .any(|t| t.criticality == TaskCriticality::Critical && t.state != TaskState::Running);
        let degraded = self
            .tasks
            .values()
            .any(|t| t.criticality == TaskCriticality::Degrading && t.state != TaskState::Running);
        HealthSnapshot {
            live: !critical_failed,
            ready: !self.shutting_down
                && !critical_failed
                && self.checks.values().all(|ready| *ready),
            degraded,
        }
    }
    fn diagnostics(&self) -> Diagnostics {
        Diagnostics {
            product: self.product.clone(),
            health: self.health(),
            checks: self.checks.clone(),
            tasks: self.tasks.clone(),
        }
    }
}

pub struct ServerRuntime {
    handle: RuntimeHandle,
    checks: Vec<(String, Arc<dyn HealthCheck>)>,
    tasks: Vec<TaskRegistration>,
}
struct TaskRegistration {
    name: String,
    criticality: TaskCriticality,
    task: Box<SupervisedTask>,
}
type SupervisedTask =
    dyn FnOnce(watch::Receiver<bool>) -> tokio::task::JoinHandle<Result<(), String>> + Send;

pub struct ServerRuntimeBuilder {
    product: ProductDescriptor,
    checks: Vec<(String, Arc<dyn HealthCheck>)>,
    tasks: Vec<TaskRegistration>,
}

impl ServerRuntime {
    pub fn builder(product: ProductDescriptor) -> ServerRuntimeBuilder {
        ServerRuntimeBuilder {
            product,
            checks: vec![],
            tasks: vec![],
        }
    }
    pub fn handle(&self) -> RuntimeHandle {
        self.handle.clone()
    }

    pub async fn run_until_shutdown(mut self) {
        let mut joins = JoinSet::new();
        for registration in self.tasks.drain(..) {
            let state = self.handle.state.clone();
            let shutdown = self.handle.shutdown_signal();
            joins.spawn(async move {
                let outcome = (registration.task)(shutdown).await;
                let task_state = match outcome {
                    Ok(Ok(())) => TaskState::Completed,
                    Ok(Err(_)) => TaskState::Failed,
                    Err(_) => TaskState::Panicked,
                };
                let mut guard = state.write().await;
                if let Some(snapshot) = guard.tasks.get_mut(&registration.name) {
                    snapshot.state = task_state;
                }
                guard.health()
            });
        }
        let mut runtime_shutdown = self.handle.shutdown.subscribe();
        loop {
            tokio::select! {
                result = joins.join_next(), if !joins.is_empty() => {
                    if let Some(Ok(health)) = result && !health.live { self.handle.shutdown(); break; }
                }
                changed = runtime_shutdown.changed() => { if changed.is_ok() { break; } }
            }
        }
        self.handle.state.write().await.shutting_down = true;
    }

    pub async fn refresh_health(&self) {
        for (name, check) in &self.checks {
            self.handle
                .state
                .write()
                .await
                .checks
                .insert(name.clone(), check.check().await);
        }
    }
}

impl ServerRuntimeBuilder {
    pub fn register_health_check(
        mut self,
        name: impl Into<String>,
        check: Arc<dyn HealthCheck>,
    ) -> Self {
        self.checks.push((name.into(), check));
        self
    }
    pub fn register_background_task<F, Fut>(
        mut self,
        name: impl Into<String>,
        criticality: TaskCriticality,
        task: F,
    ) -> Self
    where
        F: FnOnce(watch::Receiver<bool>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.tasks.push(TaskRegistration {
            name: name.into(),
            criticality,
            task: Box::new(move |shutdown| tokio::spawn(task(shutdown))),
        });
        self
    }
    pub async fn build(self) -> Result<ServerRuntime, Error> {
        self.product.validate()?;
        let mut task_states = BTreeMap::new();
        for task in &self.tasks {
            if task_states
                .insert(
                    task.name.clone(),
                    TaskSnapshot {
                        criticality: task.criticality,
                        state: TaskState::Running,
                    },
                )
                .is_some()
            {
                return Err(Error::DuplicateName(task.name.clone()));
            }
        }
        let mut check_states = BTreeMap::new();
        for (name, _) in &self.checks {
            if check_states.insert(name.clone(), false).is_some() {
                return Err(Error::DuplicateName(name.clone()));
            }
        }
        let (shutdown, _) = watch::channel(false);
        Ok(ServerRuntime {
            handle: RuntimeHandle {
                state: Arc::new(RwLock::new(RuntimeState {
                    product: self.product,
                    checks: check_states,
                    tasks: task_states,
                    shutting_down: false,
                })),
                shutdown,
            },
            checks: self.checks,
            tasks: self.tasks,
        })
    }
}

pub fn new_request_id() -> String {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("operating system randomness unavailable");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

struct PlatformState<Store> {
    handle: RuntimeHandle,
    product_id: String,
    mode: sarmg_admin_auth::AdministratorOriginMode,
    administrator: Arc<sarmg_admin_core::AdministratorService<Store>>,
}

impl<Store> Clone for PlatformState<Store> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
            product_id: self.product_id.clone(),
            mode: self.mode,
            administrator: Arc::clone(&self.administrator),
        }
    }
}

/// Compose the Foundation-owned auth, health, readiness, request-id and
/// administrator diagnostics routes. Product routes must be merged separately.
pub fn platform_router<Store>(
    handle: RuntimeHandle,
    product_id: impl Into<String>,
    mode: sarmg_admin_auth::AdministratorOriginMode,
    administrator: Arc<sarmg_admin_core::AdministratorService<Store>>,
) -> Result<Router, sarmg_admin_core::Error>
where
    Store: sarmg_admin_core::AdministratorStore + 'static,
{
    let product_id = product_id.into();
    let auth = sarmg_admin_axum::administrator_router(
        product_id.clone(),
        mode,
        Arc::clone(&administrator),
    )?;
    let state = PlatformState {
        handle,
        product_id,
        mode,
        administrator,
    };
    let runtime = Router::new()
        .route(LIVENESS_PATH, get(liveness::<Store>))
        .route(READINESS_PATH, get(readiness::<Store>))
        .route(DIAGNOSTICS_PATH, get(diagnostics::<Store>))
        .with_state(state);
    Ok(Router::new()
        .merge(auth)
        .merge(runtime)
        .layer(DefaultBodyLimit::max(DEFAULT_REQUEST_BODY_BYTES))
        .layer(middleware::from_fn(request_id_layer)))
}

async fn liveness<Store>(AxumState(state): AxumState<PlatformState<Store>>) -> StatusCode
where
    Store: sarmg_admin_core::AdministratorStore + 'static,
{
    if state.handle.health().await.live {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

async fn readiness<Store>(AxumState(state): AxumState<PlatformState<Store>>) -> Response
where
    Store: sarmg_admin_core::AdministratorStore + 'static,
{
    let ready = state.handle.health().await.ready;
    (
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(serde_json::json!({"ready": ready})),
    )
        .into_response()
}

async fn diagnostics<Store>(
    AxumState(state): AxumState<PlatformState<Store>>,
    request: Request,
) -> Response
where
    Store: sarmg_admin_core::AdministratorStore + 'static,
{
    if let Err(response) = sarmg_admin_axum::authenticate_request(
        &state.administrator,
        request.headers(),
        request.uri(),
        request.method(),
        &state.product_id,
        state.mode,
    )
    .await
    {
        return *response;
    }
    Json(state.handle.diagnostics().await).into_response()
}

async fn request_id_layer(mut request: Request, next: Next) -> Response {
    static HEADER: axum::http::HeaderName = axum::http::HeaderName::from_static("x-request-id");
    let request_id = match request
        .headers()
        .get_all(&HEADER)
        .iter()
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => new_request_id(),
        [value] => match value.to_str() {
            Ok(value) if !value.is_empty() && value.len() <= 128 && value.is_ascii() => {
                value.to_owned()
            }
            _ => return StatusCode::BAD_REQUEST.into_response(),
        },
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    request.extensions_mut().insert(request_id.clone());
    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(HEADER.clone(), value);
    }
    response
}

pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| eprintln!("fatal runtime panic: {info}")));
}

pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate =
            signal(SignalKind::terminate()).expect("SIGTERM handler installation failed");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("shutdown handler failed");
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid product descriptor field: {0}")]
    InvalidDescriptor(&'static str),
    #[error("duplicate runtime registration: {0}")]
    DuplicateName(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Ready;
    #[async_trait]
    impl HealthCheck for Ready {
        async fn check(&self) -> bool {
            true
        }
    }
    fn descriptor() -> ProductDescriptor {
        ProductDescriptor {
            id: "example".into(),
            version: "1.0.0".into(),
            foundation_revision: "0123456789abcdef".into(),
            profile: "server-control-plane".into(),
            capabilities: vec!["server-runtime".into()],
        }
    }

    #[tokio::test]
    async fn readiness_requires_checks_and_critical_exit_stops_runtime() {
        let runtime = ServerRuntime::builder(descriptor())
            .register_health_check("database", Arc::new(Ready))
            .register_background_task("worker", TaskCriticality::Critical, |_shutdown| async {
                Err("stopped".into())
            })
            .build()
            .await
            .unwrap();
        runtime.refresh_health().await;
        assert!(runtime.handle().health().await.ready);
        let handle = runtime.handle();
        runtime.run_until_shutdown().await;
        let health = handle.health().await;
        assert!(!health.live && !health.ready);
    }

    #[test]
    fn request_ids_are_bounded_random_hex() {
        let id = new_request_id();
        assert_eq!(id.len(), 32);
        assert!(id.bytes().all(|b| b.is_ascii_hexdigit()));
    }
}
