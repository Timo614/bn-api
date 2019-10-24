use actix_web::web::{Path, Query};
use actix_web::HttpResponse;
use auth::user::User;
use bigneon_db::prelude::*;
use db::Connection;
use errors::*;
use extractors::Json;
use helpers::application;
use models::*;
use reqwest::StatusCode;

#[derive(Deserialize, Serialize)]
pub struct NewNoteRequest {
    pub note: String,
}

#[derive(Deserialize, Serialize)]
pub struct NoteFilterParameters {
    pub filter_deleted: Option<bool>,
}

pub fn create(
    conn: Connection,
    path: Path<MainTablePathParameters>,
    json: Json<NewNoteRequest>,
    auth_user: User,
) -> Result<HttpResponse, BigNeonError> {
    let connection = conn.get();
    let main_table: Tables = path.main_table.parse().map_err(|_| NotFoundError {})?;
    let note = match main_table {
        Tables::Orders => {
            let order = Order::find(path.id, connection)?;
            auth_user.requires_scope_for_order(Scopes::NoteWrite, &order, connection)?;
            order.create_note(json.note.clone(), auth_user.id(), connection)?
        }
        _ => return application::unauthorized(Some(auth_user), None),
    };

    Ok(HttpResponse::Created().json(json!(note)))
}

pub fn index(
    conn: Connection,
    path: Path<MainTablePathParameters>,
    query: Query<PagingParameters>,
    note_query: Query<NoteFilterParameters>,
    auth_user: User,
) -> Result<WebPayload<Note>, BigNeonError> {
    let connection = conn.get();
    let mut filter_deleted = true;
    let main_table: Tables = path.main_table.parse().map_err(|_| NotFoundError {})?;
    match main_table {
        Tables::Orders => {
            let order = Order::find(path.id, connection)?;
            auth_user.requires_scope_for_order(Scopes::NoteRead, &order, connection)?;
            if let Some(query_filter_deleted) = note_query.filter_deleted {
                if !query_filter_deleted {
                    auth_user.requires_scope_for_order(Scopes::NoteDelete, &order, connection)?;
                    filter_deleted = false;
                }
            }
        }
        _ => return application::unauthorized(Some(auth_user), None),
    }

    let mut payload = Note::find_for_table(
        main_table,
        path.id,
        filter_deleted,
        query.page(),
        query.limit(),
        connection,
    )?;
    payload
        .paging
        .tags
        .insert("filter_deleted".to_string(), json!(filter_deleted));
    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub fn destroy(
    conn: Connection,
    path: Path<PathParameters>,
    auth_user: User,
) -> Result<HttpResponse, BigNeonError> {
    let connection = conn.get();
    let note = Note::find(path.id, connection)?;

    match note.main_table {
        Tables::Orders => {
            let order = Order::find(note.main_id, connection)?;
            auth_user.requires_scope_for_order(Scopes::NoteDelete, &order, connection)?;
        }
        _ => return application::unauthorized(Some(auth_user), None),
    }

    note.destroy(auth_user.id(), connection)?;
    Ok(HttpResponse::Ok().json(json!({})))
}
