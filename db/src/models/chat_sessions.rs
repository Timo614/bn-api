use chrono::NaiveDateTime;
use chrono::Utc;
use diesel::expression::dsl;
use diesel::prelude::*;
use models::*;
use schema::{chat_sessions, chat_workflows};
use serde_json::Value;
use std::collections::HashMap;
use test::times;
use time::Duration;
use utils::errors::*;
use uuid::Uuid;

#[derive(Clone, Queryable, Identifiable, Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "chat_sessions"]
pub struct ChatSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub chat_workflow_id: Uuid,
    pub chat_workflow_item_id: Option<Uuid>,
    pub context: Value,
    pub expires_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Default, Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "chat_sessions"]
pub struct NewChatSession {
    pub user_id: Uuid,
    pub chat_workflow_id: Uuid,
    pub context: Value,
}

#[derive(AsChangeset, Default, Deserialize, Debug)]
#[table_name = "chat_sessions"]
pub struct ChatSessionEditableAttributes {
    pub chat_workflow_item_id: Option<Option<Uuid>>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub context: Option<Value>,
}

impl ChatSession {
    pub fn create(user_id: Uuid, chat_workflow_id: Uuid, context: Option<Value>) -> NewChatSession {
        NewChatSession {
            user_id,
            chat_workflow_id,
            context: context.unwrap_or(json!({})),
        }
    }

    pub fn find_active_for_user(user: &User, conn: &PgConnection) -> Result<ChatSession, DatabaseError> {
        chat_sessions::table
            .inner_join(chat_workflows::table.on(chat_sessions::chat_workflow_id.eq(chat_workflows::id)))
            .filter(chat_sessions::user_id.eq(user.id))
            .filter(chat_sessions::expires_at.ge(Utc::now().naive_utc()))
            .filter(chat_workflows::status.eq(ChatWorkflowStatus::Published))
            .select(chat_sessions::all_columns)
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load chat session")
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<ChatSession, DatabaseError> {
        chat_sessions::table
            .filter(chat_sessions::id.eq(id))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load chat session")
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at.unwrap_or(times::infinity()) < Utc::now().naive_utc()
    }

    pub fn update(
        &self,
        attributes: ChatSessionEditableAttributes,
        conn: &PgConnection,
    ) -> Result<ChatSession, DatabaseError> {
        if self.is_expired() {
            return DatabaseError::business_process_error("Unable to update chat session as it has expired");
        }

        diesel::update(self)
            .set((
                attributes,
                chat_sessions::expires_at.eq(Some(ChatSession::next_expires_at())),
                chat_sessions::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update chat session")
    }

    pub fn next_expires_at() -> NaiveDateTime {
        let now = Utc::now().naive_utc();
        now + Duration::minutes(15)
    }

    pub fn add_value_to_context(
        &self,
        field: &str,
        value: Value,
        conn: &PgConnection,
    ) -> Result<ChatSession, DatabaseError> {
        let mut context: HashMap<String, Value> = serde_json::from_value(self.context.clone())?;
        *context.entry(field.to_string()).or_insert(Value::Null) = value.clone();

        let attributes = ChatSessionEditableAttributes {
            context: Some(json!(context)),
            ..Default::default()
        };

        self.update(attributes, conn)
    }

    pub fn process_response(
        &mut self,
        chat_workflow_item: &ChatWorkflowItem,
        chat_workflow_response: Option<ChatWorkflowResponse>,
        last_input: Option<String>,
        conn: &PgConnection,
    ) -> Result<ChatWorkflowInteraction, DatabaseError> {
        let mut chat_workflow_response = chat_workflow_response;
        let mut input = last_input;
        if chat_workflow_response.is_none() && input.is_none() {
            // Check if a noop response is present
            chat_workflow_response = chat_workflow_item
                .find_chat_workflow_response_by_response_type(ChatWorkflowResponseType::Noop, conn)
                .optional()?;
        }

        if input.is_none() && chat_workflow_item.item_type == ChatWorkflowItemType::Question {
            if let Some(chat_workflow_response) = chat_workflow_response.clone() {
                input = chat_workflow_response.answer_value;
            }
        }
        self.context = self
            .add_value_to_context("last_input", json!(input.clone()), conn)?
            .context;

        if let Some(input) = input.as_ref() {
            if chat_workflow_response.is_none() {
                chat_workflow_response = ChatWorkflowResponse::find_by_chat_workflow_item_and_answer_value(
                    &chat_workflow_item,
                    &input,
                    conn,
                )
                .optional()?;
            }
        }

        if chat_workflow_response.is_none() {
            return DatabaseError::business_process_error("Unable to process response, no valid input provided");
        }

        match chat_workflow_item.item_type {
            ChatWorkflowItemType::Render => {
                self.context = self
                    .add_value_to_context("render_type", json!(chat_workflow_item.render_type), conn)?
                    .context;
                chat_workflow_response = Some(
                    chat_workflow_item
                        .find_chat_workflow_response_by_response_type(ChatWorkflowResponseType::Noop, conn)?,
                );
            }
            ChatWorkflowItemType::Question => {
                if let Some(chat_workflow_response) = chat_workflow_response.as_ref() {
                    input = chat_workflow_response.answer_value.clone();
                    self.context = self
                        .add_value_to_context("answer_selection", json!(chat_workflow_response.answer_value), conn)?
                        .context;
                }
            }
            _ => (),
        }

        if let Some(chat_workflow_response) = chat_workflow_response.as_ref() {
            if chat_workflow_item.response_valid(&chat_workflow_response, conn)? {
                return self.select_response(chat_workflow_item, &chat_workflow_response, input, conn);
            } else {
                return DatabaseError::business_process_error(
                    "Unable to process transfer, chat workflow response not valid for chat workflow item",
                );
            }
        }
        return DatabaseError::business_process_error("No chat workflow response found for input");
    }

    pub fn next_chat_workflow_item(&self, conn: &PgConnection) -> Result<Option<ChatWorkflowItem>, DatabaseError> {
        if let Some(chat_workflow_item_id) = self.chat_workflow_item_id {
            return Ok(Some(ChatWorkflowItem::find(chat_workflow_item_id, conn)?));
        }

        Ok(None)
    }

    pub fn select_response(
        &self,
        chat_workflow_item: &ChatWorkflowItem,
        chat_workflow_response: &ChatWorkflowResponse,
        input: Option<String>,
        conn: &PgConnection,
    ) -> Result<ChatWorkflowInteraction, DatabaseError> {
        self.update(
            ChatSessionEditableAttributes {
                chat_workflow_item_id: Some(chat_workflow_response.next_chat_workflow_item_id),
                ..Default::default()
            },
            conn,
        )?;

        ChatWorkflowInteraction::log_interaction(self, chat_workflow_item, chat_workflow_response, input, conn)
    }
}

impl NewChatSession {
    pub fn commit(&self, conn: &PgConnection) -> Result<ChatSession, DatabaseError> {
        let chat_workflow = ChatWorkflow::find(self.chat_workflow_id, conn)?;
        if chat_workflow.status == ChatWorkflowStatus::Draft {
            return DatabaseError::business_process_error("Unable to start chat session, workflow is in draft status");
        }

        match chat_workflow.initial_chat_workflow_item_id {
            Some(chat_workflow_item_id) => diesel::insert_into(chat_sessions::table)
                .values((
                    self,
                    chat_sessions::chat_workflow_item_id.eq(chat_workflow_item_id),
                    chat_sessions::expires_at.eq(Some(ChatSession::next_expires_at())),
                ))
                .get_result(conn)
                .to_db_error(ErrorCode::InsertError, "Could not create chat session"),
            None => {
                return DatabaseError::business_process_error(
                    "Unable to start chat session, workflow does not have an initial workflow item",
                )
            }
        }
    }
}
