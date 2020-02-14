use diesel::prelude::*;
use models::*;
use rand::prelude::*;
use serde_json::Value;
use test::builders::*;
use uuid::Uuid;

pub struct ChatWorkflowResponseBuilder<'a> {
    chat_workflow_item_id: Option<Uuid>,
    response_type: ChatWorkflowResponseType,
    response: Option<String>,
    answer_value: Option<Value>,
    next_chat_workflow_item_id: Option<Uuid>,
    rank: i32,
    connection: &'a PgConnection,
}

impl<'a> ChatWorkflowResponseBuilder<'a> {
    pub fn new(connection: &'a PgConnection) -> Self {
        ChatWorkflowResponseBuilder {
            chat_workflow_item_id: None,
            response_type: ChatWorkflowResponseType::Noop,
            response: Some("This is the chatbot response message".to_string()),
            answer_value: None,
            next_chat_workflow_item_id: None,
            rank: 0,
            connection,
        }
    }

    pub fn with_answer_value(mut self, answer_value: Value) -> Self {
        self.response_type = ChatWorkflowResponseType::Answer;
        self.answer_value = Some(answer_value);
        self
    }

    pub fn with_chat_workflow_item(mut self, chat_workflow_item: &ChatWorkflowItem) -> Self {
        self.chat_workflow_item_id = Some(chat_workflow_item.id);
        self
    }

    pub fn with_next_chat_workflow_item(mut self, next_chat_workflow_item: &ChatWorkflowItem) -> Self {
        self.next_chat_workflow_item_id = Some(next_chat_workflow_item.id);
        self
    }

    pub fn with_response(mut self, response: &str) -> Self {
        self.response = Some(response.to_string());
        self
    }

    pub fn with_rank(mut self, rank: i32) -> Self {
        self.rank = rank;
        self
    }

    pub fn finish(mut self) -> ChatWorkflowResponse {
        let chat_workflow_item = self
            .chat_workflow_item_id
            .map(|id| ChatWorkflowItem::find(id, self.connection).unwrap())
            .unwrap_or_else(|| ChatWorkflowItemBuilder::new(self.connection).finish());
        if chat_workflow_item.item_type == ChatWorkflowItemType::Question {
            if self.answer_value.is_none() {
                let x: u32 = random();
                self = self.with_answer_value(json!(format!("Answer {}", x)));
            }
        }

        ChatWorkflowResponse::create(
            chat_workflow_item.id,
            self.response_type,
            self.response.clone(),
            self.answer_value.clone(),
            self.next_chat_workflow_item_id,
            self.rank,
        )
        .commit(self.connection)
        .unwrap()
    }
}
