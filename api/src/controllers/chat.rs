use actix_web::{HttpRequest, HttpResponse}; //ws,
use auth::user::User;
use bigneon_db::prelude::*;
use db::Connection;
use errors::*;
use models::*;
use server::AppState;

pub fn initate(
    (conn, _request, _user): (Connection, HttpRequest<AppState>, User),
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    //
    //Ok(ws::start(&request, ChatWebSocket::new(event.id))
    //    .map_err(|err| ApplicationError::new(format!("Websocket error: {:?}", err)))?)
    Ok(HttpResponse::Ok().json(json!(None)))
}
