use super::*;
use axum::extract::{Path, Query, rejection::PathRejection};
use sarmg_admin_core::{AdministratorManagementContext, ManagementError};
use sarmg_contracts::{
    ADMINISTRATORS_PATH, AdministratorCreateRequest, AdministratorPasswordRequest,
};

pub(super) fn routes<Store: AdministratorStore + 'static>(
    router: Router<AdapterState<Store>>,
) -> Router<AdapterState<Store>> {
    router
        .route(
            ADMINISTRATORS_PATH,
            get(list::<Store>).post(create::<Store>),
        )
        .route(
            &format!("{ADMINISTRATORS_PATH}/{{id}}/password"),
            post(password::<Store>),
        )
        .route(
            &format!("{ADMINISTRATORS_PATH}/{{id}}/disable"),
            post(disable::<Store>),
        )
}

pub(super) fn account_routes<Store: AdministratorStore + 'static>(
    router: Router<AdapterState<Store>>,
) -> Router<AdapterState<Store>> {
    router.route(sarmg_contracts::ADMIN_ACCOUNT_PATH, post(account::<Store>))
}

async fn account<Store: AdministratorStore + 'static>(
    State(state): State<AdapterState<Store>>,
    request: Request,
) -> Response {
    let context = match authorize(&state, request.headers(), request.uri(), request.method()).await
    {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let id = context_id(&context);
    let input = match input::<sarmg_contracts::AdministratorAccountRequest>(
        request,
        &state.body_admission,
        id.as_ref(),
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return *response,
    };
    mutation_response(
        state
            .service
            .update_own_account(
                &context,
                &input.username,
                &input.current_password,
                input.new_password.as_deref(),
            )
            .await,
        id.as_ref(),
    )
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Pagination {
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(default)]
    offset: u64,
}
fn default_limit() -> u32 {
    50
}

async fn authorize<Store: AdministratorStore + 'static>(
    state: &AdapterState<Store>,
    headers: &HeaderMap,
    uri: &Uri,
    method: &Method,
) -> Result<AdministratorManagementContext, Box<Response>> {
    let id = request_id(headers)?;
    let identity = authenticate_request(
        &state.service,
        headers,
        uri,
        method,
        &state.product_id,
        state.mode,
    )
    .await?;
    Ok(AdministratorManagementContext {
        identity,
        request_id: id.map(|id| id.to_string()),
        now_micros: now_micros()?,
    })
}

fn context_id(context: &AdministratorManagementContext) -> Option<RequestId> {
    context
        .request_id
        .as_ref()
        .map(|id| RequestId::new(id.clone()).expect("authorization validated request id"))
}

async fn list<Store: AdministratorStore + 'static>(
    State(state): State<AdapterState<Store>>,
    request: Request,
) -> Response {
    let context = match authorize(&state, request.headers(), request.uri(), request.method()).await
    {
        Ok(context) => context,
        Err(response) => return *response,
    };
    let id = context_id(&context);
    let pagination = match Query::<Pagination>::try_from_uri(request.uri()) {
        Ok(Query(value)) => value,
        Err(_) => return invalid(id.as_ref()),
    };
    match state
        .service
        .list_administrators(pagination.limit, pagination.offset)
        .await
    {
        Ok(records) => {
            // Serialize explicitly so out-of-contract stored numbers cannot
            // become an unmarked framework serialization error response.
            match serde_json::to_value(records) {
                Ok(value) => no_store(axum::Json(value).into_response()),
                Err(_) => error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "platform.internal",
                    false,
                    id.as_ref(),
                ),
            }
        }
        Err(failure) => management_error(failure, id.as_ref()),
    }
}

async fn input<T: serde::de::DeserializeOwned>(
    request: Request,
    admission: &body::BodyAdmission,
    id: Option<&RequestId>,
) -> Result<T, Box<Response>> {
    let bytes = admission
        .read(request, body::Scope::Management, true, id)
        .await?;
    serde_json::from_slice(&bytes).map_err(|_| Box::new(invalid(id)))
}

async fn create<Store: AdministratorStore + 'static>(
    State(state): State<AdapterState<Store>>,
    request: Request,
) -> Response {
    let context = match authorize(&state, request.headers(), request.uri(), request.method()).await
    {
        Ok(context) => context,
        Err(response) => return *response,
    };
    let id = context_id(&context);
    let input = match input::<AdministratorCreateRequest>(
        request,
        &state.body_admission,
        id.as_ref(),
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return *response,
    };
    mutation_response(
        state
            .service
            .create_administrator(&context, &input.username, &input.password)
            .await,
        id.as_ref(),
    )
}

async fn password<Store: AdministratorStore + 'static>(
    State(state): State<AdapterState<Store>>,
    path: Result<Path<String>, PathRejection>,
    request: Request,
) -> Response {
    let context = match authorize(&state, request.headers(), request.uri(), request.method()).await
    {
        Ok(context) => context,
        Err(response) => return *response,
    };
    let id = context_id(&context);
    let Ok(Path(target)) = path else {
        return invalid(id.as_ref());
    };
    let input =
        match input::<AdministratorPasswordRequest>(request, &state.body_admission, id.as_ref())
            .await
        {
            Ok(value) => value,
            Err(response) => return *response,
        };
    mutation_response(
        state
            .service
            .set_administrator_password(&context, &target, &input.password)
            .await,
        id.as_ref(),
    )
}

async fn disable<Store: AdministratorStore + 'static>(
    State(state): State<AdapterState<Store>>,
    path: Result<Path<String>, PathRejection>,
    request: Request,
) -> Response {
    let context = match authorize(&state, request.headers(), request.uri(), request.method()).await
    {
        Ok(context) => context,
        Err(response) => return *response,
    };
    let id = context_id(&context);
    let Ok(Path(target)) = path else {
        return invalid(id.as_ref());
    };
    // This operation has no input fields. Require an empty body, never ignore it.
    match state
        .body_admission
        .read(request, body::Scope::Management, false, id.as_ref())
        .await
    {
        Ok(bytes) if bytes.is_empty() => {}
        Ok(_) => return invalid(id.as_ref()),
        Err(response) => return *response,
    }
    mutation_response(
        state.service.disable_administrator(&context, &target).await,
        id.as_ref(),
    )
}

fn invalid(id: Option<&RequestId>) -> Response {
    error(StatusCode::BAD_REQUEST, "admin.invalid_request", false, id)
}

fn mutation_response<E: std::error::Error + Send + Sync + 'static>(
    result: Result<(), ManagementError<E>>,
    id: Option<&RequestId>,
) -> Response {
    match result {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(failure) => management_error(failure, id),
    }
}

fn management_error<E: std::error::Error + Send + Sync + 'static>(
    failure: ManagementError<E>,
    id: Option<&RequestId>,
) -> Response {
    let (status, code, retryable) = match failure {
        ManagementError::Unsupported => (StatusCode::NOT_FOUND, "admin.unsupported", false),
        ManagementError::Unauthorized => (StatusCode::UNAUTHORIZED, "auth.session_required", false),
        ManagementError::NotFound => (StatusCode::NOT_FOUND, "admin.not_found", false),
        ManagementError::Conflict => (StatusCode::CONFLICT, "admin.conflict", false),
        ManagementError::LastAdministrator => {
            (StatusCode::CONFLICT, "admin.last_administrator", false)
        }
        ManagementError::InvalidInput => return invalid(id),
        ManagementError::InvalidCurrentPassword => (
            StatusCode::FORBIDDEN,
            "admin.current_password_invalid",
            false,
        ),
        ManagementError::Busy => (
            StatusCode::SERVICE_UNAVAILABLE,
            "auth.capacity_unavailable",
            true,
        ),
        ManagementError::Store(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "platform.internal",
            false,
        ),
    };
    error(status, code, retryable, id)
}
