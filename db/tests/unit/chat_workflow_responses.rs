use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use bigneon_db::utils::errors::ErrorCode::ValidationError;

#[test]
fn for_display() {
    let project = TestProject::new();
    let connection = project.get_connection();

    let chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .finish();
    let chat_workflow_item2 = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Message)
        .finish();
    let chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .with_rank(1)
        .finish();
    let chat_workflow_response2 = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .with_next_chat_workflow_item(&chat_workflow_item2)
        .with_rank(2)
        .finish();

    assert_eq!(
        chat_workflow_response.for_display(&mut vec![], connection).unwrap(),
        DisplayChatWorkflowResponse {
            id: chat_workflow_response.id,
            chat_workflow_item_id: chat_workflow_response.chat_workflow_item_id,
            response_type: chat_workflow_response.response_type,
            response: chat_workflow_response.response.clone(),
            answer_value: chat_workflow_response.answer_value.clone(),
            next_chat_workflow_item_id: chat_workflow_response.next_chat_workflow_item_id,
            rank: chat_workflow_response.rank,
            tree: json!({}),
            created_at: chat_workflow_response.created_at,
            updated_at: chat_workflow_response.updated_at,
        }
    );

    // Chat workflow item with responses
    assert_eq!(
        chat_workflow_response2.for_display(&mut vec![], connection).unwrap(),
        DisplayChatWorkflowResponse {
            id: chat_workflow_response2.id,
            chat_workflow_item_id: chat_workflow_response2.chat_workflow_item_id,
            response_type: chat_workflow_response2.response_type,
            response: chat_workflow_response2.response.clone(),
            answer_value: chat_workflow_response2.answer_value.clone(),
            next_chat_workflow_item_id: chat_workflow_response2.next_chat_workflow_item_id,
            rank: chat_workflow_response2.rank,
            tree: json!(chat_workflow_item2.for_display(&mut vec![], connection).unwrap()),
            created_at: chat_workflow_response2.created_at,
            updated_at: chat_workflow_response2.updated_at,
        }
    );

    // Chat workflow item with recursive loop
    let chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .finish();
    // Loops with self
    let chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .with_next_chat_workflow_item(&chat_workflow_item)
        .with_rank(1)
        .finish();
    let displayed_chat_workflow_response = chat_workflow_response.for_display(&mut vec![], connection).unwrap();
    assert_eq!(
        displayed_chat_workflow_response,
        DisplayChatWorkflowResponse {
            id: chat_workflow_response.id,
            chat_workflow_item_id: chat_workflow_response.chat_workflow_item_id,
            response_type: chat_workflow_response.response_type,
            response: chat_workflow_response.response.clone(),
            answer_value: chat_workflow_response.answer_value.clone(),
            next_chat_workflow_item_id: chat_workflow_response.next_chat_workflow_item_id,
            rank: chat_workflow_response.rank,
            tree: json!(chat_workflow_item
                .for_display(&mut vec![chat_workflow_item.id], connection)
                .unwrap()),
            created_at: chat_workflow_response.created_at,
            updated_at: chat_workflow_response.updated_at,
        }
    );
    let tree_json = displayed_chat_workflow_response.tree.to_string();
    assert!(tree_json.contains(&json!({"id": chat_workflow_item.id, "type": "multiple_references"}).to_string()));
}

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

    assert!(chat_workflow_response.destroy(connection).is_ok());
    assert!(ChatWorkflow::find(chat_workflow.id, connection).is_ok());
    assert!(ChatWorkflowItem::find(chat_workflow_item.id, connection).is_ok());
    assert!(ChatWorkflowResponse::find(chat_workflow_response.id, connection).is_err());
}

#[test]
fn create_with_validation_errors() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .finish();
    let chat_workflow_item2 = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .finish();
    let response_type = ChatWorkflowResponseType::Answer;
    let answer_value = "Answer 1".to_string();

    // Creates chat workflow without issue
    assert!(ChatWorkflowResponse::create(
        chat_workflow_item.id,
        response_type,
        None,
        Some(answer_value.clone()),
        None,
        1
    )
    .commit(connection)
    .is_ok());

    // Failure because answer value already in use
    let result = ChatWorkflowResponse::create(
        chat_workflow_item.id,
        response_type,
        None,
        Some(answer_value.clone()),
        None,
        1,
    )
    .commit(connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("answer_value"));
                assert_eq!(errors["answer_value"].len(), 1);
                assert_eq!(errors["answer_value"][0].code, "uniqueness");
                assert_eq!(
                    &errors["answer_value"][0].message.clone().unwrap().into_owned(),
                    "Answer value is already in use"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Additional attempt using a unique answer type does not lead to errors
    assert!(ChatWorkflowResponse::create(
        chat_workflow_item.id,
        response_type,
        None,
        Some("Answer 2".to_string()),
        None,
        1
    )
    .commit(connection)
    .is_ok());

    // Can succeed for other question type as it filters to chat workflow item for the uniqueness constraint
    assert!(ChatWorkflowResponse::create(
        chat_workflow_item2.id,
        response_type,
        None,
        Some(answer_value.clone()),
        None,
        1
    )
    .commit(connection)
    .is_ok());
}

#[test]
fn create_commit() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow_item = project.create_chat_workflow_item().finish();
    let response_type = ChatWorkflowResponseType::Noop;
    let response = Some("Example response".to_string());
    let chat_workflow_response =
        ChatWorkflowResponse::create(chat_workflow_item.id, response_type, response.clone(), None, None, 1)
            .commit(connection)
            .unwrap();

    assert!(!chat_workflow_response.id.is_nil());
    assert_eq!(chat_workflow_response.chat_workflow_item_id, chat_workflow_item.id);
    assert_eq!(chat_workflow_response.response_type, response_type);
    assert_eq!(chat_workflow_response.response, response);
    assert_eq!(chat_workflow_response.next_chat_workflow_item_id, None);
    assert_eq!(chat_workflow_response.rank, 1);

    // Add another workflow response set to rank 1, should move the original rank to 2
    let chat_workflow_response2 = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .with_response("Example response")
        .with_rank(1)
        .finish();

    assert_eq!(chat_workflow_response2.chat_workflow_item_id, chat_workflow_item.id);
    assert_eq!(chat_workflow_response2.response_type, response_type);
    assert_eq!(chat_workflow_response2.response, response);
    assert_eq!(chat_workflow_response2.next_chat_workflow_item_id, None);
    assert_eq!(chat_workflow_response2.rank, 1);

    let chat_workflow_response = ChatWorkflowResponse::find(chat_workflow_response.id, connection).unwrap();
    assert_eq!(chat_workflow_response.rank, 2);

    // Add another workflow response set to rank 2, should move original but not second added response rank
    let chat_workflow_response3 = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .with_response("Example response")
        .with_rank(2)
        .finish();

    assert_eq!(chat_workflow_response3.chat_workflow_item_id, chat_workflow_item.id);
    assert_eq!(chat_workflow_response3.response_type, response_type);
    assert_eq!(chat_workflow_response3.response, response);
    assert_eq!(chat_workflow_response3.next_chat_workflow_item_id, None);
    assert_eq!(chat_workflow_response3.rank, 2);

    let chat_workflow_response = ChatWorkflowResponse::find(chat_workflow_response.id, connection).unwrap();
    assert_eq!(chat_workflow_response.rank, 3);

    let chat_workflow_response2 = ChatWorkflowResponse::find(chat_workflow_response2.id, connection).unwrap();
    assert_eq!(chat_workflow_response2.rank, 1);
}

#[test]
fn response() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut chat_session = project.create_chat_session().finish();
    let chat_workflow_response = project
        .create_chat_workflow_response()
        .with_response("Testing normal response")
        .finish();
    assert_eq!(
        chat_workflow_response.response(&chat_session).unwrap(),
        chat_workflow_response.response.unwrap_or("".to_string())
    );

    let chat_workflow_response = project
        .create_chat_workflow_response()
        .with_response("Testing replacements, no valid {last_input} {error_message}")
        .finish();
    assert_eq!(
        chat_workflow_response.response(&chat_session).unwrap(),
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
        chat_workflow_response.response(&chat_session).unwrap(),
        "Testing replacements, no valid Input Error".to_string()
    );
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow_response = project.create_chat_workflow_response().finish();
    assert_eq!(
        ChatWorkflowResponse::find(chat_workflow_response.id, connection).unwrap(),
        chat_workflow_response
    );
}

#[test]
fn find_for_chat_workflow_item() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow_item = project.create_chat_workflow_item().finish();
    chat_workflow_item.responses(connection).unwrap()[0]
        .destroy(connection)
        .unwrap();
    let chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .with_rank(0)
        .finish();
    let chat_workflow_response2 = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .with_rank(1)
        .finish();
    let _chat_workflow_response3 = project.create_chat_workflow_response().finish();

    assert_eq!(
        ChatWorkflowResponse::find_for_chat_workflow_item(&chat_workflow_item, connection).unwrap(),
        vec![chat_workflow_response, chat_workflow_response2]
    );
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow_item = project.create_chat_workflow_item().finish();
    let chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .with_rank(1)
        .finish();
    let chat_workflow_response2 = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .with_rank(2)
        .finish();
    let new_response = "New response".to_string();
    let attributes = ChatWorkflowResponseEditableAttributes {
        response: Some(Some(new_response.clone())),
        ..Default::default()
    };
    let chat_workflow_response = chat_workflow_response.update(attributes, connection).unwrap();
    let chat_workflow_response2 = ChatWorkflowResponse::find(chat_workflow_response2.id, connection).unwrap();
    assert_eq!(chat_workflow_response.response, Some(new_response));
    assert_eq!(chat_workflow_response.rank, 1);
    assert_eq!(chat_workflow_response2.rank, 2);

    let attributes = ChatWorkflowResponseEditableAttributes {
        rank: Some(2),
        ..Default::default()
    };
    let chat_workflow_response = chat_workflow_response.update(attributes, connection).unwrap();
    let chat_workflow_response2 = ChatWorkflowResponse::find(chat_workflow_response2.id, connection).unwrap();
    assert_eq!(chat_workflow_response.rank, 2);
    assert_eq!(chat_workflow_response2.rank, 1);
}

#[test]
fn chat_workflow_item() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow_item = project.create_chat_workflow_item().finish();
    let chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .finish();

    assert_eq!(
        chat_workflow_item,
        chat_workflow_response.chat_workflow_item(connection).unwrap()
    );
}

#[test]
fn find_by_chat_workflow_item_and_answer_value() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow_item = project.create_chat_workflow_item().finish();
    let chat_workflow_response = project
        .create_chat_workflow_response()
        .with_answer_value("Yes!")
        .with_chat_workflow_item(&chat_workflow_item)
        .finish();
    let chat_workflow_response2 = project
        .create_chat_workflow_response()
        .with_answer_value("No way!")
        .with_rank(2)
        .with_chat_workflow_item(&chat_workflow_item)
        .finish();

    assert_eq!(
        chat_workflow_response,
        ChatWorkflowResponse::find_by_chat_workflow_item_and_answer_value(&chat_workflow_item, "Yes!", connection)
            .unwrap()
    );
    assert_eq!(
        chat_workflow_response2,
        ChatWorkflowResponse::find_by_chat_workflow_item_and_answer_value(&chat_workflow_item, "No way!", connection)
            .unwrap()
    );
    assert!(ChatWorkflowResponse::find_by_chat_workflow_item_and_answer_value(
        &chat_workflow_item,
        "Not real!",
        connection
    )
    .is_err());
}
