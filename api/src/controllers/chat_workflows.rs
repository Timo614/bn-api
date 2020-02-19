use actix_web::{HttpResponse, Path};
use auth::user::User;
use bigneon_db::models::*;
use db::Connection;
use errors::*;
use extractors::*;
use models::PathParameters;

pub fn index(connection: Connection, user: User) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::ChatWorkflowRead)?;
    let connection = connection.get();
    let chat_workflows = ChatWorkflow::all(connection)?;
    Ok(HttpResponse::Ok().json(&chat_workflows))
}

pub fn show(
    (connection, parameters, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::ChatWorkflowRead)?;
    let connection = connection.get();
    let chat_workflow = ChatWorkflow::find(parameters.id, connection)?;
    Ok(HttpResponse::Ok().json(chat_workflow.for_display(connection)?))
}

pub fn create(
    (connection, new_chat_workflow, user): (Connection, Json<NewChatWorkflow>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::ChatWorkflowWrite)?;
    let connection = connection.get();
    let chat_workflow = new_chat_workflow.into_inner().commit(Some(user.id()), connection)?;
    Ok(HttpResponse::Created().json(&chat_workflow))
}

pub fn update(
    (connection, parameters, chat_workflow_parameters, user): (
        Connection,
        Path<PathParameters>,
        Json<ChatWorkflowEditableAttributes>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::ChatWorkflowWrite)?;
    let connection = connection.get();
    let chat_workflow = ChatWorkflow::find(parameters.id, connection)?;
    let updated_chat_workflow = chat_workflow.update(chat_workflow_parameters.into_inner(), connection)?;
    Ok(HttpResponse::Ok().json(updated_chat_workflow))
}

pub fn publish(
    (connection, parameters, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::ChatWorkflowWrite)?;
    let connection = connection.get();
    let chat_workflow = ChatWorkflow::find(parameters.id, connection)?.publish(Some(user.id()), connection)?;
    Ok(HttpResponse::Ok().json(chat_workflow))
}

pub fn destroy((conn, path, user): (Connection, Path<PathParameters>, User)) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::ChatWorkflowDelete)?;
    let conn = conn.get();
    let chat_workflow = ChatWorkflow::find(path.id, conn)?;

    chat_workflow.destroy(Some(user.id()), &*conn)?;
    Ok(HttpResponse::Ok().json(json!({})))
}
