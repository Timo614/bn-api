use actix_web::{HttpResponse, Path};
use auth::user::User;
use bigneon_db::models::*;
use db::Connection;
use errors::*;
use extractors::*;
use models::PathParameters;

pub fn show(
    (connection, parameters, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::ChatWorkflowRead)?;
    let chat_workflow_item = ChatWorkflowItem::find(parameters.id, connection.get())?;
    Ok(HttpResponse::Ok().json(&chat_workflow_item))
}

pub fn create(
    (connection, new_chat_workflow_item, user): (Connection, Json<NewChatWorkflowItem>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::ChatWorkflowWrite)?;
    let connection = connection.get();
    let chat_workflow_item = new_chat_workflow_item.into_inner().commit(connection)?;
    Ok(HttpResponse::Created().json(&chat_workflow_item))
}

pub fn update(
    (connection, parameters, chat_workflow_item_parameters, user): (
        Connection,
        Path<PathParameters>,
        Json<ChatWorkflowItemEditableAttributes>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::ChatWorkflowWrite)?;
    let connection = connection.get();
    let chat_workflow_item = ChatWorkflowItem::find(parameters.id, connection)?;
    let updated_chat_workflow_item =
        chat_workflow_item.update(chat_workflow_item_parameters.into_inner(), connection)?;
    Ok(HttpResponse::Ok().json(updated_chat_workflow_item))
}

pub fn destroy((conn, path, user): (Connection, Path<PathParameters>, User)) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::ChatWorkflowDelete)?;
    let conn = conn.get();
    let chat_workflow_item = ChatWorkflowItem::find(path.id, conn)?;

    chat_workflow_item.destroy(&*conn)?;
    Ok(HttpResponse::Ok().json(json!({})))
}
