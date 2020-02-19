use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use bigneon_db::utils::dates;
use diesel;
use diesel::sql_types;
use diesel::RunQueryDsl;

#[test]
fn create_commit() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_session = project.create_chat_session().finish();
    let chat_workflow_item = project.create_chat_workflow_item().finish();
    let chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .finish();
    let input = "Input".to_string();
    let chat_workflow_interaction = ChatWorkflowInteraction::create(
        chat_workflow_item.id,
        chat_workflow_response.id,
        chat_session.id,
        Some(input.clone()),
    )
    .commit(connection)
    .unwrap();

    assert!(!chat_workflow_interaction.id.is_nil());
    assert_eq!(chat_workflow_interaction.chat_workflow_item_id, chat_workflow_item.id);
    assert_eq!(
        chat_workflow_interaction.chat_workflow_response_id,
        chat_workflow_response.id
    );
    assert_eq!(chat_workflow_interaction.chat_session_id, chat_session.id);
    assert_eq!(chat_workflow_interaction.input, Some(input));
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow_interaction = project.create_chat_workflow_interaction().finish();
    assert_eq!(
        ChatWorkflowInteraction::find(chat_workflow_interaction.id, connection).unwrap(),
        chat_workflow_interaction
    );
}

#[test]
fn log_interaction() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_session = project.create_chat_session().finish();
    let chat_workflow_item = project.create_chat_workflow_item().finish();
    let chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .finish();
    let input = "Input".to_string();
    assert_eq!(
        ChatWorkflowInteraction::find_by_chat_session(&chat_session, connection)
            .unwrap()
            .len(),
        0
    );

    assert!(ChatWorkflowInteraction::log_interaction(
        &chat_session,
        &chat_workflow_item,
        &chat_workflow_response,
        Some(input.clone()),
        connection
    )
    .is_ok());

    let chat_workflow_interactions = ChatWorkflowInteraction::find_by_chat_session(&chat_session, connection).unwrap();
    assert_eq!(chat_workflow_interactions.len(), 1);
    let chat_workflow_interaction = &chat_workflow_interactions[0];
    assert_eq!(chat_workflow_interaction.chat_session_id, chat_session.id);
    assert_eq!(chat_workflow_interaction.chat_workflow_item_id, chat_workflow_item.id);
    assert_eq!(
        chat_workflow_interaction.chat_workflow_response_id,
        chat_workflow_response.id
    );
    assert_eq!(chat_workflow_interaction.input, Some(input));
}

#[test]
fn find_by_chat_session() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_session = project.create_chat_session().finish();
    let chat_workflow_interaction = project
        .create_chat_workflow_interaction()
        .with_chat_session(&chat_session)
        .finish();
    let chat_workflow_interaction2 = project
        .create_chat_workflow_interaction()
        .with_chat_session(&chat_session)
        .finish();
    let chat_session2 = project.create_chat_session().finish();
    let chat_workflow_interaction3 = project
        .create_chat_workflow_interaction()
        .with_chat_session(&chat_session2)
        .finish();

    // Force created at to be in the past
    diesel::sql_query(
        r#"
        UPDATE chat_workflow_interactions
        SET created_at = $2
        WHERE id = $1;
        "#,
    )
    .bind::<sql_types::Uuid, _>(chat_workflow_interaction.id)
    .bind::<sql_types::Timestamp, _>(dates::now().add_seconds(-20).finish())
    .execute(connection)
    .unwrap();
    let chat_workflow_interaction = ChatWorkflowInteraction::find(chat_workflow_interaction.id, connection).unwrap();

    assert_eq!(
        ChatWorkflowInteraction::find_by_chat_session(&chat_session, connection).unwrap(),
        vec![chat_workflow_interaction, chat_workflow_interaction2]
    );
    assert_eq!(
        ChatWorkflowInteraction::find_by_chat_session(&chat_session2, connection).unwrap(),
        vec![chat_workflow_interaction3]
    );
}
