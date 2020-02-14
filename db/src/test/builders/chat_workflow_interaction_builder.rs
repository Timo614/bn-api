use diesel::prelude::*;
use models::*;
use serde_json::Value;
use test::builders::*;
use uuid::Uuid;

pub struct ChatWorkflowInteractionBuilder<'a> {
    chat_workflow_item_id: Option<Uuid>,
    chat_workflow_response_id: Option<Uuid>,
    chat_session_id: Option<Uuid>,
    input: Option<Value>,
    connection: &'a PgConnection,
}

impl<'a> ChatWorkflowInteractionBuilder<'a> {
    pub fn new(connection: &'a PgConnection) -> Self {
        ChatWorkflowInteractionBuilder {
            chat_workflow_item_id: None,
            chat_workflow_response_id: None,
            chat_session_id: None,
            input: None,
            connection,
        }
    }

    pub fn with_chat_workflow_item(mut self, chat_workflow_item: &ChatWorkflowItem) -> Self {
        self.chat_workflow_item_id = Some(chat_workflow_item.id);
        self
    }

    pub fn with_chat_workflow_response(mut self, chat_workflow_response: &ChatWorkflowResponse) -> Self {
        self.chat_workflow_response_id = Some(chat_workflow_response.id);
        self
    }

    pub fn with_chat_session(mut self, chat_session: &ChatSession) -> Self {
        self.chat_session_id = Some(chat_session.id);
        self
    }

    pub fn with_input(mut self, input: Value) -> Self {
        self.input = Some(input);
        self
    }

    pub fn finish(&mut self) -> ChatWorkflowInteraction {
        let chat_workflow_item = self
            .chat_workflow_item_id
            .map(|id| ChatWorkflowItem::find(id, self.connection).unwrap())
            .unwrap_or_else(|| ChatWorkflowItemBuilder::new(self.connection).finish());
        let chat_workflow_response_id = self.chat_workflow_response_id.unwrap_or_else(|| {
            ChatWorkflowResponseBuilder::new(self.connection)
                .with_chat_workflow_item(&chat_workflow_item)
                .finish()
                .id
        });
        let chat_workflow = chat_workflow_item.chat_workflow(self.connection).unwrap();
        let chat_session_id = self.chat_session_id.unwrap_or_else(|| {
            ChatSessionBuilder::new(self.connection)
                .with_chat_workflow(&chat_workflow)
                .finish()
                .id
        });

        ChatWorkflowInteraction::create(
            chat_workflow_item.id,
            chat_workflow_response_id,
            chat_session_id,
            self.input.clone(),
        )
        .commit(self.connection)
        .unwrap()
    }
}
