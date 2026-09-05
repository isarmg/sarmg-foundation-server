//! Hyper integration for the exact administrator semantics implemented by the
//! Foundation Axum router. The wrapper adapts only body and peer-address types;
//! it intentionally owns no second authentication handler implementation.

use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, Response},
};
use bytes::Bytes;
use http_body::Body as HttpBody;
use sarmg_admin_auth::AdministratorOriginMode;
pub use sarmg_admin_axum::{FoundationErrorResponse, VerifiedAdministrator, authenticate_request};
use sarmg_admin_core::{AdministratorService, AdministratorStore};
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceExt;

#[derive(Clone, Debug)]
pub struct HyperAdministratorRouter {
    router: Router,
}

impl HyperAdministratorRouter {
    pub fn new<Store>(
        product_id: impl Into<String>,
        mode: AdministratorOriginMode,
        service: Arc<AdministratorService<Store>>,
    ) -> Result<Self, sarmg_admin_core::Error>
    where
        Store: AdministratorStore + 'static,
    {
        Ok(Self {
            router: sarmg_admin_axum::administrator_router(product_id, mode, service)?,
        })
    }

    pub fn owns_path(path: &str) -> bool {
        path == sarmg_contracts::ADMINISTRATORS_PATH
            || path
                .strip_prefix(sarmg_contracts::ADMINISTRATORS_PATH)
                .is_some_and(|tail| tail.starts_with('/'))
            || matches!(
                path,
                sarmg_contracts::ADMIN_LOGIN_PATH
                    | sarmg_contracts::ADMIN_SESSION_PATH
                    | sarmg_contracts::ADMIN_LOGOUT_PATH
            )
    }

    pub async fn handle_incoming(
        &self,
        request: Request<hyper::body::Incoming>,
        peer: SocketAddr,
    ) -> Response<Body> {
        self.handle(request, peer).await
    }

    pub async fn handle<InputBody>(
        &self,
        mut request: Request<InputBody>,
        peer: SocketAddr,
    ) -> Response<Body>
    where
        InputBody: HttpBody<Data = Bytes> + Send + 'static,
        InputBody::Error: Into<axum::BoxError>,
    {
        request.extensions_mut().insert(ConnectInfo(peer));
        let request = request.map(Body::new);
        match self.router.clone().oneshot(request).await {
            Ok(response) => response,
            Err(error) => match error {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, StatusCode, header};
    use sarmg_admin_core::{AdministratorRecord, Identifier};
    use sarmg_admin_static::StaticAdministratorStore;

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
        let router = HyperAdministratorRouter::new(
            "example-product",
            AdministratorOriginMode::LoopbackDevelopmentHttp,
            service,
        )
        .unwrap();
        sarmg_testkit::assert_administrator_management_http_contract(|request| {
            let router = router.clone();
            assert!(HyperAdministratorRouter::owns_path(request.uri().path()));
            async move {
                router
                    .handle(request, SocketAddr::from(([127, 0, 0, 1], 1234)))
                    .await
            }
        })
        .await;
    }

    #[tokio::test]
    async fn shared_administrator_wire_contract() {
        let store = StaticAdministratorStore::new([AdministratorRecord {
            administrator_id: Identifier::new("administrator-1").unwrap(),
            username: sarmg_testkit::ADMINISTRATOR_USERNAME.into(),
            password_hash: sarmg_admin_auth::hash_password(sarmg_testkit::ADMINISTRATOR_PASSWORD)
                .unwrap(),
            active: true,
            session_version: 1,
            created_at_micros: 1,
            updated_at_micros: 1,
            last_login_at_micros: None,
        }])
        .unwrap();
        let router = HyperAdministratorRouter::new(
            "example-product",
            AdministratorOriginMode::LoopbackDevelopmentHttp,
            Arc::new(AdministratorService::new(store)),
        )
        .unwrap();
        sarmg_testkit::assert_administrator_http_contract(
            |request| {
                let router = router.clone();
                async move {
                    router
                        .handle(request, SocketAddr::from(([127, 0, 0, 1], 1234)))
                        .await
                }
            },
            "example-product",
        )
        .await;
    }

    #[tokio::test]
    async fn hyper_body_path_uses_the_shared_router() {
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
        let router = HyperAdministratorRouter::new(
            "dufs-ram",
            AdministratorOriginMode::LoopbackDevelopmentHttp,
            Arc::new(AdministratorService::new(store)),
        )
        .unwrap();
        let request = Request::builder()
            .method(Method::POST)
            .uri(sarmg_contracts::ADMIN_LOGIN_PATH)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ORIGIN, "http://127.0.0.1")
            .header(header::HOST, "127.0.0.1")
            .header("sec-fetch-site", "same-origin")
            .body(Body::from(
                r#"{"username":"admin","password":"correct horse battery"}"#,
            ))
            .unwrap();
        let response = router
            .handle(request, SocketAddr::from(([127, 0, 0, 1], 1234)))
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response
                .headers()
                .get(header::SET_COOKIE)
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("sarmg-dufs-ram-session=")
        );
        assert!(HyperAdministratorRouter::owns_path(
            sarmg_contracts::ADMIN_LOGOUT_PATH
        ));
        assert!(!HyperAdministratorRouter::owns_path("/api/v2/files"));
    }
}
