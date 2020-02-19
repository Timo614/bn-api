use actix_web::{http::StatusCode, FromRequest, HttpResponse, Query};
use bigneon_api::controllers::chat_sessions::{self, *};
use bigneon_db::models::*;
use chrono::prelude::*;
use serde_json;
use support::database::TestDatabase;
use support::test_request::TestRequest;
use support::{self, *};

#[test]
fn create() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, Some(&organization), &database);

    // No active chat session
    assert!(ChatSession::find_active_for_user(&user, connection).is_err());
    let chat_workflow = database.create_chat_workflow().finish();

    let test_request = TestRequest::create_with_uri(&format!("/events?chat_workflow_id={}", chat_workflow.id));
    let parameters = Query::<ChatParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse =
        chat_sessions::create((database.connection.clone().into(), parameters, auth_user)).into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let returned_chat_session: ChatSession = serde_json::from_str(&body).unwrap();
    assert_eq!(
        ChatSession::find_active_for_user(&user, connection).unwrap(),
        returned_chat_session
    );
    assert_eq!(returned_chat_session.chat_workflow_id, chat_workflow.id);
    assert_eq!(returned_chat_session.user_id, user.id);
    assert!(returned_chat_session.expires_at.unwrap() > Utc::now().naive_utc());
}

#[test]
fn create_fails_due_to_existing_chat_session() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, Some(&organization), &database);
    database.create_chat_session().with_user(&user).finish();

    // Active chat session
    assert!(ChatSession::find_active_for_user(&user, connection).is_ok());
    let chat_workflow = database.create_chat_workflow().finish();

    let test_request = TestRequest::create_with_uri(&format!("/events?chat_workflow_id={}", chat_workflow.id));
    let parameters = Query::<ChatParameters>::extract(&test_request.request).unwrap();

    let response: HttpResponse =
        chat_sessions::create((database.connection.clone().into(), parameters, auth_user)).into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let expected_json = HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY)
        .into_builder()
        .json(json!({
            "error": "Could not create chat session as one is already ongoing"
        }));
    let expected_text = unwrap_body_to_string(&expected_json).unwrap();
    let body = unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_text);
}
