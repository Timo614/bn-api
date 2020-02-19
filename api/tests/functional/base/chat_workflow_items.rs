use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path};
use bigneon_api::controllers::chat_workflow_items;
use bigneon_api::extractors::*;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use serde_json;
use support;
use support::database::TestDatabase;
use support::test_request::TestRequest;

pub fn show(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let chat_workflow_item = database.create_chat_workflow_item().finish();
    let expected_json = serde_json::to_string(&chat_workflow_item).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = chat_workflow_item.id;

    let response: HttpResponse = chat_workflow_items::show((database.connection.clone(), path, auth_user)).into();

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
    let chat_workflow_item = database.create_chat_workflow_item().finish();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = chat_workflow_item.id;

    let response: HttpResponse =
        chat_workflow_items::destroy((database.connection.clone().into(), path, auth_user)).into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let chat_workflow_item = ChatWorkflowItem::find(chat_workflow_item.id, connection);
        assert!(chat_workflow_item.is_err());
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let chat_workflow = database.create_chat_workflow().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let message = "Message".to_string();
    let json = Json(NewChatWorkflowItem {
        message: Some(message.clone()),
        chat_workflow_id: chat_workflow.id,
        item_type: ChatWorkflowItemType::Question,
        render_type: None,
        response_wait: 3,
    });

    let response: HttpResponse =
        chat_workflow_items::create((database.connection.clone().into(), json, auth_user)).into();

    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let chat_workflow_item: ChatWorkflowItem = serde_json::from_str(&body).unwrap();
        assert_eq!(chat_workflow_item.message, Some(message));
        assert_eq!(chat_workflow_item.item_type, ChatWorkflowItemType::Question);
        assert_eq!(chat_workflow_item.chat_workflow_id, chat_workflow.id);
        assert_eq!(chat_workflow_item.response_wait, 3);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn update(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let chat_workflow_item = database.create_chat_workflow_item().finish();

    let new_message = "New Message".to_string();
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = chat_workflow_item.id;

    let json = Json(ChatWorkflowItemEditableAttributes {
        message: Some(Some(new_message.clone())),
        ..Default::default()
    });

    let response: HttpResponse =
        chat_workflow_items::update((database.connection.clone().into(), path, json, auth_user)).into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let updated_chat_workflow_item: ChatWorkflowItem = serde_json::from_str(&body).unwrap();
        assert_eq!(updated_chat_workflow_item.message, Some(new_message));
    } else {
        support::expects_unauthorized(&response);
    }
}
