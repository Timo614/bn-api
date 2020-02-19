use actix_web::{ws, HttpRequest, HttpResponse};
use auth::user::User;
use bigneon_db::prelude::*;
use db::Connection;
use errors::*;
use models::*;
use server::AppState;

pub fn initate((conn, request, user): (Connection, HttpRequest<AppState>, User)) -> Result<HttpResponse, BigNeonError> {
    let cloned_connection = conn.clone();
    let connection = conn.get();

    // Endpoint returns 404 if no active chat session
    let chat_session = ChatSession::find_active_for_user(&user.user, connection)?;

    Ok(
        ws::start(&request, ChatWebsocket::new(chat_session.id, cloned_connection))
            .map_err(|err| ApplicationError::new(format!("Websocket error: {:?}", err)))?,
    )
}
