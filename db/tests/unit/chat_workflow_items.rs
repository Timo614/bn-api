use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow = project.create_chat_workflow().draft().finish();
    let chat_workflow_item = project
        .create_chat_workflow_item()
        .with_chat_workflow(&chat_workflow)
        .finish();
    let chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .finish();

    assert!(chat_workflow_item.destroy(connection).is_ok());
    assert!(ChatWorkflow::find(chat_workflow.id, connection).is_ok());
    assert!(ChatWorkflowItem::find(chat_workflow_item.id, connection).is_err());
    assert!(ChatWorkflowResponse::find(chat_workflow_response.id, connection).is_err());
}

#[test]
fn remaining_response_types() {
    let project = TestProject::new();
    let connection = project.get_connection();

    let message_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Message)
        .finish();
    message_chat_workflow_item.responses(connection).unwrap()[0]
        .destroy(connection)
        .unwrap();
    assert_eq!(
        message_chat_workflow_item.remaining_response_types(connection).unwrap(),
        vec![ChatWorkflowResponseType::Noop]
    );
    project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&message_chat_workflow_item)
        .finish();
    assert_eq!(
        message_chat_workflow_item.remaining_response_types(connection).unwrap(),
        Vec::new()
    );

    // Questions do not run out of available answers
    let question_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .finish();
    assert_eq!(
        question_chat_workflow_item
            .remaining_response_types(connection)
            .unwrap(),
        vec![ChatWorkflowResponseType::Answer]
    );

    project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&question_chat_workflow_item)
        .finish();
    assert_eq!(
        question_chat_workflow_item
            .remaining_response_types(connection)
            .unwrap(),
        vec![ChatWorkflowResponseType::Answer]
    );
}

#[test]
fn response_valid() {
    let project = TestProject::new();
    let connection = project.get_connection();

    let message_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Message)
        .finish();
    let noop_chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&message_chat_workflow_item)
        .finish();

    // Questions do not run out of available answers
    let question_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .finish();
    let answer_chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&question_chat_workflow_item)
        .finish();

    assert!(message_chat_workflow_item
        .response_valid(&noop_chat_workflow_response, connection)
        .unwrap());
    assert!(!message_chat_workflow_item
        .response_valid(&answer_chat_workflow_response, connection)
        .unwrap());

    assert!(!question_chat_workflow_item
        .response_valid(&noop_chat_workflow_response, connection)
        .unwrap());
    assert!(question_chat_workflow_item
        .response_valid(&answer_chat_workflow_response, connection)
        .unwrap());
}

#[test]
fn message() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut chat_session = project.create_chat_session().finish();
    let chat_workflow_item = project
        .create_chat_workflow_item()
        .with_message("Testing normal message")
        .finish();
    assert_eq!(
        chat_workflow_item.message(&chat_session).unwrap(),
        chat_workflow_item.message.unwrap_or("".to_string())
    );

    let chat_workflow_item = project
        .create_chat_workflow_item()
        .with_message("Testing replacements, no valid {last_input} {error_message}")
        .finish();
    assert_eq!(
        chat_workflow_item.message(&chat_session).unwrap(),
        "Testing replacements, no valid  ".to_string()
    );

    // With some values set on the chat session
    chat_session = chat_session
        .add_value_to_context("last_input", json!("Input"), connection)
        .unwrap();
    chat_session = chat_session
        .add_value_to_context("error_message", json!("Error"), connection)
        .unwrap();
    assert_eq!(
        chat_workflow_item.message(&chat_session).unwrap(),
        "Testing replacements, no valid Input Error".to_string()
    );
}

#[test]
fn response_types() {
    let project = TestProject::new();
    let connection = project.get_connection();

    let message_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Message)
        .finish();
    message_chat_workflow_item.responses(connection).unwrap()[0]
        .destroy(connection)
        .unwrap();
    assert!(message_chat_workflow_item
        .response_types(connection)
        .unwrap()
        .is_empty());
    project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&message_chat_workflow_item)
        .finish();
    assert_eq!(
        vec![ChatWorkflowResponseType::Noop],
        message_chat_workflow_item.response_types(connection).unwrap()
    );

    // Questions do not run out of available answers
    let question_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .finish();
    assert!(question_chat_workflow_item
        .response_types(connection)
        .unwrap()
        .is_empty());
    project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&question_chat_workflow_item)
        .finish();
    assert_eq!(
        vec![ChatWorkflowResponseType::Answer],
        question_chat_workflow_item.response_types(connection).unwrap()
    );
}

#[test]
fn chat_workflow() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow = project.create_chat_workflow().finish();
    let chat_workflow_item = project
        .create_chat_workflow_item()
        .with_chat_workflow(&chat_workflow)
        .finish();
    assert_eq!(chat_workflow_item.chat_workflow(connection).unwrap(), chat_workflow);
}

#[test]
fn responses() {
    let project = TestProject::new();
    let connection = project.get_connection();

    let message_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Message)
        .finish();
    message_chat_workflow_item.responses(connection).unwrap()[0]
        .destroy(connection)
        .unwrap();
    assert!(message_chat_workflow_item.responses(connection).unwrap().is_empty());
    let noop_chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&message_chat_workflow_item)
        .finish();
    assert_eq!(
        vec![noop_chat_workflow_response],
        message_chat_workflow_item.responses(connection).unwrap()
    );

    // Questions do not run out of available answers
    let question_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .finish();
    assert!(question_chat_workflow_item.responses(connection).unwrap().is_empty());
    let answer_chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&question_chat_workflow_item)
        .finish();
    assert_eq!(
        vec![answer_chat_workflow_response],
        question_chat_workflow_item.responses(connection).unwrap()
    );
}

#[test]
fn available_response_types() {
    let project = TestProject::new();
    let message_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Message)
        .finish();
    assert_eq!(
        vec![ChatWorkflowResponseType::Noop],
        message_chat_workflow_item.available_response_types()
    );

    let question_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .finish();
    assert_eq!(
        vec![ChatWorkflowResponseType::Answer],
        question_chat_workflow_item.available_response_types()
    );
}

#[test]
fn create_commit() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow = project.create_chat_workflow().finish();
    let chat_workflow_item_type = ChatWorkflowItemType::Question;
    let message = Some("Example message".to_string());
    let chat_workflow_item = ChatWorkflowItem::create(chat_workflow.id, chat_workflow_item_type, message, None, None)
        .commit(connection)
        .unwrap();

    assert!(!chat_workflow_item.id.is_nil());
    assert_eq!(chat_workflow_item.chat_workflow_id, chat_workflow.id);
    assert_eq!(chat_workflow_item.item_type, chat_workflow_item_type);
    assert_eq!(chat_workflow_item.render_type, None);
    assert_eq!(chat_workflow_item.response_wait, 10);
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow_item = project.create_chat_workflow_item().finish();
    assert_eq!(
        ChatWorkflowItem::find(chat_workflow_item.id, connection).unwrap(),
        chat_workflow_item
    );
}

#[test]
fn find_chat_workflow_response_by_response_type() {
    let project = TestProject::new();
    let connection = project.get_connection();

    let message_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Message)
        .finish();
    let noop_chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&message_chat_workflow_item)
        .finish();
    assert_eq!(
        message_chat_workflow_item
            .find_chat_workflow_response_by_response_type(ChatWorkflowResponseType::Noop, connection)
            .unwrap(),
        noop_chat_workflow_response
    );

    assert!(message_chat_workflow_item
        .find_chat_workflow_response_by_response_type(ChatWorkflowResponseType::Answer, connection)
        .is_err());
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow_item = project.create_chat_workflow_item().finish();

    let new_message = "New message".to_string();
    let attributes = ChatWorkflowItemEditableAttributes {
        message: Some(Some(new_message.clone())),
        ..Default::default()
    };

    let chat_workflow_item = chat_workflow_item.update(attributes, connection).unwrap();
    assert_eq!(chat_workflow_item.message, Some(new_message));
}
