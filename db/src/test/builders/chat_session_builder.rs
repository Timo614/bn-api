use diesel::prelude::*;
use models::*;
use serde_json::Value;
use test::builders::*;
use uuid::Uuid;

pub struct ChatSessionBuilder<'a> {
    user_id: Option<Uuid>,
    chat_workflow_id: Option<Uuid>,
    context: Option<Value>,
    connection: &'a PgConnection,
}

impl<'a> ChatSessionBuilder<'a> {
    pub fn new(connection: &'a PgConnection) -> Self {
        ChatSessionBuilder {
            user_id: None,
            chat_workflow_id: None,
            context: None,
            connection,
        }
    }

    pub fn with_user(mut self, user: &User) -> Self {
        self.user_id = Some(user.id);
        self
    }

    pub fn with_chat_workflow(mut self, chat_workflow: &ChatWorkflow) -> Self {
        self.chat_workflow_id = Some(chat_workflow.id);
        self
    }

    pub fn finish(&mut self) -> ChatSession {
        let user_id = self
            .user_id
            .unwrap_or_else(|| UserBuilder::new(self.connection).finish().id);
        let chat_workflow_id = self
            .chat_workflow_id
            .unwrap_or_else(|| ChatWorkflowBuilder::new(self.connection).finish().id);

        let chat_session = ChatSession::create(user_id, chat_workflow_id, self.context.clone());

        chat_session.commit(self.connection).unwrap()
    }
}
