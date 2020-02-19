use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path};
use bigneon_api::controllers::chat_workflows;
use bigneon_api::extractors::*;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use serde_json;
use support;
use support::database::TestDatabase;
use support::test_request::TestRequest;

pub fn index(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let chat_workflow = database.create_chat_workflow().with_name("Chat Workflow 1").finish();
    let chat_workflow2 = database.create_chat_workflow().with_name("Chat Workflow 2").finish();

    let response: HttpResponse = chat_workflows::index(database.connection.clone().into(), auth_user).into();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let response_chat_workflows: Vec<ChatWorkflow> = serde_json::from_str(body).unwrap();
        assert_eq!(response_chat_workflows, vec![chat_workflow, chat_workflow2]);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn show(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let chat_workflow = database.create_chat_workflow().finish();
    let expected_json = serde_json::to_string(&chat_workflow.for_display(connection).unwrap()).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = chat_workflow.id;

    let response: HttpResponse = chat_workflows::show((database.connection.clone(), path, auth_user)).into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(body, expected_json);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn destroy(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let chat_workflow = database.create_chat_workflow().finish();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = chat_workflow.id;

    let response: HttpResponse = chat_workflows::destroy((database.connection.clone().into(), path, auth_user)).into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let chat_workflow = ChatWorkflow::find(chat_workflow.id, connection);
        assert!(chat_workflow.is_err());
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let name = "Chat Workflow Name".to_string();
    let json = Json(NewChatWorkflow { name: name.clone() });

    let response: HttpResponse = chat_workflows::create((database.connection.clone().into(), json, auth_user)).into();

    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let chat_workflow: ChatWorkflow = serde_json::from_str(&body).unwrap();
        assert_eq!(chat_workflow.name, name);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn publish(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    // Draft with initial chat workflow item set
    let chat_workflow = database.create_chat_workflow().draft().finish();
    let chat_workflow_item = database
        .create_chat_workflow_item()
        .with_chat_workflow(&chat_workflow)
        .finish();
    let chat_workflow = chat_workflow
        .update(
            ChatWorkflowEditableAttributes {
                initial_chat_workflow_item_id: Some(Some(chat_workflow_item.id)),
                ..Default::default()
            },
            connection,
        )
        .unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = chat_workflow.id;

    let response: HttpResponse = chat_workflows::publish((database.connection.clone().into(), path, auth_user)).into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let updated_chat_workflow: ChatWorkflow = serde_json::from_str(&body).unwrap();
        assert_eq!(updated_chat_workflow.status, ChatWorkflowStatus::Published);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn update(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let chat_workflow = database.create_chat_workflow().finish();

    let new_name = "New Name".to_string();
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = chat_workflow.id;

    let json = Json(ChatWorkflowEditableAttributes {
        name: Some(new_name.clone()),
        ..Default::default()
    });

    let response: HttpResponse =
        chat_workflows::update((database.connection.clone().into(), path, json, auth_user)).into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let updated_chat_workflow: ChatWorkflow = serde_json::from_str(&body).unwrap();
        assert_eq!(updated_chat_workflow.name, new_name);
    } else {
        support::expects_unauthorized(&response);
    }
}
