use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use bigneon_db::utils::dates;
use bigneon_db::utils::errors::DatabaseError;
use chrono::{Duration, NaiveDateTime, Utc};
use diesel;
use diesel::prelude::*;
use diesel::sql_types;
use diesel::RunQueryDsl;
use serde_json::Value;
use std::collections::HashMap;

#[test]
fn create_commit() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let chat_workflow = project.create_chat_workflow().draft().finish();
    let chat_workflow_item = project
        .create_chat_workflow_item()
        .with_chat_workflow(&chat_workflow)
        .finish();
    let chat_session = ChatSession::create(user.id, chat_workflow.id, None);

    // Fails due to the chat workflow being in draft status
    let result = chat_session.commit(connection);
    assert_eq!(
        result,
        DatabaseError::business_process_error("Unable to start chat session, workflow is in draft status")
    );
    let chat_workflow = chat_workflow
        .update(
            ChatWorkflowEditableAttributes {
                initial_chat_workflow_item_id: Some(Some(chat_workflow_item.id)),
                ..Default::default()
            },
            connection,
        )
        .unwrap();
    assert!(chat_workflow.publish(None, connection).is_ok());

    // Once published, it succeeds and chat is given an expires_at value 15 minutes into the future
    let chat_session = chat_session.commit(connection).unwrap();
    assert!(!chat_session.id.is_nil());
    assert_eq!(chat_session.user_id, user.id);
    assert_eq!(chat_session.chat_workflow_id, chat_workflow.id);
    assert_eq!(chat_session.context, json!({}));
    assert!(chat_session.expires_at.is_some());
    assert!(
        chat_session.expires_at.unwrap() > (Utc::now().naive_utc() + Duration::minutes(14) + Duration::seconds(30))
    );
    assert!(chat_session.expires_at.unwrap() <= (Utc::now().naive_utc() + Duration::minutes(15)));
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_session = project.create_chat_session().finish();
    assert_eq!(Ok(chat_session.clone()), ChatSession::find(chat_session.id, connection));
}

#[test]
fn is_expired() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_session = project.create_chat_session().finish();
    assert!(!chat_session.is_expired());

    // Set chat session to have an expiration a second in the past
    diesel::sql_query(
        r#"
        UPDATE chat_sessions
        SET expires_at = $2
        WHERE id = $1;
        "#,
    )
    .bind::<sql_types::Uuid, _>(chat_session.id)
    .bind::<sql_types::Timestamp, _>(dates::now().add_seconds(-1).finish())
    .execute(connection)
    .unwrap();
    // Reload chat session
    let chat_session = ChatSession::find(chat_session.id, connection).unwrap();
    assert!(chat_session.is_expired());
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();

    let chat_session = project.create_chat_session().finish();

    let new_context = json!({"context": true});
    let attributes = ChatSessionEditableAttributes {
        context: Some(new_context.clone()),
        ..Default::default()
    };

    let chat_session = chat_session.update(attributes, connection).unwrap();
    assert_eq!(chat_session.context, new_context);
}

#[test]
fn next_expires_at() {
    let fifteen_minutes = dates::now().add_minutes(15).finish();
    let next_expires_at = ChatSession::next_expires_at();
    // 2 second wiggle room for any test slowness
    assert!(next_expires_at.signed_duration_since(fifteen_minutes).num_seconds() < 2);
}

#[test]
fn add_value_to_context() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut chat_session = project.create_chat_session().finish();
    let mut context: HashMap<String, Value> = HashMap::new();
    // Reload chat session for updated context
    assert_eq!(context, serde_json::from_value(chat_session.context.clone()).unwrap());

    // Initial value added
    chat_session = chat_session
        .add_value_to_context("Context", json!(true), connection)
        .unwrap();
    context.insert("Context".to_string(), json!(true));
    assert_eq!(context, serde_json::from_value(chat_session.context.clone()).unwrap());

    // Replacing the value
    chat_session = chat_session
        .add_value_to_context("Context", json!(false), connection)
        .unwrap();
    *context.entry("Context".to_string()).or_insert(json!(false)) = json!(false);

    assert_eq!(context, serde_json::from_value(chat_session.context.clone()).unwrap());

    // With additional context
    chat_session = chat_session
        .add_value_to_context("MoreContext", json!(12), connection)
        .unwrap();
    context.insert("MoreContext".to_string(), json!(12));
    assert_eq!(context, serde_json::from_value(chat_session.context.clone()).unwrap());
}

#[test]
fn process_response() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow = project.create_chat_workflow().finish();
    let mut chat_session = project.create_chat_session().finish();
    let question_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .with_chat_workflow(&chat_workflow)
        .finish();
    let answer_chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&question_chat_workflow_item)
        .finish();
    let answer_chat_workflow_response2 = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&question_chat_workflow_item)
        .finish();
    let message_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Message)
        .with_chat_workflow(&chat_workflow)
        .finish();
    let noop_chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&message_chat_workflow_item)
        .finish();
    let render_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Render)
        .with_render_type(ChatWorkflowItemRenderType::OrderDetails)
        .with_chat_workflow(&chat_workflow)
        .finish();
    let noop_chat_workflow_response2 = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&render_chat_workflow_item)
        .finish();
    let last_input_key = "last_input".to_string();
    let answer_selection_key = "answer_selection".to_string();

    let number_of_interactions = ChatWorkflowInteraction::find_by_chat_session(&chat_session, connection)
        .unwrap()
        .len();
    assert_eq!(number_of_interactions, 0);

    // Failure due to no response provided
    let response = chat_session.process_response(&question_chat_workflow_item, None, None, connection);
    assert_eq!(
        response,
        DatabaseError::business_process_error("Unable to process response, no valid input provided")
    );
    let number_of_interactions = ChatWorkflowInteraction::find_by_chat_session(&chat_session, connection)
        .unwrap()
        .len();
    assert_eq!(number_of_interactions, 0);

    // Failure due to invalid response provided
    let response =
        chat_session.process_response(&question_chat_workflow_item, None, Some(json!("Invalid")), connection);
    assert_eq!(
        response,
        DatabaseError::business_process_error("Unable to process response, no valid input provided")
    );
    let number_of_interactions = ChatWorkflowInteraction::find_by_chat_session(&chat_session, connection)
        .unwrap()
        .len();
    assert_eq!(number_of_interactions, 0);

    let mut context: HashMap<String, Value> = HashMap::new();
    let none_input: Option<Value> = None;
    context.insert(last_input_key.clone(), json!("Invalid"));
    assert_eq!(context, serde_json::from_value(chat_session.context.clone()).unwrap());

    // Failure due to invalid response for item
    let response = chat_session.process_response(
        &message_chat_workflow_item,
        Some(answer_chat_workflow_response.clone()),
        None,
        connection,
    );
    assert_eq!(
        response,
        DatabaseError::business_process_error(
            "Unable to process transfer, chat workflow response not valid for chat workflow item"
        )
    );
    let number_of_interactions = ChatWorkflowInteraction::find_by_chat_session(&chat_session, connection)
        .unwrap()
        .len();
    assert_eq!(number_of_interactions, 0);
    *context.get_mut(&last_input_key).unwrap() = json!(none_input); // Did not store input due to the error
    assert_eq!(context, serde_json::from_value(chat_session.context.clone()).unwrap());

    // No failure for noop response like a message if no input provided, interaction is logged
    assert!(chat_session
        .process_response(&message_chat_workflow_item, None, None, connection)
        .is_ok());
    let interactions = ChatWorkflowInteraction::find_by_chat_session(&chat_session, connection).unwrap();
    assert_eq!(interactions.len(), 1);
    let interaction = &interactions[0];
    update_chat_interaction_timing(interaction, dates::now().add_seconds(-50).finish(), connection);
    assert_eq!(interaction.chat_workflow_item_id, message_chat_workflow_item.id);
    assert_eq!(interaction.chat_workflow_response_id, noop_chat_workflow_response.id);
    assert_eq!(interaction.input, None);
    assert_eq!(context, serde_json::from_value(chat_session.context.clone()).unwrap());

    // A chat workflow interaction is processed if an input is provided
    assert!(chat_session
        .process_response(
            &question_chat_workflow_item,
            Some(answer_chat_workflow_response.clone()),
            None,
            connection
        )
        .is_ok());
    let interactions = ChatWorkflowInteraction::find_by_chat_session(&chat_session, connection).unwrap();
    assert_eq!(interactions.len(), 2);
    let interaction = &interactions[1];
    update_chat_interaction_timing(interaction, dates::now().add_seconds(-40).finish(), connection);
    assert_eq!(interaction.chat_workflow_item_id, question_chat_workflow_item.id);
    assert_eq!(interaction.chat_workflow_response_id, answer_chat_workflow_response.id);
    assert_eq!(interaction.input, answer_chat_workflow_response.answer_value.clone());
    context.insert(
        answer_selection_key.clone(),
        json!(Some(answer_chat_workflow_response.answer_value.clone())),
    );
    *context.get_mut(&last_input_key).unwrap() = interaction.input.clone().unwrap();
    assert_eq!(context, serde_json::from_value(chat_session.context.clone()).unwrap());

    // Can also be provided by the answer input
    assert!(chat_session
        .process_response(
            &question_chat_workflow_item,
            None,
            Some(answer_chat_workflow_response2.answer_value.clone().unwrap()),
            connection
        )
        .is_ok());
    let interactions = ChatWorkflowInteraction::find_by_chat_session(&chat_session, connection).unwrap();
    assert_eq!(interactions.len(), 3);
    let interaction = &interactions[2];
    update_chat_interaction_timing(interaction, dates::now().add_seconds(-30).finish(), connection);
    assert_eq!(interaction.chat_workflow_item_id, question_chat_workflow_item.id);
    assert_eq!(interaction.chat_workflow_response_id, answer_chat_workflow_response2.id);
    assert_eq!(interaction.input, answer_chat_workflow_response2.answer_value);
    *context.get_mut(&answer_selection_key).unwrap() = json!(Some(answer_chat_workflow_response2.answer_value.clone()));
    *context.get_mut(&last_input_key).unwrap() = interaction.input.clone().unwrap();
    *context.get_mut(&last_input_key).unwrap() = answer_chat_workflow_response2.answer_value.clone().unwrap();
    assert_eq!(context, serde_json::from_value(chat_session.context.clone()).unwrap());

    // Render type set on render type processing is stored in context in addition
    assert!(chat_session
        .process_response(&render_chat_workflow_item, None, None, connection)
        .is_ok());
    let interactions = ChatWorkflowInteraction::find_by_chat_session(&chat_session, connection).unwrap();
    assert_eq!(interactions.len(), 4);
    let interaction = &interactions[3];
    assert_eq!(interaction.chat_workflow_item_id, render_chat_workflow_item.id);
    assert_eq!(interaction.chat_workflow_response_id, noop_chat_workflow_response2.id);
    assert_eq!(interaction.input, None);
    context.insert("render_type".to_string(), json!(render_chat_workflow_item.render_type));

    *context.get_mut(&last_input_key).unwrap() = json!(none_input);
    assert_eq!(context, serde_json::from_value(chat_session.context.clone()).unwrap());
}

#[test]
fn select_response() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow = project.create_chat_workflow().finish();
    let chat_session = project.create_chat_session().finish();
    let question_chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .with_chat_workflow(&chat_workflow)
        .finish();
    let question_chat_workflow_item2 = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .with_chat_workflow(&chat_workflow)
        .finish();
    let answer_chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&question_chat_workflow_item)
        .with_next_chat_workflow_item(&question_chat_workflow_item2)
        .finish();
    let _answer_chat_workflow_response2 = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&question_chat_workflow_item2)
        .finish();

    assert!(chat_session
        .select_response(
            &question_chat_workflow_item,
            &answer_chat_workflow_response,
            None,
            connection
        )
        .is_ok());
    let interactions = ChatWorkflowInteraction::find_by_chat_session(&chat_session, connection).unwrap();
    assert_eq!(interactions.len(), 1);
    let interaction = &interactions[0];
    update_chat_interaction_timing(interaction, dates::now().add_seconds(-50).finish(), connection);
    assert_eq!(interaction.chat_workflow_item_id, question_chat_workflow_item.id);
    assert_eq!(interaction.chat_workflow_response_id, answer_chat_workflow_response.id);
    assert_eq!(interaction.input, None);
    let chat_session = ChatSession::find(chat_session.id, connection).unwrap();
    assert_eq!(chat_session.chat_workflow_item_id, question_chat_workflow_item2.id);
}

fn update_chat_interaction_timing(
    interaction: &ChatWorkflowInteraction,
    new_created_at: NaiveDateTime,
    connection: &PgConnection,
) {
    diesel::sql_query(
        r#"
        UPDATE chat_sessions
        SET created_at = $2
        WHERE id = $1;
        "#,
    )
    .bind::<sql_types::Uuid, _>(interaction.id)
    .bind::<sql_types::Timestamp, _>(new_created_at)
    .execute(connection)
    .unwrap();
}
