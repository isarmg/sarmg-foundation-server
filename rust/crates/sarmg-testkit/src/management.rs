use super::*;
use sarmg_contracts::{ADMINISTRATORS_PATH, AdministratorSummary};

/// Run on a fresh persistent store containing only the standard test account.
pub async fn assert_administrator_management_http_contract<F, Fut>(send: F)
where
    F: Fn(Request<Body>) -> Fut,
    Fut: Future<Output = Response>,
{
    assert_error(
        send(request(Method::GET, ADMINISTRATORS_PATH, "")).await,
        StatusCode::UNAUTHORIZED,
        "auth.session_required",
    )
    .await;
    let login = send(request(Method::POST, ADMIN_LOGIN_PATH, LOGIN_JSON)).await;
    assert_eq!(login.status(), StatusCode::OK);
    let cookie = login.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let session: AdministratorSession =
        serde_json::from_slice(&to_bytes(login.into_body(), 8192).await.unwrap()).unwrap();
    let authorized = |method: Method, path: &str, body: &str| {
        let mut request = request(method, path, body.to_owned());
        request
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        request.headers_mut().insert(
            "x-csrf-token",
            HeaderValue::from_str(&session.csrf_token).unwrap(),
        );
        request
    };
    let create = r#"{"username":"secondary","password":"another correct password"}"#;
    let mut no_csrf = authorized(Method::POST, ADMINISTRATORS_PATH, create);
    no_csrf.headers_mut().remove("x-csrf-token");
    assert_error(
        send(no_csrf).await,
        StatusCode::FORBIDDEN,
        "auth.csrf_rejected",
    )
    .await;
    for name in [
        header::ORIGIN,
        header::HOST,
        header::COOKIE,
        header::HeaderName::from_static("x-csrf-token"),
    ] {
        let mut duplicate = authorized(Method::POST, ADMINISTRATORS_PATH, create);
        let value = duplicate.headers()[&name].clone();
        duplicate.headers_mut().append(name.clone(), value);
        let (status, code) = if name == header::COOKIE {
            (StatusCode::UNAUTHORIZED, "auth.session_required")
        } else if name.as_str() == "x-csrf-token" {
            (StatusCode::FORBIDDEN, "auth.csrf_rejected")
        } else {
            (StatusCode::FORBIDDEN, "auth.origin_rejected")
        };
        assert_error(send(duplicate).await, status, code).await;
    }
    for body in [
        r#"{"username":"secondary","password":"another correct password","role":"admin"}"#,
        r#"{"username":"secondary","username":"other","password":"another correct password"}"#,
    ] {
        assert_error(
            send(authorized(Method::POST, ADMINISTRATORS_PATH, body)).await,
            StatusCode::BAD_REQUEST,
            "admin.invalid_request",
        )
        .await;
    }
    assert_error(
        send(authorized(
            Method::POST,
            ADMINISTRATORS_PATH,
            &"x".repeat(16385),
        ))
        .await,
        StatusCode::PAYLOAD_TOO_LARGE,
        "admin.body_too_large",
    )
    .await;
    let mut wrong_type = authorized(Method::POST, ADMINISTRATORS_PATH, create);
    wrong_type
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    assert_error(
        send(wrong_type).await,
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "admin.content_type_required",
    )
    .await;
    let root_disable = format!("{ADMINISTRATORS_PATH}/{}/disable", session.user_id);
    let response = send(authorized(Method::POST, &root_disable, "")).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let envelope: ErrorEnvelope =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(envelope.code.as_str(), "admin.last_administrator");
    assert_eq!(envelope.request_id.unwrap().as_str(), REQUEST_ID);
    assert_eq!(
        send(authorized(Method::POST, ADMINISTRATORS_PATH, create))
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_error(
        send(authorized(Method::POST, ADMINISTRATORS_PATH, create)).await,
        StatusCode::CONFLICT,
        "admin.conflict",
    )
    .await;
    for query in [
        "?limit=101",
        "?limit=0",
        "?limit=1&limit=2",
        "?extra=1",
        "?offset=-1",
    ] {
        assert_error(
            send(authorized(
                Method::GET,
                &format!("{ADMINISTRATORS_PATH}{query}"),
                "",
            ))
            .await,
            StatusCode::BAD_REQUEST,
            "admin.invalid_request",
        )
        .await;
    }
    let listed = send(authorized(
        Method::GET,
        &format!("{ADMINISTRATORS_PATH}?limit=1&offset=1"),
        "",
    ))
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    assert!(
        listed.headers()[header::CACHE_CONTROL]
            .to_str()
            .unwrap()
            .contains("no-store")
    );
    let records: Vec<AdministratorSummary> =
        serde_json::from_slice(&to_bytes(listed.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].username, "secondary");
    let secondary = &records[0].administrator_id;
    let password_path = format!("{ADMINISTRATORS_PATH}/{secondary}/password");
    assert_eq!(
        send(authorized(
            Method::POST,
            &password_path,
            r#"{"password":"replacement correct password"}"#
        ))
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    let secondary_login = send(request(
        Method::POST,
        ADMIN_LOGIN_PATH,
        r#"{"username":"secondary","password":"replacement correct password"}"#,
    ))
    .await;
    assert_eq!(secondary_login.status(), StatusCode::OK);
    let secondary_cookie = secondary_login.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    assert_eq!(
        send(authorized(
            Method::POST,
            &format!("{ADMINISTRATORS_PATH}/{secondary}/disable"),
            ""
        ))
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    let mut revoked = request(Method::GET, ADMINISTRATORS_PATH, "");
    revoked.headers_mut().insert(
        header::COOKIE,
        HeaderValue::from_str(&secondary_cookie).unwrap(),
    );
    assert_error(
        send(revoked).await,
        StatusCode::UNAUTHORIZED,
        "auth.session_required",
    )
    .await;
    assert_eq!(
        send(authorized(
            Method::POST,
            &format!("{ADMINISTRATORS_PATH}/{}/password", session.user_id),
            r#"{"password":"replacement correct password"}"#
        ))
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_error(
        send(authorized(Method::GET, ADMINISTRATORS_PATH, "")).await,
        StatusCode::UNAUTHORIZED,
        "auth.session_required",
    )
    .await;
}
