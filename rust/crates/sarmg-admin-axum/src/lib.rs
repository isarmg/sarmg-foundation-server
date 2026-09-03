//! Axum wire adapter for the Foundation-owned administrator endpoints.

use axum::{
    Router,
    body::to_bytes,
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

pub const MAX_LOGIN_BODY_BYTES: usize = 16 * 1024;
const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

struct AdapterState<Store> {
    product_id: String,
    mode: AdministratorOriginMode,
    service: Arc<AdministratorService<Store>>,
}

impl<Store> Clone for AdapterState<Store> {
    fn clone(&self) -> Self {
        Self {
            product_id: self.product_id.clone(),
            mode: self.mode,
            service: Arc::clone(&self.service),
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
    };
    Ok(Router::new()
        .route(ADMIN_LOGIN_PATH, post(login::<Store>))
        .route(ADMIN_SESSION_PATH, get(session::<Store>))
        .route(ADMIN_LOGOUT_PATH, post(logout::<Store>))
        .with_state(state))
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
    let token = session_cookie(headers, product_id, mode)?;
    let now = now_micros()?;
    let identity = service
        .authenticate_session(&token, now)
        .await
        .map_err(|failure| Box::new(service_error(failure, None)))?;
    if !matches!(
        *method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    ) {
        require_same_origin_with_uri(mode, headers, uri, None)?;
        let values = header_values(headers, sarmg_admin_auth::CSRF_HEADER);
        sarmg_admin_auth::require_csrf_token_matches_hash(&values, &identity.csrf_hash).map_err(
            |_| {
                Box::new(error(
                    StatusCode::FORBIDDEN,
                    "auth.csrf_rejected",
                    false,
                    None,
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
    if let Err(response) = require_same_origin(state.mode, &parts.headers, request_id.as_ref()) {
        return *response;
    }
    let source = match parts.extensions.get::<ConnectInfo<SocketAddr>>() {
        Some(value) => value.0.ip().to_string(),
        None => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "platform.peer_missing",
                false,
                request_id.as_ref(),
            );
        }
    };
    let bytes = match to_bytes(body, MAX_LOGIN_BODY_BYTES).await {
        Ok(value) => value,
        Err(_) => {
            return error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "auth.body_too_large",
                false,
                request_id.as_ref(),
            );
        }
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
            json_with_cookie(StatusCode::OK, &authenticated.administrator, &cookie)
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
    let token = match session_cookie(request.headers(), &state.product_id, state.mode) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let now = match now_micros() {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state.service.restore_session(&token, now).await {
        Ok(authenticated) => no_store(axum::Json(authenticated.administrator).into_response()),
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
    if let Err(response) = require_same_origin(state.mode, request.headers(), request_id.as_ref()) {
        return *response;
    }
    let token = match session_cookie(request.headers(), &state.product_id, state.mode) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let csrf_values = header_values(request.headers(), sarmg_admin_auth::CSRF_HEADER);
    let now = match now_micros() {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if let Err(failure) = state.service.require_csrf(&token, &csrf_values, now).await {
        return service_error(failure, request_id.as_ref());
    }
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
                    no_store(response)
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

fn require_same_origin(
    mode: AdministratorOriginMode,
    headers: &HeaderMap,
    request_id: Option<&RequestId>,
) -> Result<(), Box<Response>> {
    sarmg_admin_auth::require_administrator_same_origin(
        mode,
        &header_values(headers, sarmg_admin_auth::ORIGIN_HEADER),
        &header_values(headers, sarmg_admin_auth::HOST_HEADER),
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

fn session_cookie(
    headers: &HeaderMap,
    product_id: &str,
    mode: AdministratorOriginMode,
) -> Result<String, Box<Response>> {
    let values: Vec<_> = headers.get_all(header::COOKIE).iter().collect();
    let [value] = values.as_slice() else {
        return Err(Box::new(error(
            StatusCode::UNAUTHORIZED,
            "auth.session_required",
            false,
            None,
        )));
    };
    let header = value.to_str().map_err(|_| {
        Box::new(error(
            StatusCode::UNAUTHORIZED,
            "auth.session_required",
            false,
            None,
        ))
    })?;
    let name = sarmg_admin_core::session_cookie_name(product_id, mode).map_err(|_| {
        Box::new(error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "platform.cookie_failed",
            false,
            None,
        ))
    })?;
    let token = sarmg_admin_auth::parse_cookie_value(header, &name)
        .filter(|value| sarmg_admin_auth::is_token_shape(value))
        .ok_or_else(|| {
            Box::new(error(
                StatusCode::UNAUTHORIZED,
                "auth.session_required",
                false,
                None,
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
        ServiceError::LoginRateLimited => error(
            StatusCode::TOO_MANY_REQUESTS,
            "auth.rate_limited",
            true,
            request_id,
        ),
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
    no_store((status, axum::Json(envelope)).into_response())
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
