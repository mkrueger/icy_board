use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    Form, Json, Router,
    extract::{ConnectInfo, Path as AxumPath, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    auth::{AuthState, Principal, SESSION_COOKIE, cookie_value},
    dto::*,
    error::AdminError,
    service::AdminBackend,
    ui::{self, Notice, SectionId},
};

#[derive(Clone)]
pub struct AppState {
    pub backend: Arc<dyn AdminBackend>,
    pub auth: Arc<AuthState>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(page_overview))
        .route("/settings/{section}", get(page_section).post(page_section_submit))
        .route("/settings", get(|| async { Redirect::to("/settings/general") }))
        .route("/conferences", get(page_conferences))
        .route("/conferences/new", get(page_conference_new).post(page_conference_create))
        .route("/conferences/{index}", get(page_conference).post(page_conference_submit))
        .route("/conferences/{index}/delete", post(page_conference_delete))
        .route("/login", get(page_login).post(login_submit))
        .route("/logout", post(logout_submit))
        .route("/style.css", get(stylesheet))
        .route("/api/health", get(api_health))
        .route("/api/overview", get(api_overview))
        .route("/api/settings/{section}", get(api_get_section).put(api_put_section))
        .route("/api/settings/{section}/preview", post(api_preview_section))
        .route("/api/conferences", get(api_list_conferences).post(api_create_conference))
        .route(
            "/api/conferences/{index}",
            get(api_get_conference).put(api_update_conference).delete(api_delete_conference),
        )
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

async fn security_headers(request: axum::extract::Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static("default-src 'none'; style-src 'self'; form-action 'self'; base-uri 'none'"),
    );
    response
}

fn authenticate(state: &AppState, headers: &HeaderMap) -> Option<Principal> {
    if let Some(token) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        return state.auth.verify_token(token.trim()).then_some(Principal::Token);
    }
    session_principal(state, headers)
}

fn session_principal(state: &AppState, headers: &HeaderMap) -> Option<Principal> {
    let cookies = headers.get(header::COOKIE).and_then(|v| v.to_str().ok())?;
    let id = cookie_value(cookies, SESSION_COOKIE)?;
    let csrf = state.auth.session_csrf(id)?;
    Some(Principal::Session { csrf })
}

fn session_id(headers: &HeaderMap) -> Option<String> {
    let cookies = headers.get(header::COOKIE).and_then(|v| v.to_str().ok())?;
    cookie_value(cookies, SESSION_COOKIE).map(|s| s.to_string())
}

fn check_csrf(principal: &Principal, presented: Option<&str>) -> bool {
    match principal {
        Principal::Token => true,
        Principal::Session { csrf } => presented.is_some_and(|p| AuthState::verify_csrf(csrf, p)),
    }
}

fn actor(principal: &Principal, addr: SocketAddr) -> String {
    let kind = match principal {
        Principal::Token => "token",
        Principal::Session { .. } => "session",
    };
    format!("{kind}@{}", addr.ip())
}

fn status_for(error: &AdminError) -> StatusCode {
    match error {
        AdminError::NotFound(_) | AdminError::Missing(_) => StatusCode::NOT_FOUND,
        AdminError::Validation(_) => StatusCode::BAD_REQUEST,
        AdminError::Conflict => StatusCode::CONFLICT,
        AdminError::Locked => StatusCode::LOCKED,
        AdminError::Load(_) | AdminError::Save(_) | AdminError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn json_error(error: &AdminError) -> Response {
    let body = match error {
        AdminError::Validation(details) => serde_json::json!({ "error": "validation failed", "details": details }),
        other => serde_json::json!({ "error": other.to_string() }),
    };
    (status_for(error), Json(body)).into_response()
}

fn csrf_header(headers: &HeaderMap) -> Option<String> {
    headers.get("x-csrf-token").and_then(|v| v.to_str().ok()).map(|s| s.to_string())
}

fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "authentication required" }))).into_response()
}

fn forbidden(message: &str) -> Response {
    (StatusCode::FORBIDDEN, Json(serde_json::json!({ "error": message }))).into_response()
}

fn parse_section(name: &str) -> Option<SectionId> {
    SectionId::from_slug(name)
}

async fn api_health() -> Response {
    Json(serde_json::json!({ "status": "ok", "service": "icbadmin", "version": env!("CARGO_PKG_VERSION") })).into_response()
}

async fn api_overview(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if authenticate(&state, &headers).is_none() {
        return unauthorized();
    }
    Json(state.backend.overview().await).into_response()
}

async fn api_get_section(State(state): State<AppState>, headers: HeaderMap, AxumPath(section): AxumPath<String>) -> Response {
    if authenticate(&state, &headers).is_none() {
        return unauthorized();
    }
    let Some(section) = parse_section(&section) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "unknown settings section" }))).into_response();
    };
    match get_section_json(&state, section).await {
        Ok(value) => Json(value).into_response(),
        Err(e) => json_error(&e),
    }
}

async fn api_preview_section(State(state): State<AppState>, headers: HeaderMap, AxumPath(section): AxumPath<String>, Json(body): Json<Value>) -> Response {
    let Some(principal) = authenticate(&state, &headers) else {
        return unauthorized();
    };
    if !check_csrf(&principal, csrf_header(&headers).as_deref()) {
        return forbidden("missing or invalid CSRF token");
    }
    let Some(section) = parse_section(&section) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "unknown settings section" }))).into_response();
    };
    match preview_section_json(&state, section, body).await {
        Ok(diff) => Json(diff).into_response(),
        Err(e) => json_error(&e),
    }
}

async fn api_put_section(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(section): AxumPath<String>,
    Json(body): Json<Value>,
) -> Response {
    let Some(principal) = authenticate(&state, &headers) else {
        return unauthorized();
    };
    if !check_csrf(&principal, csrf_header(&headers).as_deref()) {
        return forbidden("missing or invalid CSRF token");
    }
    let Some(section) = parse_section(&section) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "unknown settings section" }))).into_response();
    };
    match update_section_json(&state, section, body, &actor(&principal, addr)).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => json_error(&e),
    }
}

async fn get_section_json(state: &AppState, section: SectionId) -> Result<Value, AdminError> {
    Ok(match section {
        SectionId::General => serde_json::to_value(state.backend.get_general_settings().await?).unwrap(),
        SectionId::Messages => serde_json::to_value(state.backend.get_message_settings().await?).unwrap(),
        SectionId::FileTransfer => serde_json::to_value(state.backend.get_file_transfer_settings().await?).unwrap(),
        SectionId::SystemControl => serde_json::to_value(state.backend.get_system_control_settings().await?).unwrap(),
        SectionId::Switches => serde_json::to_value(state.backend.get_switches_settings().await?).unwrap(),
        SectionId::Limits => serde_json::to_value(state.backend.get_limits_settings().await?).unwrap(),
        SectionId::NewUser => serde_json::to_value(state.backend.get_new_user_settings().await?).unwrap(),
        SectionId::Events => serde_json::to_value(state.backend.get_event_settings().await?).unwrap(),
        SectionId::Subscription => serde_json::to_value(state.backend.get_subscription_settings().await?).unwrap(),
        SectionId::Connections => serde_json::to_value(state.backend.get_connection_settings().await?).unwrap(),
        SectionId::Paths => serde_json::to_value(state.backend.get_paths_settings().await?).unwrap(),
        SectionId::Accounting => serde_json::to_value(state.backend.get_accounting_settings().await?).unwrap(),
        SectionId::FunctionKeys => serde_json::to_value(state.backend.get_function_keys_settings().await?).unwrap(),
    })
}

fn extract_fingerprint(body: &Value) -> Result<String, AdminError> {
    body.get("fingerprint")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AdminError::Validation(vec!["fingerprint is required".into()]))
}

fn strip_fingerprint(mut body: Value) -> Value {
    if let Some(obj) = body.as_object_mut() {
        obj.remove("fingerprint");
    }
    body
}

async fn preview_section_json(state: &AppState, section: SectionId, body: Value) -> Result<DiffDto, AdminError> {
    let body = strip_fingerprint(body);
    Ok(match section {
        SectionId::General => state.backend.preview_general_settings(&serde_json::from_value(body).map_err(json_val)?).await?,
        SectionId::Messages => state.backend.preview_message_settings(&serde_json::from_value(body).map_err(json_val)?).await?,
        SectionId::FileTransfer => {
            state
                .backend
                .preview_file_transfer_settings(&serde_json::from_value(body).map_err(json_val)?)
                .await?
        }
        SectionId::SystemControl => {
            state
                .backend
                .preview_system_control_settings(&serde_json::from_value(body).map_err(json_val)?)
                .await?
        }
        SectionId::Switches => {
            state
                .backend
                .preview_switches_settings(&serde_json::from_value(body).map_err(json_val)?)
                .await?
        }
        SectionId::Limits => state.backend.preview_limits_settings(&serde_json::from_value(body).map_err(json_val)?).await?,
        SectionId::NewUser => {
            state
                .backend
                .preview_new_user_settings(&serde_json::from_value(body).map_err(json_val)?)
                .await?
        }
        SectionId::Events => state.backend.preview_event_settings(&serde_json::from_value(body).map_err(json_val)?).await?,
        SectionId::Subscription => {
            state
                .backend
                .preview_subscription_settings(&serde_json::from_value(body).map_err(json_val)?)
                .await?
        }
        SectionId::Connections => {
            state
                .backend
                .preview_connection_settings(&serde_json::from_value(body).map_err(json_val)?)
                .await?
        }
        SectionId::Paths => state.backend.preview_paths_settings(&serde_json::from_value(body).map_err(json_val)?).await?,
        SectionId::Accounting => {
            state
                .backend
                .preview_accounting_settings(&serde_json::from_value(body).map_err(json_val)?)
                .await?
        }
        SectionId::FunctionKeys => {
            state
                .backend
                .preview_function_keys_settings(&serde_json::from_value(body).map_err(json_val)?)
                .await?
        }
    })
}

async fn update_section_json(state: &AppState, section: SectionId, body: Value, actor: &str) -> Result<ApplyResultDto, AdminError> {
    let fingerprint = extract_fingerprint(&body)?;
    let body = strip_fingerprint(body);
    Ok(match section {
        SectionId::General => {
            state
                .backend
                .update_general_settings(&serde_json::from_value(body).map_err(json_val)?, &fingerprint, actor)
                .await?
        }
        SectionId::Messages => {
            state
                .backend
                .update_message_settings(&serde_json::from_value(body).map_err(json_val)?, &fingerprint, actor)
                .await?
        }
        SectionId::FileTransfer => {
            state
                .backend
                .update_file_transfer_settings(&serde_json::from_value(body).map_err(json_val)?, &fingerprint, actor)
                .await?
        }
        SectionId::SystemControl => {
            state
                .backend
                .update_system_control_settings(&serde_json::from_value(body).map_err(json_val)?, &fingerprint, actor)
                .await?
        }
        SectionId::Switches => {
            state
                .backend
                .update_switches_settings(&serde_json::from_value(body).map_err(json_val)?, &fingerprint, actor)
                .await?
        }
        SectionId::Limits => {
            state
                .backend
                .update_limits_settings(&serde_json::from_value(body).map_err(json_val)?, &fingerprint, actor)
                .await?
        }
        SectionId::NewUser => {
            state
                .backend
                .update_new_user_settings(&serde_json::from_value(body).map_err(json_val)?, &fingerprint, actor)
                .await?
        }
        SectionId::Events => {
            state
                .backend
                .update_event_settings(&serde_json::from_value(body).map_err(json_val)?, &fingerprint, actor)
                .await?
        }
        SectionId::Subscription => {
            state
                .backend
                .update_subscription_settings(&serde_json::from_value(body).map_err(json_val)?, &fingerprint, actor)
                .await?
        }
        SectionId::Connections => {
            state
                .backend
                .update_connection_settings(&serde_json::from_value(body).map_err(json_val)?, &fingerprint, actor)
                .await?
        }
        SectionId::Paths => {
            state
                .backend
                .update_paths_settings(&serde_json::from_value(body).map_err(json_val)?, &fingerprint, actor)
                .await?
        }
        SectionId::Accounting => {
            state
                .backend
                .update_accounting_settings(&serde_json::from_value(body).map_err(json_val)?, &fingerprint, actor)
                .await?
        }
        SectionId::FunctionKeys => {
            state
                .backend
                .update_function_keys_settings(&serde_json::from_value(body).map_err(json_val)?, &fingerprint, actor)
                .await?
        }
    })
}

fn json_val(err: serde_json::Error) -> AdminError {
    AdminError::Validation(vec![format!("invalid request body: {err}")])
}

async fn stylesheet() -> Response {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], ui::STYLESHEET).into_response()
}

async fn page_login(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if session_principal(&state, &headers).is_some() {
        return Redirect::to("/").into_response();
    }
    html(ui::login_page(None))
}

#[derive(Deserialize)]
struct LoginForm {
    token: String,
}

async fn login_submit(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    if !state.auth.verify_token(form.token.trim()) {
        log::warn!("icbadmin: rejected sign in attempt");
        return (
            StatusCode::UNAUTHORIZED,
            html_body(ui::login_page(Some(Notice::Failure("Invalid access token.".to_string())))),
        )
            .into_response();
    }
    let id = state.auth.create_session();
    let cookie = format!("{SESSION_COOKIE}={id}; Path=/; HttpOnly; SameSite=Strict");
    ([(header::SET_COOKIE, cookie)], Redirect::to("/")).into_response()
}

async fn logout_submit(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(id) = session_id(&headers) {
        state.auth.destroy_session(&id);
    }
    let cookie = format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
    ([(header::SET_COOKIE, cookie)], Redirect::to("/login")).into_response()
}

async fn page_overview(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if session_principal(&state, &headers).is_none() {
        return Redirect::to("/login").into_response();
    }
    html(ui::overview_page(&state.backend.overview().await))
}

async fn page_section(State(state): State<AppState>, headers: HeaderMap, AxumPath(section): AxumPath<String>) -> Response {
    let Some(Principal::Session { csrf }) = session_principal(&state, &headers) else {
        return Redirect::to("/login").into_response();
    };
    let Some(section) = parse_section(&section) else {
        return (StatusCode::NOT_FOUND, html_body(ui::error_page("Not found", "Unknown settings section."))).into_response();
    };
    render_section(&state, section, &csrf, None).await
}

async fn page_section_submit(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(section): AxumPath<String>,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    let Some(principal @ Principal::Session { .. }) = session_principal(&state, &headers) else {
        return Redirect::to("/login").into_response();
    };
    let Some(section) = parse_section(&section) else {
        return (StatusCode::NOT_FOUND, html_body(ui::error_page("Not found", "Unknown settings section."))).into_response();
    };
    let csrf_token = form.get("csrf").map(|s| s.as_str());
    if !check_csrf(&principal, csrf_token) {
        return (
            StatusCode::FORBIDDEN,
            html_body(ui::error_page("Rejected", "The form was submitted with an invalid CSRF token.")),
        )
            .into_response();
    }
    let Principal::Session { csrf } = &principal else {
        unreachable!("checked above");
    };

    let fingerprint = form.get("fingerprint").cloned().unwrap_or_default();
    match form_to_update(&state, section, &form, &fingerprint, &actor(&principal, addr)).await {
        Ok(result) if result.changed_fields.is_empty() => render_section(&state, section, csrf, Some(Notice::Success("No changes to save.".to_string()))).await,
        Ok(result) => {
            render_section(
                &state,
                section,
                csrf,
                Some(Notice::Success(format!(
                    "Saved {} setting(s). Backup: {}",
                    result.changed_fields.len(),
                    result.backup.as_deref().unwrap_or("-")
                ))),
            )
            .await
        }
        Err(e) => render_section(&state, section, csrf, Some(Notice::Failure(e.to_string()))).await,
    }
}

async fn form_to_update(
    state: &AppState,
    section: SectionId,
    form: &HashMap<String, String>,
    fingerprint: &str,
    actor: &str,
) -> Result<ApplyResultDto, AdminError> {
    match section {
        SectionId::General => {
            let patch = general_from_form(form);
            state.backend.update_general_settings(&patch, fingerprint, actor).await
        }
        SectionId::Messages => {
            let patch = message_from_form(form);
            state.backend.update_message_settings(&patch, fingerprint, actor).await
        }
        SectionId::FileTransfer => {
            let patch = file_transfer_from_form(form);
            state.backend.update_file_transfer_settings(&patch, fingerprint, actor).await
        }
        SectionId::SystemControl => {
            let patch = system_control_from_form(form);
            state.backend.update_system_control_settings(&patch, fingerprint, actor).await
        }
        SectionId::Switches => {
            let patch = switches_from_form(form);
            state.backend.update_switches_settings(&patch, fingerprint, actor).await
        }
        SectionId::Limits => {
            let patch = limits_from_form(form);
            state.backend.update_limits_settings(&patch, fingerprint, actor).await
        }
        SectionId::NewUser => {
            let patch = new_user_from_form(form);
            state.backend.update_new_user_settings(&patch, fingerprint, actor).await
        }
        SectionId::Events => {
            let patch = event_from_form(form);
            state.backend.update_event_settings(&patch, fingerprint, actor).await
        }
        SectionId::Subscription => {
            let patch = subscription_from_form(form);
            state.backend.update_subscription_settings(&patch, fingerprint, actor).await
        }
        SectionId::Connections => {
            let patch = connection_from_form(form);
            state.backend.update_connection_settings(&patch, fingerprint, actor).await
        }
        SectionId::Paths => {
            let patch = paths_from_form(form);
            state.backend.update_paths_settings(&patch, fingerprint, actor).await
        }
        SectionId::Accounting => {
            let patch = accounting_from_form(form);
            state.backend.update_accounting_settings(&patch, fingerprint, actor).await
        }
        SectionId::FunctionKeys => {
            let patch = function_keys_from_form(form);
            state.backend.update_function_keys_settings(&patch, fingerprint, actor).await
        }
    }
}

async fn render_section(state: &AppState, section: SectionId, csrf: &str, notice: Option<Notice>) -> Response {
    match section_page(state, section, csrf, notice).await {
        Ok(page) => html(page),
        Err(e) => (status_for(&e), html_body(ui::error_page(section.title(), &e.to_string()))).into_response(),
    }
}

async fn section_page(state: &AppState, section: SectionId, csrf: &str, notice: Option<Notice>) -> Result<String, AdminError> {
    Ok(match section {
        SectionId::General => ui::general_page(&state.backend.get_general_settings().await?, csrf, notice),
        SectionId::Messages => ui::message_page(&state.backend.get_message_settings().await?, csrf, notice),
        SectionId::FileTransfer => ui::file_transfer_page(&state.backend.get_file_transfer_settings().await?, csrf, notice),
        SectionId::SystemControl => ui::system_control_page(&state.backend.get_system_control_settings().await?, csrf, notice),
        SectionId::Switches => ui::switches_page(&state.backend.get_switches_settings().await?, csrf, notice),
        SectionId::Limits => ui::limits_page(&state.backend.get_limits_settings().await?, csrf, notice),
        SectionId::NewUser => ui::new_user_page(&state.backend.get_new_user_settings().await?, csrf, notice),
        SectionId::Events => ui::event_page(&state.backend.get_event_settings().await?, csrf, notice),
        SectionId::Subscription => ui::subscription_page(&state.backend.get_subscription_settings().await?, csrf, notice),
        SectionId::Connections => ui::connection_page(&state.backend.get_connection_settings().await?, csrf, notice),
        SectionId::Paths => ui::paths_page(&state.backend.get_paths_settings().await?, csrf, notice),
        SectionId::Accounting => ui::accounting_page(&state.backend.get_accounting_settings().await?, csrf, notice),
        SectionId::FunctionKeys => ui::function_keys_page(&state.backend.get_function_keys_settings().await?, csrf, notice),
    })
}

fn html(body: String) -> Response {
    html_body(body).into_response()
}

fn html_body(body: String) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], body)
}

fn checked(form: &HashMap<String, String>, name: &str) -> bool {
    matches!(form.get(name).map(|s| s.as_str()), Some("true") | Some("on") | Some("1"))
}

fn text(form: &HashMap<String, String>, name: &str) -> String {
    form.get(name).cloned().unwrap_or_default()
}

fn parse_u16(form: &HashMap<String, String>, name: &str, default: u16) -> u16 {
    text(form, name).parse().unwrap_or(default)
}

fn parse_u32(form: &HashMap<String, String>, name: &str, default: u32) -> u32 {
    text(form, name).parse().unwrap_or(default)
}

fn parse_u8(form: &HashMap<String, String>, name: &str, default: u8) -> u8 {
    text(form, name).parse().unwrap_or(default)
}

fn general_from_form(form: &HashMap<String, String>) -> GeneralSettingsDto {
    GeneralSettingsDto {
        board_name: text(form, "board_name"),
        location: text(form, "location"),
        operator: text(form, "operator"),
        notice: text(form, "notice"),
        capabilities: text(form, "capabilities"),
        date_format: text(form, "date_format"),
        num_nodes: parse_u16(form, "num_nodes", 1),
        allow_iemsi: checked(form, "allow_iemsi"),
        who_include_city: checked(form, "who_include_city"),
        who_show_alias: checked(form, "who_show_alias"),
        sysop_name: text(form, "sysop_name"),
        sysop_use_real_name: checked(form, "sysop_use_real_name"),
        sysop_require_password_to_exit: checked(form, "sysop_require_password_to_exit"),
        sysop_external_editor: text(form, "sysop_external_editor"),
        sysop_config_color_theme: text(form, "sysop_config_color_theme"),
        web_admin_enabled: checked(form, "web_admin_enabled"),
        web_admin_address: text(form, "web_admin_address"),
        web_admin_port: parse_u16(form, "web_admin_port", 8787),
        web_admin_allow_remote: checked(form, "web_admin_allow_remote"),
    }
}

fn message_from_form(form: &HashMap<String, String>) -> MessageSettingsDto {
    MessageSettingsDto {
        max_msg_lines: parse_u16(form, "max_msg_lines", 1),
        scan_all_mail_at_login: checked(form, "scan_all_mail_at_login"),
        disable_message_scan_prompt: checked(form, "disable_message_scan_prompt"),
        allow_esc_codes: checked(form, "allow_esc_codes"),
        allow_carbon_copy: checked(form, "allow_carbon_copy"),
        validate_to_name: checked(form, "validate_to_name"),
        default_quick_personal_scan: checked(form, "default_quick_personal_scan"),
        default_scan_all_selected_confs_at_login: checked(form, "default_scan_all_selected_confs_at_login"),
        prompt_to_read_mail: checked(form, "prompt_to_read_mail"),
        force_comments_to_main: checked(form, "force_comments_to_main"),
        update_last_read_pointer: checked(form, "update_last_read_pointer"),
    }
}

fn file_transfer_from_form(form: &HashMap<String, String>) -> FileTransferSettingsDto {
    FileTransferSettingsDto {
        disallow_batch_uploads: checked(form, "disallow_batch_uploads"),
        promote_to_batch_transfers: checked(form, "promote_to_batch_transfers"),
        upload_credit_time: parse_u32(form, "upload_credit_time", 0),
        upload_credit_bytes: parse_u32(form, "upload_credit_bytes", 0),
        verify_files_uploaded: checked(form, "verify_files_uploaded"),
        upload_descr_lines: parse_u8(form, "upload_descr_lines", 1),
        display_uploader: checked(form, "display_uploader"),
        disable_drive_size_check: checked(form, "disable_drive_size_check"),
        stop_uploads_free_space: parse_u32(form, "stop_uploads_free_space", 0),
    }
}

fn system_control_from_form(form: &HashMap<String, String>) -> SystemControlSettingsDto {
    SystemControlSettingsDto {
        disable_ns_logon: checked(form, "disable_ns_logon"),
        disable_full_record_updating: checked(form, "disable_full_record_updating"),
        allow_alias_change: checked(form, "allow_alias_change"),
        is_multi_lingual: checked(form, "is_multi_lingual"),
        is_closed_board: checked(form, "is_closed_board"),
        enforce_daily_time_limit: checked(form, "enforce_daily_time_limit"),
        allow_password_failure_comment: checked(form, "allow_password_failure_comment"),
        guard_logoff: checked(form, "guard_logoff"),
        password_storage_method: text(form, "password_storage_method"),
        confirm_caller_name: checked(form, "confirm_caller_name"),
        reread_sec_level_on_join: checked(form, "reread_sec_level_on_join"),
    }
}

fn switches_from_form(form: &HashMap<String, String>) -> SwitchesSettingsDto {
    SwitchesSettingsDto {
        default_graphics_at_login: checked(form, "default_graphics_at_login"),
        non_graphics: checked(form, "non_graphics"),
        exclude_local_calls_stats: checked(form, "exclude_local_calls_stats"),
        display_news_behavior: text(form, "display_news_behavior"),
        disable_registration_edits: checked(form, "disable_registration_edits"),
        disable_high_ascii_filter: checked(form, "disable_high_ascii_filter"),
        display_userinfo_at_login: checked(form, "display_userinfo_at_login"),
        force_intro_on_join: checked(form, "force_intro_on_join"),
        scan_new_blt: checked(form, "scan_new_blt"),
        capture_grp_chat_session: checked(form, "capture_grp_chat_session"),
        allow_handle_in_grpchat: checked(form, "allow_handle_in_grpchat"),
        give_user_password_to_doors: checked(form, "give_user_password_to_doors"),
        call_log: checked(form, "call_log"),
        page_bell: checked(form, "page_bell"),
        alarm: checked(form, "alarm"),
        log_caller_number: checked(form, "log_caller_number"),
        log_connect_string: checked(form, "log_connect_string"),
        log_security_level: checked(form, "log_security_level"),
    }
}

fn limits_from_form(form: &HashMap<String, String>) -> LimitsSettingsDto {
    LimitsSettingsDto {
        keyboard_timeout: parse_u16(form, "keyboard_timeout", 0),
        max_number_upload_descr_lines: parse_u16(form, "max_number_upload_descr_lines", 0),
        min_pwd_length: parse_u8(form, "min_pwd_length", 0),
        password_expire_days: parse_u16(form, "password_expire_days", 0),
        password_expire_warn_days: parse_u16(form, "password_expire_warn_days", 0),
        sysop_start: text(form, "sysop_start"),
        sysop_stop: text(form, "sysop_stop"),
    }
}

fn new_user_from_form(form: &HashMap<String, String>) -> NewUserSettingsDto {
    NewUserSettingsDto {
        sec_level: parse_u8(form, "sec_level", 0),
        new_user_groups: text(form, "new_user_groups"),
        allow_one_name_users: checked(form, "allow_one_name_users"),
        use_newask_and_builtin: checked(form, "use_newask_and_builtin"),
        ask_city_or_state: checked(form, "ask_city_or_state"),
        ask_address: checked(form, "ask_address"),
        ask_verification: checked(form, "ask_verification"),
        ask_business_phone: checked(form, "ask_business_phone"),
        ask_home_phone: checked(form, "ask_home_phone"),
        ask_comment: checked(form, "ask_comment"),
        ask_clr_msg: checked(form, "ask_clr_msg"),
        ask_xfer_protocol: checked(form, "ask_xfer_protocol"),
        ask_date_format: checked(form, "ask_date_format"),
        ask_fse: checked(form, "ask_fse"),
        ask_alias: checked(form, "ask_alias"),
        ask_gender: checked(form, "ask_gender"),
        ask_birthdate: checked(form, "ask_birthdate"),
        ask_email: checked(form, "ask_email"),
        ask_web_address: checked(form, "ask_web_address"),
        ask_use_short_descr: checked(form, "ask_use_short_descr"),
        auto_register_conferences: checked(form, "auto_register_conferences"),
    }
}

fn event_from_form(form: &HashMap<String, String>) -> EventSettingsDto {
    EventSettingsDto {
        enabled: checked(form, "enabled"),
        event_file: text(form, "event_file"),
        suspend_minutes: parse_u16(form, "suspend_minutes", 0),
        disallow_uploads: checked(form, "disallow_uploads"),
        minutes_uploads_disallowed: parse_u16(form, "minutes_uploads_disallowed", 0),
    }
}

fn subscription_from_form(form: &HashMap<String, String>) -> SubscriptionSettingsDto {
    SubscriptionSettingsDto {
        is_enabled: checked(form, "is_enabled"),
        subscription_length: parse_u32(form, "subscription_length", 0),
        default_expired_level: parse_u8(form, "default_expired_level", 0),
        warning_days: parse_u32(form, "warning_days", 0),
    }
}

fn connection_from_form(form: &HashMap<String, String>) -> ConnectionSettingsDto {
    ConnectionSettingsDto {
        telnet: ListenerDto {
            is_enabled: checked(form, "telnet_is_enabled"),
            port: parse_u16(form, "telnet_port", 23),
            address: text(form, "telnet_address"),
            display_file: text(form, "telnet_display_file"),
        },
        ssh: ListenerDto {
            is_enabled: checked(form, "ssh_is_enabled"),
            port: parse_u16(form, "ssh_port", 22),
            address: text(form, "ssh_address"),
            display_file: text(form, "ssh_display_file"),
        },
        secure_websocket: SecureWebsocketDto {
            is_enabled: checked(form, "wss_is_enabled"),
            port: parse_u16(form, "wss_port", 8811),
            address: text(form, "wss_address"),
            display_file: text(form, "wss_display_file"),
            cert_pem: text(form, "wss_cert_pem"),
            key_pem: text(form, "wss_key_pem"),
        },
    }
}

fn paths_from_form(form: &HashMap<String, String>) -> PathsSettingsDto {
    PathsSettingsDto {
        help_path: text(form, "help_path"),
        security_file_path: text(form, "security_file_path"),
        email_msgbase: text(form, "email_msgbase"),
        command_display_path: text(form, "command_display_path"),
        tmp_work_path: text(form, "tmp_work_path"),
        icbtext: text(form, "icbtext"),
        conferences: text(form, "conferences"),
        welcome: text(form, "welcome"),
        newuser: text(form, "newuser"),
        closed: text(form, "closed"),
        expire_warning: text(form, "expire_warning"),
        expired: text(form, "expired"),
        conf_join_menu: text(form, "conf_join_menu"),
        chat_intro_file: text(form, "chat_intro_file"),
        chat_menu: text(form, "chat_menu"),
        chat_actions_menu: text(form, "chat_actions_menu"),
        no_ansi: text(form, "no_ansi"),
        trashcan_upload_files: text(form, "trashcan_upload_files"),
        trashcan_user: text(form, "trashcan_user"),
        trashcan_email: text(form, "trashcan_email"),
        trashcan_passwords: text(form, "trashcan_passwords"),
        vip_users: text(form, "vip_users"),
        protocol_data_file: text(form, "protocol_data_file"),
        pwrd_sec_level_file: text(form, "pwrd_sec_level_file"),
        command_file: text(form, "command_file"),
        statistics_file: text(form, "statistics_file"),
        language_file: text(form, "language_file"),
        group_file: text(form, "group_file"),
        ftn_file: text(form, "ftn_file"),
        user_file: text(form, "user_file"),
        caller_log: text(form, "caller_log"),
        transfer_log: text(form, "transfer_log"),
        logon_survey: text(form, "logon_survey"),
        logon_answer: text(form, "logon_answer"),
        logoff_survey: text(form, "logoff_survey"),
        logoff_answer: text(form, "logoff_answer"),
        newask_survey: text(form, "newask_survey"),
        newask_answer: text(form, "newask_answer"),
    }
}

fn accounting_from_form(form: &HashMap<String, String>) -> AccountingSettingsDto {
    AccountingSettingsDto {
        enabled: checked(form, "enabled"),
        use_money: checked(form, "use_money"),
        concurrent_tracking: checked(form, "concurrent_tracking"),
        ignore_empty_sec_level: checked(form, "ignore_empty_sec_level"),
        peak_usage_start: text(form, "peak_usage_start"),
        peak_usage_end: text(form, "peak_usage_end"),
        peak_days_of_week: text(form, "peak_days_of_week"),
        peak_holiday_list_file: text(form, "peak_holiday_list_file"),
        cfg_file: text(form, "cfg_file"),
        tracking_file: text(form, "tracking_file"),
        info_file: text(form, "info_file"),
        warning_file: text(form, "warning_file"),
        logoff_file: text(form, "logoff_file"),
    }
}

fn function_keys_from_form(form: &HashMap<String, String>) -> FunctionKeysSettingsDto {
    let keys = std::array::from_fn(|i| text(form, &format!("f{}", i + 1)));
    FunctionKeysSettingsDto { keys }
}

// ---------------------------------------------------------------- conferences

fn parse_i32(form: &HashMap<String, String>, name: &str, default: i32) -> i32 {
    text(form, name).trim().parse().unwrap_or(default)
}

fn parse_f64(form: &HashMap<String, String>, name: &str, default: f64) -> f64 {
    text(form, name).trim().parse().unwrap_or(default)
}

fn conference_from_form(form: &HashMap<String, String>) -> ConferenceDto {
    ConferenceDto {
        name: text(form, "name"),
        conference_type: text(form, "conference_type"),
        is_public: checked(form, "is_public"),
        is_read_only: checked(form, "is_read_only"),
        echo_mail_in_conference: checked(form, "echo_mail_in_conference"),
        force_echomail: checked(form, "force_echomail"),
        auto_rejoin: checked(form, "auto_rejoin"),
        allow_view_conf_members: checked(form, "allow_view_conf_members"),
        private_uploads: checked(form, "private_uploads"),
        private_msgs: checked(form, "private_msgs"),
        disallow_private_msgs: checked(form, "disallow_private_msgs"),
        allow_aliases: checked(form, "allow_aliases"),
        show_intro_in_scan: checked(form, "show_intro_in_scan"),
        use_main_commands: checked(form, "use_main_commands"),
        record_origin: checked(form, "record_origin"),
        prompt_for_routing: checked(form, "prompt_for_routing"),
        long_to_names: checked(form, "long_to_names"),
        required_security: text(form, "required_security"),
        sec_attachments: text(form, "sec_attachments"),
        sec_write_message: text(form, "sec_write_message"),
        sec_request_rr: text(form, "sec_request_rr"),
        sec_carbon_copy: text(form, "sec_carbon_copy"),
        carbon_list_limit: parse_u8(form, "carbon_list_limit", 0),
        add_conference_security: parse_i32(form, "add_conference_security", 0),
        add_conference_time: parse_u16(form, "add_conference_time", 0),
        pub_upload_sort: parse_u8(form, "pub_upload_sort", 0),
        private_upload_sort: parse_u8(form, "private_upload_sort", 0),
        charge_time: parse_f64(form, "charge_time", 0.0),
        charge_msg_read: parse_f64(form, "charge_msg_read", 0.0),
        charge_msg_write: parse_f64(form, "charge_msg_write", 0.0),
        users_menu: text(form, "users_menu"),
        sysop_menu: text(form, "sysop_menu"),
        news_file: text(form, "news_file"),
        intro_file: text(form, "intro_file"),
        attachment_location: text(form, "attachment_location"),
        command_file: text(form, "command_file"),
        pub_upload_location: text(form, "pub_upload_location"),
        pub_upload_metadata: text(form, "pub_upload_metadata"),
        private_upload_location: text(form, "private_upload_location"),
        private_upload_metadata: text(form, "private_upload_metadata"),
        doors_menu: text(form, "doors_menu"),
        doors_file: text(form, "doors_file"),
        blt_menu: text(form, "blt_menu"),
        blt_file: text(form, "blt_file"),
        survey_menu: text(form, "survey_menu"),
        survey_file: text(form, "survey_file"),
        dir_menu: text(form, "dir_menu"),
        dir_file: text(form, "dir_file"),
        area_menu: text(form, "area_menu"),
        area_file: text(form, "area_file"),
        new_password: text(form, "new_password"),
        clear_password: checked(form, "clear_password"),
    }
}

async fn render_conference_list(state: &AppState, csrf: &str, notice: Option<Notice>) -> Response {
    match state.backend.list_conferences().await {
        Ok(list) => html(ui::conference_list_page(&list, csrf, notice)),
        Err(e) => (status_for(&e), html_body(ui::error_page("Conferences", &e.to_string()))).into_response(),
    }
}

async fn render_conference(state: &AppState, index: usize, csrf: &str, notice: Option<Notice>) -> Response {
    match state.backend.get_conference(index).await {
        Ok(conf) => html(ui::conference_page(&conf, csrf, notice)),
        Err(e) => (status_for(&e), html_body(ui::error_page("Conference", &e.to_string()))).into_response(),
    }
}

async fn page_conferences(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(Principal::Session { csrf }) = session_principal(&state, &headers) else {
        return Redirect::to("/login").into_response();
    };
    render_conference_list(&state, &csrf, None).await
}

async fn page_conference(State(state): State<AppState>, headers: HeaderMap, AxumPath(index): AxumPath<usize>) -> Response {
    let Some(Principal::Session { csrf }) = session_principal(&state, &headers) else {
        return Redirect::to("/login").into_response();
    };
    render_conference(&state, index, &csrf, None).await
}

async fn page_conference_new(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(Principal::Session { csrf }) = session_principal(&state, &headers) else {
        return Redirect::to("/login").into_response();
    };
    match state.backend.list_conferences().await {
        Ok(list) => html(ui::conference_new_page(&list, csrf.as_str(), None)),
        Err(e) => (status_for(&e), html_body(ui::error_page("Conferences", &e.to_string()))).into_response(),
    }
}

fn session_or_redirect(state: &AppState, headers: &HeaderMap, form: &HashMap<String, String>) -> Result<(Principal, String), Box<Response>> {
    let Some(principal @ Principal::Session { .. }) = session_principal(state, headers) else {
        return Err(Box::new(Redirect::to("/login").into_response()));
    };
    if !check_csrf(&principal, form.get("csrf").map(|s| s.as_str())) {
        return Err(Box::new(
            (
                StatusCode::FORBIDDEN,
                html_body(ui::error_page("Rejected", "The form was submitted with an invalid CSRF token.")),
            )
                .into_response(),
        ));
    }
    let Principal::Session { csrf } = &principal else {
        unreachable!("checked above");
    };
    let csrf = csrf.clone();
    Ok((principal, csrf))
}

async fn page_conference_create(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    let (principal, csrf) = match session_or_redirect(&state, &headers, &form) {
        Ok(v) => v,
        Err(response) => return *response,
    };
    let fingerprint = text(&form, "fingerprint");
    let patch = conference_from_form(&form);
    match state.backend.create_conference(&patch, &fingerprint, &actor(&principal, addr)).await {
        Ok(_) => render_conference_list(&state, &csrf, Some(Notice::Success(format!("Conference '{}' created.", patch.name)))).await,
        Err(e) => match state.backend.list_conferences().await {
            Ok(list) => html(ui::conference_new_page(&list, &csrf, Some(Notice::Failure(e.to_string())))),
            Err(e) => (status_for(&e), html_body(ui::error_page("Conferences", &e.to_string()))).into_response(),
        },
    }
}

async fn page_conference_submit(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(index): AxumPath<usize>,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    let (principal, csrf) = match session_or_redirect(&state, &headers, &form) {
        Ok(v) => v,
        Err(response) => return *response,
    };
    let fingerprint = text(&form, "fingerprint");
    let patch = conference_from_form(&form);
    let notice = match state.backend.update_conference(index, &patch, &fingerprint, &actor(&principal, addr)).await {
        Ok(result) if result.changed_fields.is_empty() => Notice::Success("No changes to save.".to_string()),
        Ok(result) => Notice::Success(format!(
            "Saved {} field(s). Backup: {}",
            result.changed_fields.len(),
            result.backup.as_deref().unwrap_or("-")
        )),
        Err(e) => Notice::Failure(e.to_string()),
    };
    render_conference(&state, index, &csrf, Some(notice)).await
}

async fn page_conference_delete(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(index): AxumPath<usize>,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    let (principal, csrf) = match session_or_redirect(&state, &headers, &form) {
        Ok(v) => v,
        Err(response) => return *response,
    };
    let fingerprint = text(&form, "fingerprint");
    let notice = match state.backend.delete_conference(index, &fingerprint, &actor(&principal, addr)).await {
        Ok(result) => Notice::Success(format!("Conference deleted. Backup: {}", result.backup.as_deref().unwrap_or("-"))),
        Err(e) => Notice::Failure(e.to_string()),
    };
    render_conference_list(&state, &csrf, Some(notice)).await
}

async fn api_list_conferences(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if authenticate(&state, &headers).is_none() {
        return unauthorized();
    }
    match state.backend.list_conferences().await {
        Ok(list) => Json(list).into_response(),
        Err(e) => json_error(&e),
    }
}

async fn api_get_conference(State(state): State<AppState>, headers: HeaderMap, AxumPath(index): AxumPath<usize>) -> Response {
    if authenticate(&state, &headers).is_none() {
        return unauthorized();
    }
    match state.backend.get_conference(index).await {
        Ok(conf) => Json(conf).into_response(),
        Err(e) => json_error(&e),
    }
}

async fn api_create_conference(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let Some(principal) = authenticate(&state, &headers) else {
        return unauthorized();
    };
    if !check_csrf(&principal, csrf_header(&headers).as_deref()) {
        return forbidden("missing or invalid CSRF token");
    }
    let fingerprint = match extract_fingerprint(&body) {
        Ok(value) => value,
        Err(e) => return json_error(&e),
    };
    let patch: ConferenceDto = match serde_json::from_value(strip_fingerprint(body)) {
        Ok(patch) => patch,
        Err(e) => return json_error(&json_val(e)),
    };
    match state.backend.create_conference(&patch, &fingerprint, &actor(&principal, addr)).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => json_error(&e),
    }
}

async fn api_update_conference(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(index): AxumPath<usize>,
    Json(body): Json<Value>,
) -> Response {
    let Some(principal) = authenticate(&state, &headers) else {
        return unauthorized();
    };
    if !check_csrf(&principal, csrf_header(&headers).as_deref()) {
        return forbidden("missing or invalid CSRF token");
    }
    let fingerprint = match extract_fingerprint(&body) {
        Ok(value) => value,
        Err(e) => return json_error(&e),
    };
    let patch: ConferenceDto = match serde_json::from_value(strip_fingerprint(body)) {
        Ok(patch) => patch,
        Err(e) => return json_error(&json_val(e)),
    };
    match state.backend.update_conference(index, &patch, &fingerprint, &actor(&principal, addr)).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => json_error(&e),
    }
}

async fn api_delete_conference(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(index): AxumPath<usize>,
    Json(body): Json<Value>,
) -> Response {
    let Some(principal) = authenticate(&state, &headers) else {
        return unauthorized();
    };
    if !check_csrf(&principal, csrf_header(&headers).as_deref()) {
        return forbidden("missing or invalid CSRF token");
    }
    let fingerprint = match extract_fingerprint(&body) {
        Ok(value) => value,
        Err(e) => return json_error(&e),
    };
    match state.backend.delete_conference(index, &fingerprint, &actor(&principal, addr)).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => json_error(&e),
    }
}
