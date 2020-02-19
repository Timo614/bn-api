use actix_web::{HttpResponse, Query};
use auth::user::User;
use bigneon_db::prelude::*;
use db::Connection;
use errors::*;
use helpers::application;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct ChatParameters {
    pub chat_workflow_id: Uuid,
}

pub fn create(
    (connection, parameters, user): (Connection, Query<ChatParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let chat_workflow = ChatWorkflow::find(parameters.chat_workflow_id, connection)?;

    if ChatSession::find_active_for_user(&user.user, connection)
        .optional()?
        .is_some()
    {
        return application::unprocessable("Could not create chat session as one is already ongoing");
    }

    let chat_session = ChatSession::create(user.id(), chat_workflow.id, None).commit(connection)?;
    Ok(HttpResponse::Ok().json(chat_session))
}
