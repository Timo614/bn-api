use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path};
use bigneon_api::controllers::chat_workflow_responses;
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
    let chat_workflow_response = database.create_chat_workflow_response().finish();
    let expected_json = serde_json::to_string(&chat_workflow_response).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = chat_workflow_response.id;

    let response: HttpResponse = chat_workflow_responses::show((database.connection.clone(), path, auth_user)).into();

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
    let chat_workflow_response = database.create_chat_workflow_response().finish();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = chat_workflow_response.id;

    let response: HttpResponse =
        chat_workflow_responses::destroy((database.connection.clone().into(), path, auth_user)).into();

    if should_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let chat_workflow_response = ChatWorkflowResponse::find(chat_workflow_response.id, connection);
        assert!(chat_workflow_response.is_err());
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let chat_workflow_item = database.create_chat_workflow_item().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let json = Json(NewChatWorkflowResponse {
        chat_workflow_item_id: chat_workflow_item.id,
        response_type: ChatWorkflowResponseType::Noop,
        response: None,
        answer_value: None,
        next_chat_workflow_item_id: None,
        rank: 1,
    });

    let response: HttpResponse =
        chat_workflow_responses::create((database.connection.clone().into(), json, auth_user)).into();

    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let chat_workflow_response: ChatWorkflowResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(chat_workflow_response.response_type, ChatWorkflowResponseType::Noop);
        assert_eq!(chat_workflow_response.chat_workflow_item_id, chat_workflow_item.id);
        assert_eq!(chat_workflow_response.rank, 1);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn update(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);
    let chat_workflow_response = database.create_chat_workflow_response().finish();

    let new_response = "New Response".to_string();
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = chat_workflow_response.id;

    let json = Json(ChatWorkflowResponseEditableAttributes {
        response: Some(Some(new_response.clone())),
        ..Default::default()
    });

    let response: HttpResponse =
        chat_workflow_responses::update((database.connection.clone().into(), path, json, auth_user)).into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let updated_chat_workflow_response: ChatWorkflowResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(updated_chat_workflow_response.response, Some(new_response));
    } else {
        support::expects_unauthorized(&response);
    }
}
