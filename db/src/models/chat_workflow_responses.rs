use chrono::NaiveDateTime;
use diesel::dsl::{exists, select};
use diesel::expression::dsl;
use diesel::prelude::*;
use models::*;
use schema::{chat_workflow_items, chat_workflow_responses};
use utils::errors::*;
use uuid::Uuid;
use validator::*;
use validators::{self, *};

#[derive(Clone, Queryable, Identifiable, Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "chat_workflow_responses"]
pub struct ChatWorkflowResponse {
    pub id: Uuid,
    pub chat_workflow_item_id: Uuid,
    pub response_type: ChatWorkflowResponseType,
    pub response: Option<String>,
    pub answer_value: Option<Value>,
    pub next_chat_workflow_item_id: Option<Uuid>,
    pub rank: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "chat_workflow_responses"]
pub struct NewChatWorkflowResponse {
    pub chat_workflow_item_id: Uuid,
    pub response_type: ChatWorkflowResponseType,
    pub response: Option<String>,
    pub answer_value: Option<Value>,
    pub next_chat_workflow_item_id: Option<Uuid>,
    pub rank: i32,
}

#[derive(AsChangeset, Default, Deserialize, Debug)]
#[table_name = "chat_workflow_responses"]
pub struct ChatWorkflowResponseEditableAttributes {
    pub response_type: Option<ChatWorkflowResponseType>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub response: Option<Option<String>>,
    pub answer_value: Option<Option<Value>>,
    pub next_chat_workflow_item_id: Option<Option<Uuid>>,
    pub rank: Option<i32>,
}

impl ChatWorkflowResponse {
    pub fn create(
        chat_workflow_item_id: Uuid,
        response_type: ChatWorkflowResponseType,
        response: Option<String>,
        answer_value: Option<Value>,
        next_chat_workflow_item_id: Option<Uuid>,
        rank: i32,
    ) -> NewChatWorkflowResponse {
        NewChatWorkflowResponse {
            chat_workflow_item_id,
            response_type,
            response,
            answer_value,
            next_chat_workflow_item_id,
            rank,
        }
    }

    pub fn response(&self, chat_session: &ChatSession) -> Option<String> {
        let response = self.response.clone();
        if let Some(mut response) = response {
            for text in MESSAGE_REPLACEMENT_HELPERS {
                response = response.replace(
                    &format!("{{{}}}", text),
                    chat_session.context[text].as_str().unwrap_or(""),
                );
            }

            return Some(response);
        }

        None
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<ChatWorkflowResponse, DatabaseError> {
        chat_workflow_responses::table
            .filter(chat_workflow_responses::id.eq(id))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load chat workflow response")
    }

    pub fn find_for_chat_workflow_item(
        chat_workflow_item: &ChatWorkflowItem,
        conn: &PgConnection,
    ) -> Result<Vec<ChatWorkflowResponse>, DatabaseError> {
        chat_workflow_responses::table
            .filter(chat_workflow_responses::chat_workflow_item_id.eq(chat_workflow_item.id))
            .order_by(chat_workflow_responses::rank)
            .get_results(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Unable to load chat workflow responses for chat workflow item",
            )
    }

    pub fn chat_workflow_item(&self, conn: &PgConnection) -> Result<ChatWorkflowItem, DatabaseError> {
        chat_workflow_items::table
            .inner_join(
                chat_workflow_responses::table
                    .on(chat_workflow_responses::chat_workflow_item_id.eq(chat_workflow_items::id)),
            )
            .filter(chat_workflow_responses::id.eq(self.id))
            .select(chat_workflow_items::all_columns)
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load chat workflow item for response")
    }

    pub fn shift_other_rank(&self, from_rank: Option<i32>, conn: &PgConnection) -> Result<(), DatabaseError> {
        if Some(self.rank) == from_rank {
            return Ok(());
        }

        // Update other ticket types ranks
        for chat_workflow_response in self.chat_workflow_item(conn)?.responses(conn)? {
            if chat_workflow_response.id == self.id {
                continue;
            }

            if let Some(from_rank) = from_rank {
                if (from_rank < self.rank && chat_workflow_response.rank < from_rank)
                    || (from_rank > self.rank && chat_workflow_response.rank > from_rank)
                {
                    continue;
                }
            } else if from_rank.is_none() {
                if chat_workflow_response.rank < self.rank {
                    continue;
                }
            }

            let new_rank =
                if from_rank.is_some() && from_rank.unwrap() < self.rank && chat_workflow_response.rank <= self.rank {
                    chat_workflow_response.rank - 1
                } else {
                    chat_workflow_response.rank + 1
                };

            diesel::update(
                chat_workflow_responses::table.filter(chat_workflow_responses::id.eq(chat_workflow_response.id)),
            )
            .set((
                chat_workflow_responses::rank.eq(new_rank),
                chat_workflow_responses::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update chat workflow response ranks")?;
        }

        Ok(())
    }

    pub fn find_by_chat_workflow_item_and_answer_value(
        chat_workflow_item: &ChatWorkflowItem,
        answer_value: Value,
        conn: &PgConnection,
    ) -> Result<ChatWorkflowResponse, DatabaseError> {
        chat_workflow_responses::table
            .filter(chat_workflow_responses::chat_workflow_item_id.eq(chat_workflow_item.id))
            .filter(chat_workflow_responses::answer_value.eq(Some(answer_value)))
            .get_result(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Unable to load chat workflow responses for chat workflow item",
            )
    }

    pub fn update(
        &self,
        attributes: ChatWorkflowResponseEditableAttributes,
        conn: &PgConnection,
    ) -> Result<ChatWorkflowResponse, DatabaseError> {
        let chat_workflow_response: ChatWorkflowResponse = diesel::update(self)
            .set((attributes, chat_workflow_responses::updated_at.eq(dsl::now)))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update chat workflow response")?;
        chat_workflow_response.shift_other_rank(Some(self.rank), conn)?;
        Ok(chat_workflow_response)
    }

    pub fn destroy(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        diesel::delete(self)
            .execute(conn)
            .to_db_error(ErrorCode::DeleteError, "Failed to destroy workflow response")?;

        Ok(())
    }

    pub(crate) fn answer_value_unique(
        chat_workflow_item_id: Uuid,
        answer_value: Option<Value>,
        conn: &PgConnection,
    ) -> Result<Result<(), ValidationError>, DatabaseError> {
        if answer_value.is_none() {
            return Ok(Ok(()));
        }

        let answer_value_in_use = select(exists(
            chat_workflow_responses::table.filter(
                chat_workflow_responses::chat_workflow_item_id
                    .eq(chat_workflow_item_id)
                    .and(chat_workflow_responses::answer_value.eq(answer_value)),
            ),
        ))
        .get_result(conn)
        .to_db_error(
            ErrorCode::QueryError,
            "Could not check if answer value for chat workflow item was unique",
        )?;

        if answer_value_in_use {
            let validation_error = create_validation_error("uniqueness", "Answer value is already in use");
            return Ok(Err(validation_error));
        }

        Ok(Ok(()))
    }
}

impl NewChatWorkflowResponse {
    pub fn validate_record(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let validation_errors = validators::append_validation_error(
            Ok(()),
            "answer_value",
            ChatWorkflowResponse::answer_value_unique(self.chat_workflow_item_id, self.answer_value.clone(), conn)?,
        );
        Ok(validation_errors?)
    }

    pub fn commit(&self, conn: &PgConnection) -> Result<ChatWorkflowResponse, DatabaseError> {
        self.validate_record(conn)?;

        let result: ChatWorkflowResponse = diesel::insert_into(chat_workflow_responses::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create chat workflow response")?;

        result.shift_other_rank(None, conn)?;
        Ok(result)
    }
}
