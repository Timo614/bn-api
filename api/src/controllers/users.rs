use actix_http::Response;
use actix_web;
use actix_web::Responder;
use actix_web::web::Data;
use actix_web::{http::StatusCode, web::Path, web::Query, HttpRequest, HttpResponse};
use auth::user::User as AuthUser;
use bigneon_db::prelude::*;
use communications::mailers;
use controllers::auth;
use controllers::auth::LoginRequest;
use db::Connection;
use diesel::PgConnection;
use errors::*;
use extractors::*;
use helpers::application;
use models::*;
use server::AppState;
use std::collections::HashMap;
use std::str;
use utils::google_recaptcha;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SearchUserByEmail {
    pub email: String,
}

#[derive(Serialize)]
pub struct CurrentUser {
    pub user: DisplayUser,
    pub roles: Vec<Roles>,
    pub scopes: Vec<Scopes>,
    pub organization_roles: HashMap<Uuid, Vec<Roles>>,
    pub organization_scopes: HashMap<Uuid, Vec<Scopes>>,
    pub organization_event_ids: HashMap<Uuid, Vec<Uuid>>,
    pub organization_readonly_event_ids: HashMap<Uuid, Vec<Uuid>>,
    pub event_scopes: HashMap<Uuid, Vec<Scopes>>,
}

impl Responder for CurrentUser {
    type Error = actix_web::Error;
    type Future = Result<Response, Self::Error>;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        let body = serde_json::to_string(&self)?;

        Ok(HttpResponse::Ok().json(body))
    }
}

#[derive(Serialize, Deserialize, Clone)]
///struct used to indicate paging information and search query information
pub struct ActivityParameters {
    past_or_upcoming: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct InputPushNotificationTokens {
    pub token_source: String,
    pub token: String,
}

pub fn current_user(
    connection: Connection,
    auth_user: AuthUser,
) -> Result<CurrentUser, BigNeonError> {
    let connection = connection.get();
    current_user_from_user(&auth_user.user, connection)
}

pub fn activity(
    connection: Connection,
    path: Path<OrganizationFanPathParameters>,
    query: Query<PagingParameters>,
    activity_query: Query<ActivityParameters>,
    auth_user: AuthUser,
) -> Result<WebPayload<ActivitySummary>, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(path.id, connection)?;
    auth_user.requires_scope_for_organization(Scopes::OrgFans, &organization, &connection)?;
    let user = User::find(path.user_id, connection)?;

    let past_or_upcoming = match activity_query
        .past_or_upcoming
        .clone()
        .unwrap_or("upcoming".to_string())
        .as_str()
    {
        "past" => PastOrUpcoming::Past,
        _ => PastOrUpcoming::Upcoming,
    };

    let mut payload = user.activity(
        &organization,
        query.page(),
        query.limit(),
        query.dir.unwrap_or(SortingDir::Desc),
        past_or_upcoming,
        match query.get_tag_as_str("type") {
            Some(t) => Some(t.parse()?),
            None => None,
        },
        connection,
    )?;
    payload.paging.tags.insert(
        "past_or_upcoming".to_string(),
        json!(activity_query.past_or_upcoming),
    );

    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub fn profile(
    connection: Connection,
    path: Path<OrganizationFanPathParameters>,
    auth_user: AuthUser,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(path.id, connection)?;
    auth_user.requires_scope_for_organization(Scopes::OrgFans, &organization, &connection)?;

    let user = User::find(path.user_id, connection)?;

    // Confirm organization has specified user as a fan
    if !organization.has_fan(&user, connection)? {
        return application::forbidden("Fan does not belong to this organization");
    }

    Ok(HttpResponse::Ok().json(&user.get_profile_for_organization(&organization, connection)?))
}

pub fn history(
    connection: Connection,
    path: Path<OrganizationFanPathParameters>,
    query: Query<PagingParameters>,
    auth_user: AuthUser,
) -> Result<WebPayload<HistoryItem>, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(path.id, connection)?;
    auth_user.requires_scope_for_organization(Scopes::OrgFans, &organization, &connection)?;

    let user = User::find(path.user_id, connection)?;

    // Confirm organization has specified user as a fan
    if !organization.has_fan(&user, connection)? {
        return application::unauthorized(Some(auth_user), None);
    }

    let payload = user.get_history_for_organization(
        &organization,
        query.page(),
        query.limit(),
        query.dir.unwrap_or(SortingDir::Desc),
        connection,
    )?;

    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub fn update_current_user(
    connection: Connection,
    user_parameters: Json<UserProfileAttributes>,
    auth_user: AuthUser,
) -> Result<CurrentUser, BigNeonError> {
    let connection = connection.get();

    let updated_user = auth_user.user.update(
        user_parameters.into_inner().into(),
        Some(auth_user.id()),
        connection,
    )?;
    let current_user = current_user_from_user(&updated_user, connection)?;
    Ok(current_user)
}

pub fn show(
    connection: Connection,
    parameters: Path<PathParameters>,
    auth_user: AuthUser,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let user = User::find(parameters.id, connection)?;
    if !(auth_user.user == user || auth_user.user.is_admin()) {
        return application::unauthorized(Some(auth_user), None);
    }

    Ok(HttpResponse::Ok().json(&user.for_display()?))
}

pub fn list_organizations(
    connection: Connection,
    parameters: Path<PathParameters>,
    query_parameters: Query<PagingParameters>,
    auth_user: AuthUser,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let user = User::find(parameters.id, connection)?;
    if !(auth_user.user == user || auth_user.user.is_admin()) {
        return application::unauthorized(Some(auth_user), None);
    }
    //TODO implement proper paging on db.
    let organization_links = Organization::all_org_names_linked_to_user(parameters.id, connection)?;

    Ok(HttpResponse::Ok().json(&Payload::from_data(
        organization_links,
        query_parameters.page(),
        query_parameters.limit(),
    )))
}

pub fn show_push_notification_tokens_for_user_id(
    connection: Connection,
    parameters: Path<PathParameters>,
    auth_user: AuthUser,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let user = User::find(parameters.id, connection)?;
    if !(auth_user.user == user || auth_user.user.is_admin()) {
        return application::unauthorized(Some(auth_user), None);
    }

    let push_notification_tokens: Vec<DisplayPushNotificationToken> =
        PushNotificationToken::find_by_user_id(parameters.id, connection)?
            .iter()
            .map(|t| DisplayPushNotificationToken::from(t.clone()))
            .collect();

    Ok(HttpResponse::Ok().json(&push_notification_tokens))
}

pub fn show_push_notification_tokens(
    connection: Connection,
    auth_user: AuthUser,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();

    let push_notification_tokens: Vec<DisplayPushNotificationToken> =
        PushNotificationToken::find_by_user_id(auth_user.user.id, connection)?
            .iter()
            .map(|t| DisplayPushNotificationToken::from(t.clone()))
            .collect();

    Ok(HttpResponse::Ok().json(&push_notification_tokens))
}

pub fn add_push_notification_token(
    connection: Connection,
    add_request: Json<InputPushNotificationTokens>,
    auth_user: AuthUser,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();

    let add_push_notification_token_request = add_request.into_inner();
    PushNotificationToken::create(
        auth_user.user.id,
        add_push_notification_token_request.token_source.clone(),
        add_push_notification_token_request.token.clone(),
    )
    .commit(auth_user.id(), connection)?;

    Ok(HttpResponse::Ok().finish())
}

pub fn remove_push_notification_token(
    connection: Connection,
    parameters: Path<PathParameters>,
    auth_user: AuthUser,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();

    PushNotificationToken::remove(auth_user.user.id, parameters.id, connection)?;

    Ok(HttpResponse::Ok().finish())
}

pub fn register(
    state: Data<AppState>,
    http_request: HttpRequest,
    connection: Connection,
    parameters: Json<RegisterRequest>,
) -> Result<HttpResponse, BigNeonError> {
    let connection_info = http_request.connection_info();
    let remote_ip = connection_info.remote();
    let mut log_data = HashMap::new();
    log_data.insert("email", parameters.email.clone().into());

    if let Some(ref google_recaptcha_secret_key) = state.config.google_recaptcha_secret_key {
        if let Err(err) = verify_recaptcha(
            google_recaptcha_secret_key,
            &parameters.captcha_response,
            remote_ip,
        ) {
            return application::unauthorized_with_message(
                err.reason.as_str(),
                None,
                Some(log_data),
            );
        }
    }

    let new_user: NewUser = parameters.into_inner().into();
    match new_user.commit(None, connection.get()) {
        Ok(_) => (),
        Err(e) => match e.error_code {
            ErrorCode::DuplicateKeyError => {
                return application::unprocessable("A user with this email already exists");
            }
            _ => return Err(e.into()),
        },
    };

    if let (Some(first_name), Some(email)) = (new_user.first_name, new_user.email) {
        mailers::user::user_registered(first_name, email, &state.config, connection.get())?;
    }

    Ok(HttpResponse::Created().finish())
}

pub fn register_and_login(
    state: Data<AppState>,
    http_request: HttpRequest,
    connection: Connection,
    parameters: Json<RegisterRequest>,
    request_info: RequestInfo,
) -> Result<HttpResponse, BigNeonError> {
    let connection_info = http_request.connection_info();
    let remote_ip = connection_info.remote();
    let mut log_data = HashMap::new();
    log_data.insert("email", parameters.email.clone().into());

    if let Some(ref google_recaptcha_secret_key) = state.config.google_recaptcha_secret_key {
        if let Err(err) = verify_recaptcha(
            google_recaptcha_secret_key,
            &parameters.captcha_response,
            remote_ip,
        ) {
            return application::unauthorized_with_message(
                err.reason.as_str(),
                None,
                Some(log_data),
            );
        }
    }

    let email = parameters.email.clone();
    let password = parameters.password.clone();
    let new_user: NewUser = parameters.into_inner().into();
    match new_user.commit(None, connection.get()) {
        Ok(_) => (),
        Err(e) => match e.error_code {
            ErrorCode::DuplicateKeyError => {
                return application::unprocessable("A user with this email already exists");
            }
            _ => return Err(e.into()),
        },
    };
    let json = Json(LoginRequest::new(&email, &password));
    let token_response = auth::token(state.clone(), http_request.clone(), connection.clone(), json, request_info)?;

    if let (Some(first_name), Some(email)) = (new_user.first_name, new_user.email) {
        mailers::user::user_registered(first_name, email, &state.config, connection.get())?;
    }

    Ok(HttpResponse::Created().json(token_response))
}

fn current_user_from_user(
    user: &User,
    connection: &PgConnection,
) -> Result<CurrentUser, BigNeonError> {
    let roles_by_organization = user.get_roles_by_organization(connection)?;
    let mut scopes_by_organization = HashMap::new();
    for (organization_id, roles) in &roles_by_organization {
        scopes_by_organization.insert(organization_id.clone(), scopes::get_scopes(roles.clone()));
    }
    let (events_by_organization, readonly_events_by_organization) =
        user.get_event_ids_by_organization(connection)?;
    let mut event_scopes = HashMap::new();
    for event_user in user.event_users(connection)? {
        event_scopes.insert(
            event_user.event_id,
            scopes::get_scopes(vec![event_user.role]),
        );
    }

    Ok(CurrentUser {
        user: user.clone().for_display()?,
        roles: user.role.clone(),
        scopes: user.get_global_scopes(),
        organization_roles: roles_by_organization,
        organization_scopes: scopes_by_organization,
        organization_event_ids: events_by_organization,
        organization_readonly_event_ids: readonly_events_by_organization,
        event_scopes,
    })
}

fn verify_recaptcha(
    google_recaptcha_secret_key: &str,
    captcha_response: &Option<String>,
    remote_ip: Option<&str>,
) -> Result<google_recaptcha::Response, ApplicationError> {
    match captcha_response {
        Some(ref captcha_response) => {
            let captcha_response = google_recaptcha::verify_response(
                google_recaptcha_secret_key,
                captcha_response.to_owned(),
                remote_ip,
            )?;
            if !captcha_response.success {
                return Err(ApplicationError::new("Captcha value invalid".to_string()));
            }
            Ok(captcha_response)
        }
        None => Err(ApplicationError::new("Captcha required".to_string())),
    }
}
