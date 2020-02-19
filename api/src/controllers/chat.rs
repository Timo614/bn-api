use actix_web::{ws, HttpRequest, HttpResponse};
use auth::user::User;
use db::Connection;
use errors::*;
use models::*;
use server::AppState;

pub fn initate((conn, request, user): (Connection, HttpRequest<AppState>, User)) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();

    Ok(ws::start(&request, ChatWebsocket::new(user.id()))
        .map_err(|err| ApplicationError::new(format!("Websocket error: {:?}", err)))?)
}
