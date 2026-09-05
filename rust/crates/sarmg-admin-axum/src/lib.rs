//! Axum wire adapter for the Foundation-owned administrator endpoints.

mod body;
mod management;

use axum::{
    Router,
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sarmg_admin_auth::AdministratorOriginMode;
use sarmg_admin_core::{AdministratorService, AdministratorStore, ServiceError};
use sarmg_contracts::{
    ADMIN_LOGIN_PATH, ADMIN_LOGOUT_PATH, ADMIN_SESSION_PATH, AdministratorLoginRequest, ErrorCode,
    ErrorEnvelope, RequestId,
};
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

/// Identifies an already-normalized platform response to outer product layers.
#[derive(Clone, Copy, Debug)]
pub struct FoundationErrorResponse;

/// In-process access-log metadata produced only after successful authentication.
/// Never contains a credential and is not serialized into response headers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedAdministrator {
    username: String,
}

impl VerifiedAdministrator {
    pub fn username(&self) -> &str {
        &self.username
    }

    fn attach(username: String, mut response: Response) -> Response {
        response.extensions_mut().insert(Self { username });
        response
    }
}
const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

struct AdapterState<Store> {
    product_id: String,
    mode: AdministratorOriginMode,
    service: Arc<AdministratorService<Store>>,
    body_admission: body::BodyAdmission,
}

impl<Store> Clone for AdapterState<Store> {
    fn clone(&self) -> Self {
        Self {
            product_id: self.product_id.clone(),
            mode: self.mode,
            service: Arc::clone(&self.service),
            body_admission: self.body_admission.clone(),
        }
    }
}

pub fn administrator_router<Store>(
    product_id: impl Into<String>,
    mode: AdministratorOriginMode,
    service: Arc<AdministratorService<Store>>,
) -> Result<Router, sarmg_admin_core::Error>
where
    Store: AdministratorStore + 'static,
{
    let product_id = product_id.into();
    sarmg_admin_core::session_cookie_name(&product_id, mode)?;
    let state = AdapterState {
        product_id,
        mode,
        service,
        body_admission: body::BodyAdmission::default(),
    };
    let router = Router::new()
        .route(ADMIN_LOGIN_PATH, post(login::<Store>))
        .route(ADMIN_SESSION_PATH, get(session::<Store>))
        .route(ADMIN_LOGOUT_PATH, post(logout::<Store>));
    let router = if state.service.store().supports_management() {
        management::routes(router)
    } else {
        router
    };
    Ok(router.with_state(state))
}

pub async fn authenticate_request<Store>(
    service: &AdministratorService<Store>,
    headers: &HeaderMap,
    uri: &Uri,
    method: &Method,
    product_id: &str,
    mode: AdministratorOriginMode,
) -> Result<sarmg_admin_core::AuthenticatedIdentity, Box<Response>>
where
    Store: AdministratorStore + 'static,
{
    let id = request_id(headers)?;
    let token = session_cookie(headers, product_id, mode, id.as_ref())?;
    let now = now_micros()?;
    let identity = service
        .authenticate_session(&token, now)
        .await
        .map_err(|failure| Box::new(service_error(failure, id.as_ref())))?;
    if !matches!(
        *method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    ) {
        require_same_origin_with_uri(mode, headers, uri, id.as_ref())?;
        let values = header_values(headers, sarmg_admin_auth::CSRF_HEADER);
        sarmg_admin_auth::require_csrf_token_matches_hash(&values, &identity.csrf_hash).map_err(
            |_| {
                Box::new(error(
                    StatusCode::FORBIDDEN,
                    "auth.csrf_rejected",
                    false,
                    id.as_ref(),
                ))
            },
        )?;
    }
    Ok(identity)
}

async fn login<Store>(State(state): State<AdapterState<Store>>, request: Request) -> Response
where
    Store: AdministratorStore + 'static,
{
    let (parts, body) = request.into_parts();
    let request_id = match request_id(&parts.headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if let Err(response) =
        require_same_origin_with_uri(state.mode, &parts.headers, &parts.uri, request_id.as_ref())
    {
        return *response;
    }
    let source = match parts.extensions.get::<ConnectInfo<SocketAddr>>() {
        Some(value) => value.0.ip().to_canonical().to_string(),
        None => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "platform.peer_missing",
                false,
                request_id.as_ref(),
            );
        }
    };
    if let Err(response) = validate_login_cookie(
        &parts.headers,
        &state.product_id,
        state.mode,
        request_id.as_ref(),
    ) {
        return *response;
    }
    let bytes = match state
        .body_admission
        .read(
            Request::from_parts(parts, body),
            body::Scope::Login,
            true,
            request_id.as_ref(),
        )
        .await
    {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let input: AdministratorLoginRequest = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                "auth.invalid_request",
                false,
                request_id.as_ref(),
            );
        }
    };
    let now = match now_micros() {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let context = sarmg_admin_core::LoginContext {
        source,
        request_id: request_id.as_ref().map(ToString::to_string),
        now_micros: now,
    };
    match state
        .service
        .login(&input.username, &input.password, &context)
        .await
    {
        Ok(authenticated) => {
            let cookie = match sarmg_admin_core::session_set_cookie(
                &state.product_id,
                state.mode,
                &authenticated.session_token,
            ) {
                Ok(value) => value,
                Err(_) => {
                    return error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "platform.cookie_failed",
                        false,
                        request_id.as_ref(),
                    );
                }
            };
            VerifiedAdministrator::attach(
                authenticated.administrator.username.clone(),
                json_with_cookie(StatusCode::OK, &authenticated.administrator, &cookie),
            )
        }
        Err(failure) => service_error(failure, request_id.as_ref()),
    }
}

async fn session<Store>(State(state): State<AdapterState<Store>>, request: Request) -> Response
where
    Store: AdministratorStore + 'static,
{
    let request_id = match request_id(request.headers()) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let token = match session_cookie(
        request.headers(),
        &state.product_id,
        state.mode,
        request_id.as_ref(),
    ) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let now = match now_micros() {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.service.restore_session(&token, now).await {
        Ok(authenticated) => VerifiedAdministrator::attach(
            authenticated.administrator.username.clone(),
            no_store(axum::Json(authenticated.administrator).into_response()),
        ),
        Err(failure) => service_error(failure, request_id.as_ref()),
    }
}

async fn logout<Store>(State(state): State<AdapterState<Store>>, request: Request) -> Response
where
    Store: AdministratorStore + 'static,
{
    let request_id = match request_id(request.headers()) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let identity = match authenticate_request(
        &state.service,
        request.headers(),
        request.uri(),
        request.method(),
        &state.product_id,
        state.mode,
    )
    .await
    {
        Ok(identity) => identity,
        Err(response) => return *response,
    };
    let token = match session_cookie(
        request.headers(),
        &state.product_id,
        state.mode,
        request_id.as_ref(),
    ) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let now = match now_micros() {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .service
        .logout(&token, now, request_id.as_ref().map(ToString::to_string))
        .await
    {
        Ok(()) => {
            let cookie = match sarmg_admin_core::session_clear_cookie(&state.product_id, state.mode)
            {
                Ok(value) => value,
                Err(_) => {
                    return error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "platform.cookie_failed",
                        false,
                        request_id.as_ref(),
                    );
                }
            };
            let mut response = StatusCode::NO_CONTENT.into_response();
            match HeaderValue::from_str(&cookie) {
                Ok(value) => {
                    response.headers_mut().insert(header::SET_COOKIE, value);
                    VerifiedAdministrator::attach(identity.username, no_store(response))
                }
                Err(_) => error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "platform.cookie_failed",
                    false,
                    request_id.as_ref(),
                ),
            }
        }
        Err(failure) => service_error(failure, request_id.as_ref()),
    }
}

fn request_id(headers: &HeaderMap) -> Result<Option<RequestId>, Box<Response>> {
    let values: Vec<_> = headers.get_all(&REQUEST_ID_HEADER).iter().collect();
    match values.as_slice() {
        [] => Ok(None),
        [value] => {
            let value = value.to_str().map_err(|_| {
                Box::new(error(
                    StatusCode::BAD_REQUEST,
                    "request.invalid_id",
                    false,
                    None,
                ))
            })?;
            RequestId::new(value.to_owned()).map(Some).map_err(|_| {
                Box::new(error(
                    StatusCode::BAD_REQUEST,
                    "request.invalid_id",
                    false,
                    None,
                ))
            })
        }
        _ => Err(Box::new(error(
            StatusCode::BAD_REQUEST,
            "request.duplicate_id",
            false,
            None,
        ))),
    }
}

fn require_same_origin_with_uri(
    mode: AdministratorOriginMode,
    headers: &HeaderMap,
    uri: &Uri,
    request_id: Option<&RequestId>,
) -> Result<(), Box<Response>> {
    let mut hosts = header_values(headers, sarmg_admin_auth::HOST_HEADER);
    if hosts.is_empty() {
        if let Some(authority) = uri.authority() {
            hosts.push(authority.as_str().as_bytes().to_vec());
        }
    } else if uri.authority().is_some() {
        return Err(Box::new(error(
            StatusCode::FORBIDDEN,
            "auth.origin_rejected",
            false,
            request_id,
        )));
    }
    sarmg_admin_auth::require_administrator_same_origin(
        mode,
        &header_values(headers, sarmg_admin_auth::ORIGIN_HEADER),
        &hosts,
        &header_values(headers, sarmg_admin_auth::SEC_FETCH_SITE_HEADER),
    )
    .map(|_| ())
    .map_err(|_| {
        Box::new(error(
            StatusCode::FORBIDDEN,
            "auth.origin_rejected",
            false,
            request_id,
        ))
    })
}

fn header_values(headers: &HeaderMap, name: &'static str) -> Vec<Vec<u8>> {
    headers
        .get_all(name)
        .iter()
        .map(|value| value.as_bytes().to_vec())
        .collect()
}

fn validate_login_cookie(
    headers: &HeaderMap,
    product_id: &str,
    mode: AdministratorOriginMode,
    request_id: Option<&RequestId>,
) -> Result<(), Box<Response>> {
    let invalid = || {
        Box::new(error(
            StatusCode::BAD_REQUEST,
            "auth.invalid_cookie",
            false,
            request_id,
        ))
    };
    let values: Vec<_> = headers.get_all(header::COOKIE).iter().collect();
    if values.is_empty() {
        return Ok(());
    }
    let [value] = values.as_slice() else {
        return Err(invalid());
    };
    let header = value.to_str().map_err(|_| invalid())?;
    let name = sarmg_admin_core::session_cookie_name(product_id, mode).map_err(|_| {
        Box::new(error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "platform.cookie_failed",
            false,
            request_id,
        ))
    })?;
    let mut found = false;
    for part in header.split(';') {
        let (key, token) = part.trim().split_once('=').ok_or_else(invalid)?;
        if key == name {
            if found || !sarmg_admin_auth::is_token_shape(token) {
                return Err(invalid());
            }
            found = true;
        }
    }
    Ok(())
}

fn session_cookie(
    headers: &HeaderMap,
    product_id: &str,
    mode: AdministratorOriginMode,
    request_id: Option<&RequestId>,
) -> Result<String, Box<Response>> {
    let values: Vec<_> = headers.get_all(header::COOKIE).iter().collect();
    let [value] = values.as_slice() else {
        return Err(Box::new(error(
            StatusCode::UNAUTHORIZED,
            "auth.session_required",
            false,
            request_id,
        )));
    };
    let header = value.to_str().map_err(|_| {
        Box::new(error(
            StatusCode::UNAUTHORIZED,
            "auth.session_required",
            false,
            request_id,
        ))
    })?;
    let name = sarmg_admin_core::session_cookie_name(product_id, mode).map_err(|_| {
        Box::new(error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "platform.cookie_failed",
            false,
            request_id,
        ))
    })?;
    let token = sarmg_admin_auth::parse_cookie_value(header, &name)
        .filter(|value| sarmg_admin_auth::is_token_shape(value))
        .ok_or_else(|| {
            Box::new(error(
                StatusCode::UNAUTHORIZED,
                "auth.session_required",
                false,
                request_id,
            ))
        })?;
    Ok(token.to_owned())
}

fn now_micros() -> Result<u64, Box<Response>> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        Box::new(error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "platform.clock_failed",
            false,
            None,
        ))
    })?;
    u64::try_from(duration.as_micros()).map_err(|_| {
        Box::new(error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "platform.clock_failed",
            false,
            None,
        ))
    })
}

fn service_error<StoreError>(
    failure: ServiceError<StoreError>,
    request_id: Option<&RequestId>,
) -> Response
where
    StoreError: std::error::Error + Send + Sync + 'static,
{
    match failure {
        ServiceError::InvalidCredentials => error(
            StatusCode::UNAUTHORIZED,
            "auth.invalid_credentials",
            false,
            request_id,
        ),
        ServiceError::InvalidSession => error(
            StatusCode::UNAUTHORIZED,
            "auth.session_required",
            false,
            request_id,
        ),
        ServiceError::LoginRateLimited => {
            let mut response = error(
                StatusCode::TOO_MANY_REQUESTS,
                "auth.rate_limited",
                true,
                request_id,
            );
            // Conservatively wait one complete platform window; never advise immediate retries.
            let seconds = sarmg_admin_core::LOGIN_WINDOW_MICROS.div_ceil(1_000_000);
            response.headers_mut().insert(
                header::RETRY_AFTER,
                HeaderValue::from_str(&seconds.to_string())
                    .expect("policy duration is a valid header"),
            );
            response
        }
        ServiceError::AuthenticationBusy | ServiceError::AuthenticationWorkerFailed => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "auth.capacity_unavailable",
            true,
            request_id,
        ),
        ServiceError::Authentication(_) => error(
            StatusCode::FORBIDDEN,
            "auth.csrf_rejected",
            false,
            request_id,
        ),
        ServiceError::AdministratorNotFound
        | ServiceError::Policy(_)
        | ServiceError::Store(_)
        | ServiceError::Contract(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "platform.internal",
            false,
            request_id,
        ),
    }
}

fn json_with_cookie<T: serde::Serialize>(status: StatusCode, value: &T, cookie: &str) -> Response {
    let mut response = (status, axum::Json(value)).into_response();
    match HeaderValue::from_str(cookie) {
        Ok(value) => {
            response.headers_mut().insert(header::SET_COOKIE, value);
            no_store(response)
        }
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "platform.cookie_failed",
            false,
            None,
        ),
    }
}

fn error(
    status: StatusCode,
    code: &'static str,
    retryable: bool,
    request_id: Option<&RequestId>,
) -> Response {
    let code = ErrorCode::new(code).expect("static adapter error codes are canonical");
    let envelope = ErrorEnvelope {
        message: code.as_str().replace('.', " "),
        code,
        request_id: request_id.cloned(),
        retryable,
        details: Default::default(),
    };
    let mut response = no_store((status, axum::Json(envelope)).into_response());
    response.extensions_mut().insert(FoundationErrorResponse);
    response
}

fn no_store(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, private, max-age=0"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::{Method, Request as HttpRequest};
    use sarmg_admin_core::{AdministratorRecord, Identifier};
    use sarmg_admin_static::StaticAdministratorStore;
    use tower::ServiceExt;

    fn request(method: Method, path: &str, body: &str) -> HttpRequest<axum::body::Body> {
        let mut request = HttpRequest::builder()
            .method(method)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ORIGIN, "http://127.0.0.1")
            .header(header::HOST, "127.0.0.1")
            .header("sec-fetch-site", "same-origin")
            .body(axum::body::Body::from(body.to_owned()))
            .unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1234))));
        request
    }

    fn router() -> Router {
        let store = StaticAdministratorStore::new([AdministratorRecord {
            administrator_id: Identifier::new("administrator-1").unwrap(),
            username: "admin".into(),
            password_hash: sarmg_admin_auth::hash_password("correct horse battery").unwrap(),
            active: true,
            session_version: 1,
            created_at_micros: 1,
            updated_at_micros: 1,
            last_login_at_micros: None,
        }])
        .unwrap();
        administrator_router(
            "example-product",
            AdministratorOriginMode::LoopbackDevelopmentHttp,
            Arc::new(AdministratorService::new(store)),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn verified_access_log_metadata_survives_logout_without_credentials() {
        let router = router();
        let bad = router
            .clone()
            .oneshot(request(
                Method::POST,
                ADMIN_LOGIN_PATH,
                r#"{"username":"admin","password":"incorrect password"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);
        assert!(bad.extensions().get::<VerifiedAdministrator>().is_none());
        let login = router
            .clone()
            .oneshot(request(
                Method::POST,
                ADMIN_LOGIN_PATH,
                r#"{"username":"admin","password":"correct horse battery"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(login.status(), StatusCode::OK);
        let metadata = login.extensions().get::<VerifiedAdministrator>().unwrap();
        assert_eq!(metadata.username(), "admin");
        assert_eq!(
            format!("{metadata:?}"),
            "VerifiedAdministrator { username: \"admin\" }"
        );
        let cookie = login.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let data: sarmg_contracts::AdministratorSession =
            serde_json::from_slice(&to_bytes(login.into_body(), 4096).await.unwrap()).unwrap();
        let mut logout = request(Method::POST, ADMIN_LOGOUT_PATH, "");
        logout
            .headers_mut()
            .insert(header::COOKIE, cookie.parse().unwrap());
        logout.headers_mut().insert(
            sarmg_admin_auth::CSRF_HEADER,
            data.csrf_token.parse().unwrap(),
        );
        let response = router.clone().oneshot(logout).await.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            response
                .extensions()
                .get::<VerifiedAdministrator>()
                .unwrap()
                .username(),
            "admin"
        );
        let mut restore = request(Method::GET, ADMIN_SESSION_PATH, "");
        restore
            .headers_mut()
            .insert(header::COOKIE, cookie.parse().unwrap());
        let response = router.oneshot(restore).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(
            response
                .extensions()
                .get::<VerifiedAdministrator>()
                .is_none()
        );
    }

    #[tokio::test]
    async fn shared_persistent_management_contract() {
        let directory = tempfile::tempdir().unwrap();
        let pool = sarmg_sqlite::create_if_missing(
            directory.path().join("admin.sqlite3"),
            sarmg_sqlite::PoolOptions::new(2),
        )
        .await
        .unwrap();
        sqlx::raw_sql(sarmg_admin_sqlite::ADMIN_PERSISTENT_DDL)
            .execute(&pool)
            .await
            .unwrap();
        let service = Arc::new(AdministratorService::new(
            sarmg_admin_sqlite::SqliteAdministratorStore::new(pool),
        ));
        service
            .bootstrap_administrator("admin", sarmg_testkit::ADMINISTRATOR_PASSWORD, 1)
            .await
            .unwrap();
        let router = administrator_router(
            "example-product",
            AdministratorOriginMode::LoopbackDevelopmentHttp,
            service,
        )
        .unwrap();
        sarmg_testkit::assert_administrator_management_http_contract(|mut request| {
            let router = router.clone();
            request
                .extensions_mut()
                .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1234))));
            async move { router.oneshot(request).await.unwrap() }
        })
        .await;
    }

    #[tokio::test]
    async fn static_profile_has_no_management_routes() {
        let router = router();
        for (method, path) in [
            (
                Method::GET,
                sarmg_contracts::ADMINISTRATORS_PATH.to_string(),
            ),
            (
                Method::POST,
                sarmg_contracts::ADMINISTRATORS_PATH.to_string(),
            ),
            (
                Method::POST,
                format!("{}/admin/password", sarmg_contracts::ADMINISTRATORS_PATH),
            ),
            (
                Method::POST,
                format!("{}/admin/disable", sarmg_contracts::ADMINISTRATORS_PATH),
            ),
        ] {
            assert_eq!(
                router
                    .clone()
                    .oneshot(request(method, &path, ""))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::NOT_FOUND
            );
        }
    }

    #[tokio::test]
    async fn shared_administrator_wire_contract() {
        let application = router();
        sarmg_testkit::assert_administrator_http_contract(
            |mut request| {
                let application = application.clone();
                request
                    .extensions_mut()
                    .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1234))));
                async move { application.oneshot(request).await.unwrap() }
            },
            "example-product",
        )
        .await;
    }

    #[tokio::test]
    async fn rejects_duplicate_origin_and_unknown_json() {
        let application = router();
        let mut duplicate = request(
            Method::POST,
            ADMIN_LOGIN_PATH,
            r#"{"username":"admin","password":"correct horse battery"}"#,
        );
        duplicate
            .headers_mut()
            .append(header::ORIGIN, HeaderValue::from_static("http://127.0.0.1"));
        assert_eq!(
            application
                .clone()
                .oneshot(duplicate)
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );

        let unknown = request(
            Method::POST,
            ADMIN_LOGIN_PATH,
            r#"{"username":"admin","password":"correct horse battery","extra":true}"#,
        );
        assert_eq!(
            application.oneshot(unknown).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn successful_login_uses_foundation_cookie_and_contract() {
        let response = router()
            .oneshot(request(
                Method::POST,
                ADMIN_LOGIN_PATH,
                r#"{"username":" ADMIN ","password":"correct horse battery"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cookie.starts_with("sarmg-example-product-session="));
        assert!(cookie.ends_with("; Path=/; HttpOnly; SameSite=Strict"));
        let body = to_bytes(response.into_body(), 8 * 1024).await.unwrap();
        let session: sarmg_contracts::AdministratorSession = serde_json::from_slice(&body).unwrap();
        assert_eq!(session.username, "admin");
    }
}
