use diesel::prelude::*;
use models::*;
use test::builders::*;
use uuid::Uuid;

pub struct ChatWorkflowItemBuilder<'a> {
    chat_workflow_id: Option<Uuid>,
    item_type: ChatWorkflowItemType,
    message: Option<String>,
    render_type: Option<ChatWorkflowItemRenderType>,
    response_wait: Option<i32>,
    connection: &'a PgConnection,
}

impl<'a> ChatWorkflowItemBuilder<'a> {
    pub fn new(connection: &'a PgConnection) -> Self {
        ChatWorkflowItemBuilder {
            chat_workflow_id: None,
            item_type: ChatWorkflowItemType::Message,
            message: None,
            render_type: None,
            response_wait: None,
            connection,
        }
    }

    pub fn with_chat_workflow(mut self, chat_workflow: &ChatWorkflow) -> Self {
        self.chat_workflow_id = Some(chat_workflow.id);
        self
    }

    pub fn with_item_type(mut self, item_type: ChatWorkflowItemType) -> Self {
        self.item_type = item_type;

        if item_type == ChatWorkflowItemType::Render && self.render_type.is_none() {
            self = self.with_render_type(ChatWorkflowItemRenderType::OrderDetails);
        }

        self
    }

    pub fn with_render_type(mut self, render_type: ChatWorkflowItemRenderType) -> Self {
        self.render_type = Some(render_type);

        self
    }

    pub fn with_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_string());

        self
    }

    pub fn with_response_wait(mut self, response_wait: i32) -> Self {
        self.response_wait = Some(response_wait);

        self
    }

    pub fn finish(&mut self) -> ChatWorkflowItem {
        let chat_workflow_id = self
            .chat_workflow_id
            .unwrap_or_else(|| ChatWorkflowBuilder::new(self.connection).finish().id);

        ChatWorkflowItem::create(
            chat_workflow_id,
            self.item_type,
            self.message.clone(),
            self.render_type,
            self.response_wait,
        )
        .commit(self.connection)
        .unwrap()
    }
}
