use actix_web::web::Path;
use actix_web::{web::Query, HttpResponse};
use auth::user::User;
use bigneon_db::models::enums::{BroadcastChannel, BroadcastType};
use bigneon_db::models::scopes::Scopes;
use bigneon_db::models::{Broadcast, BroadcastEditableAttributes, Organization, PagingParameters};
use chrono::NaiveDateTime;
use db::Connection;
use errors::BigNeonError;
use extractors::Json;
use models::{PathParameters, WebPayload};
use reqwest::StatusCode;

#[derive(Deserialize, Serialize)]
pub struct NewBroadcastData {
    pub name: Option<String>,
    pub notification_type: BroadcastType,
    //None is now
    pub send_at: Option<NaiveDateTime>,
    pub message: Option<String>,
    pub channel: Option<BroadcastChannel>,
}

pub fn create(
    conn: Connection,
    path: Path<PathParameters>,
    json: Json<NewBroadcastData>,
    user: User,
) -> Result<HttpResponse, BigNeonError> {
    let connection = conn.get();
    let organization = Organization::find_for_event(path.id, connection)?;

    user.requires_scope_for_organization(Scopes::EventBroadcast, &organization, connection)?;
    let push_notification = Broadcast::create(
        path.id,
        json.notification_type.clone(),
        json.channel
            .clone()
            .unwrap_or(BroadcastChannel::PushNotification),
        json.name
            .clone()
            .unwrap_or(json.notification_type.to_string()),
        json.message.clone(),
        json.send_at.clone(),
        None,
    )
    .commit(connection)?;

    Ok(HttpResponse::Created().json(json!(push_notification)))
}

pub fn index(
    conn: Connection,
    path: Path<PathParameters>,
    query: Query<PagingParameters>,
    user: User,
) -> Result<WebPayload<Broadcast>, BigNeonError> {
    let connection = conn.get();
    let organization = Organization::find_for_event(path.id, connection)?;

    user.requires_scope_for_organization(Scopes::EventBroadcast, &organization, connection)?;

    let push_notifications =
        Broadcast::find_by_event_id(path.id, query.page(), query.limit(), connection)?;

    Ok(WebPayload::new(StatusCode::OK, push_notifications))
}

pub fn show(
    conn: Connection,
    path: Path<PathParameters>,
    user: User,
) -> Result<HttpResponse, BigNeonError> {
    let connection = conn.get();
    let push_notification = Broadcast::find(path.id, connection)?;
    let organization = Organization::find_for_event(push_notification.event_id, connection)?;

    user.requires_scope_for_organization(Scopes::EventBroadcast, &organization, connection)?;

    Ok(HttpResponse::Ok().json(push_notification))
}

pub fn update(
    conn: Connection,
    path: Path<PathParameters>,
    json: Json<BroadcastEditableAttributes>,
    user: User,
) -> Result<HttpResponse, BigNeonError> {
    let connection = conn.get();
    let broadcast = Broadcast::find(path.id, connection)?;
    let organization = Organization::find_for_event(broadcast.event_id, connection)?;

    user.requires_scope_for_organization(Scopes::EventBroadcast, &organization, connection)?;
    let broadcast_attributes = json.into_inner();
    let broadcast = broadcast.update(broadcast_attributes, connection)?;
    Ok(HttpResponse::Ok().json(broadcast))
}

pub fn delete(
    conn: Connection,
    path: Path<PathParameters>,
    user: User,
) -> Result<HttpResponse, BigNeonError> {
    let connection = conn.get();
    let broadcast = Broadcast::find(path.id, connection)?;
    let organization = Organization::find_for_event(broadcast.event_id, connection)?;

    user.requires_scope_for_organization(Scopes::EventBroadcast, &organization, connection)?;

    let broadcast = broadcast.cancel(connection)?;
    Ok(HttpResponse::Ok().json(broadcast))
}
