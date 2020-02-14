use diesel::prelude::*;
use models::*;
use rand::prelude::*;
use test::builders::*;
use uuid::Uuid;

pub struct ChatWorkflowBuilder<'a> {
    name: String,
    status: ChatWorkflowStatus,
    initial_chat_workflow_item_id: Option<Uuid>,
    connection: &'a PgConnection,
}

impl<'a> ChatWorkflowBuilder<'a> {
    pub fn new(connection: &'a PgConnection) -> Self {
        let x: u32 = random();
        ChatWorkflowBuilder {
            name: format!("Chat Workflow {}", x).to_string(),
            status: ChatWorkflowStatus::Published,
            initial_chat_workflow_item_id: None,
            connection,
        }
    }

    pub fn draft(mut self) -> Self {
        self.status = ChatWorkflowStatus::Draft;
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn with_initial_chat_workflow_item(mut self, initial_chat_workflow_item: &ChatWorkflowItem) -> Self {
        self.initial_chat_workflow_item_id = Some(initial_chat_workflow_item.id);
        self
    }

    pub fn finish(&mut self) -> ChatWorkflow {
        let mut chat_workflow = ChatWorkflow::create(self.name.clone())
            .commit(None, self.connection)
            .unwrap();

        if self.initial_chat_workflow_item_id.is_some() || self.status == ChatWorkflowStatus::Published {
            let initial_chat_workflow_item_id = self.initial_chat_workflow_item_id.unwrap_or_else(|| {
                ChatWorkflowItemBuilder::new(self.connection)
                    .with_chat_workflow(&chat_workflow)
                    .finish()
                    .id
            });

            chat_workflow = chat_workflow
                .update(
                    ChatWorkflowEditableAttributes {
                        initial_chat_workflow_item_id: Some(Some(initial_chat_workflow_item_id)),
                        ..Default::default()
                    },
                    self.connection,
                )
                .unwrap();
        }

        if self.status == ChatWorkflowStatus::Published {
            chat_workflow = chat_workflow.publish(None, self.connection).unwrap();
        }

        chat_workflow
    }
}
