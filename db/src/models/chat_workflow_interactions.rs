use chrono::NaiveDateTime;
use diesel::prelude::*;
use models::*;
use schema::chat_workflow_interactions;
use utils::errors::*;
use uuid::Uuid;

#[derive(Clone, Queryable, Identifiable, Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "chat_workflow_interactions"]
pub struct ChatWorkflowInteraction {
    pub id: Uuid,
    pub chat_workflow_item_id: Uuid,
    pub chat_workflow_response_id: Uuid,
    pub chat_session_id: Uuid,
    pub input: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Default, Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "chat_workflow_interactions"]
pub struct NewChatWorkflowInteraction {
    pub chat_workflow_item_id: Uuid,
    pub chat_workflow_response_id: Uuid,
    pub chat_session_id: Uuid,
    pub input: Option<String>,
}

impl ChatWorkflowInteraction {
    pub fn create(
        chat_workflow_item_id: Uuid,
        chat_workflow_response_id: Uuid,
        chat_session_id: Uuid,
        input: Option<String>,
    ) -> NewChatWorkflowInteraction {
        NewChatWorkflowInteraction {
            chat_workflow_item_id,
            chat_workflow_response_id,
            chat_session_id,
            input,
        }
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<ChatWorkflowInteraction, DatabaseError> {
        chat_workflow_interactions::table
            .filter(chat_workflow_interactions::id.eq(id))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load chat workflow interaction")
    }

    pub fn find_by_chat_session(
        chat_session: &ChatSession,
        conn: &PgConnection,
    ) -> Result<Vec<ChatWorkflowInteraction>, DatabaseError> {
        chat_workflow_interactions::table
            .filter(chat_workflow_interactions::chat_session_id.eq(chat_session.id))
            .order_by(chat_workflow_interactions::created_at)
            .get_results(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Unable to load chat workflow interactions by chat session",
            )
    }

    pub fn log_interaction(
        chat_session: &ChatSession,
        chat_workflow_item: &ChatWorkflowItem,
        chat_workflow_response: &ChatWorkflowResponse,
        input: Option<String>,
        conn: &PgConnection,
    ) -> Result<ChatWorkflowInteraction, DatabaseError> {
        ChatWorkflowInteraction::create(chat_workflow_item.id, chat_workflow_response.id, chat_session.id, input)
            .commit(conn)
    }
}

impl NewChatWorkflowInteraction {
    pub fn commit(&self, conn: &PgConnection) -> Result<ChatWorkflowInteraction, DatabaseError> {
        diesel::insert_into(chat_workflow_interactions::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create chat workflow interaction")
    }
}
