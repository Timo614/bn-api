use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use bigneon_db::utils::errors::DatabaseError;

#[test]
fn for_display() {
    let project = TestProject::new();
    let connection = project.get_connection();

    // Draft chat workflow, lacks an initial_chat_workflow_item_id
    let chat_workflow = project.create_chat_workflow().draft().finish();
    assert_eq!(
        chat_workflow.for_display(connection).unwrap(),
        DisplayChatWorkflow {
            id: chat_workflow.id,
            name: chat_workflow.name.clone(),
            status: chat_workflow.status,
            tree: json!({}),
            created_at: chat_workflow.created_at,
            updated_at: chat_workflow.updated_at,
            initial_chat_workflow_item_id: chat_workflow.initial_chat_workflow_item_id,
        }
    );

    // Chat workflow with initial_chat_workflow_item_id
    let chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .with_chat_workflow(&chat_workflow)
        .finish();
    let chat_workflow_item2 = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Message)
        .with_chat_workflow(&chat_workflow)
        .finish();
    let attributes = ChatWorkflowEditableAttributes {
        initial_chat_workflow_item_id: Some(Some(chat_workflow_item.id)),
        ..Default::default()
    };
    let chat_workflow = chat_workflow.update(attributes, connection).unwrap();
    project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .finish();
    project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .with_next_chat_workflow_item(&chat_workflow_item2)
        .finish();
    project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item2)
        .finish();
    assert_eq!(
        chat_workflow.for_display(connection).unwrap(),
        DisplayChatWorkflow {
            id: chat_workflow.id,
            name: chat_workflow.name.clone(),
            status: chat_workflow.status,
            tree: json!(chat_workflow_item.for_display(&mut vec![], connection).unwrap()),
            created_at: chat_workflow.created_at,
            updated_at: chat_workflow.updated_at,
            initial_chat_workflow_item_id: chat_workflow.initial_chat_workflow_item_id,
        }
    );

    // Chat workflow with recursive loop
    let chat_workflow = project.create_chat_workflow().draft().finish();
    let chat_workflow_item = project
        .create_chat_workflow_item()
        .with_item_type(ChatWorkflowItemType::Question)
        .with_chat_workflow(&chat_workflow)
        .finish();
    let attributes = ChatWorkflowEditableAttributes {
        initial_chat_workflow_item_id: Some(Some(chat_workflow_item.id)),
        ..Default::default()
    };
    let chat_workflow = chat_workflow.update(attributes, connection).unwrap();
    project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .finish();
    // Loops with self
    project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .with_next_chat_workflow_item(&chat_workflow_item)
        .finish();
    let displayed_chat_workflow = chat_workflow.for_display(connection).unwrap();
    assert_eq!(
        displayed_chat_workflow.clone(),
        DisplayChatWorkflow {
            id: chat_workflow.id,
            name: chat_workflow.name.clone(),
            status: chat_workflow.status,
            tree: json!(chat_workflow_item.for_display(&mut vec![], connection).unwrap()),
            created_at: chat_workflow.created_at,
            updated_at: chat_workflow.updated_at,
            initial_chat_workflow_item_id: chat_workflow.initial_chat_workflow_item_id,
        }
    );
    let tree_json = displayed_chat_workflow.tree.to_string();
    assert!(tree_json.contains(&json!({"id": chat_workflow_item.id, "type": "multiple_references"}).to_string()));
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let chat_workflow = project.create_chat_workflow().draft().finish();
    let chat_workflow_item = project
        .create_chat_workflow_item()
        .with_chat_workflow(&chat_workflow)
        .finish();
    let chat_workflow_response = project
        .create_chat_workflow_response()
        .with_chat_workflow_item(&chat_workflow_item)
        .finish();

    let domain_events = DomainEvent::find(
        Tables::ChatWorkflows,
        Some(chat_workflow.id),
        Some(DomainEventTypes::ChatWorkflowDeleted),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    assert!(chat_workflow.destroy(Some(user.id), connection).is_ok());
    let domain_events = DomainEvent::find(
        Tables::ChatWorkflows,
        Some(chat_workflow.id),
        Some(DomainEventTypes::ChatWorkflowDeleted),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());

    assert!(ChatWorkflow::find(chat_workflow.id, connection).is_err());
    assert!(ChatWorkflowItem::find(chat_workflow_item.id, connection).is_err());
    assert!(ChatWorkflowResponse::find(chat_workflow_response.id, connection).is_err());
}

#[test]
fn publish() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let chat_workflow = project.create_chat_workflow().draft().finish();
    assert!(chat_workflow.initial_chat_workflow_item_id.is_none());
    let domain_events = DomainEvent::find(
        Tables::ChatWorkflows,
        Some(chat_workflow.id),
        Some(DomainEventTypes::ChatWorkflowPublished),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    // chat workflow fails to publish due to missing initial chat workflow item
    assert_eq!(
        chat_workflow.publish(Some(user.id), connection),
        DatabaseError::business_process_error("Initial chat workflow item must be set on workflow to publish",)
    );
    let domain_events = DomainEvent::find(
        Tables::ChatWorkflows,
        Some(chat_workflow.id),
        Some(DomainEventTypes::ChatWorkflowPublished),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    let chat_workflow_item = project
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

    // With chat workflow that includes initial chat workflow item, succeeds
    let chat_workflow = chat_workflow.publish(Some(user.id), connection).unwrap();
    assert_eq!(chat_workflow.status, ChatWorkflowStatus::Published);
    let domain_events = DomainEvent::find(
        Tables::ChatWorkflows,
        Some(chat_workflow.id),
        Some(DomainEventTypes::ChatWorkflowPublished),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());

    // Publishing again does nothing but return a clone of itself
    let updated_chat_workflow = chat_workflow.publish(Some(user.id), connection).unwrap();
    assert_eq!(updated_chat_workflow, chat_workflow);
    let domain_events = DomainEvent::find(
        Tables::ChatWorkflows,
        Some(chat_workflow.id),
        Some(DomainEventTypes::ChatWorkflowPublished),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
}

#[test]
fn create_commit() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let name = "Workflow Name".to_string();
    let chat_workflow = ChatWorkflow::create(name.clone())
        .commit(Some(user.id), connection)
        .unwrap();
    let domain_events = DomainEvent::find(
        Tables::ChatWorkflows,
        Some(chat_workflow.id),
        Some(DomainEventTypes::ChatWorkflowCreated),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());

    assert!(!chat_workflow.id.is_nil());
    assert_eq!(chat_workflow.name, name);
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow = project.create_chat_workflow().finish();
    assert_eq!(ChatWorkflow::find(chat_workflow.id, connection).unwrap(), chat_workflow);
}

#[test]
fn all() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let chat_workflow = project.create_chat_workflow().with_name("Workflow 1").finish();
    let chat_workflow2 = project.create_chat_workflow().with_name("Workflow 2").finish();
    assert_eq!(
        ChatWorkflow::all(connection).unwrap(),
        vec![chat_workflow, chat_workflow2]
    );
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();

    let chat_workflow = project.create_chat_workflow().finish();

    let new_name = "New name".to_string();
    let attributes = ChatWorkflowEditableAttributes {
        name: Some(new_name.clone()),
        ..Default::default()
    };

    let chat_workflow = chat_workflow.update(attributes, connection).unwrap();
    assert_eq!(chat_workflow.name, new_name);
}
