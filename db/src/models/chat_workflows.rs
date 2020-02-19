use chrono::NaiveDateTime;
use diesel::expression::dsl;
use diesel::prelude::*;
use models::*;
use schema::chat_workflows;
use serde_json::Value;
use utils::errors::*;
use uuid::Uuid;

#[derive(Clone, Queryable, Identifiable, Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "chat_workflows"]
pub struct ChatWorkflow {
    pub id: Uuid,
    pub name: String,
    pub status: ChatWorkflowStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub initial_chat_workflow_item_id: Option<Uuid>,
}

#[derive(Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "chat_workflows"]
pub struct NewChatWorkflow {
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct DisplayChatWorkflow {
    pub id: Uuid,
    pub name: String,
    pub status: ChatWorkflowStatus,
    pub tree: Value,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub initial_chat_workflow_item_id: Option<Uuid>,
}

#[derive(AsChangeset, Default, Deserialize, Debug)]
#[table_name = "chat_workflows"]
pub struct ChatWorkflowEditableAttributes {
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub name: Option<String>,
    pub initial_chat_workflow_item_id: Option<Option<Uuid>>,
}

impl ChatWorkflow {
    pub fn publish(&self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<ChatWorkflow, DatabaseError> {
        if self.status == ChatWorkflowStatus::Published {
            // Do nothing, returns samer chat workflow
            return Ok(self.clone());
        } else if self.initial_chat_workflow_item_id.is_none() {
            return DatabaseError::business_process_error(
                "Initial chat workflow item must be set on workflow to publish",
            );
        }

        let chat_workflow: ChatWorkflow = diesel::update(self)
            .set((
                chat_workflows::status.eq(ChatWorkflowStatus::Published),
                chat_workflows::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not publish chat workflow")?;

        DomainEvent::create(
            DomainEventTypes::ChatWorkflowPublished,
            format!("Chat workflow '{}' published", &self.name),
            Tables::ChatWorkflows,
            Some(chat_workflow.id),
            current_user_id,
            Some(json!(self.for_display(conn)?)),
        )
        .commit(conn)?;

        Ok(chat_workflow)
    }

    pub fn create(name: String) -> NewChatWorkflow {
        NewChatWorkflow { name }
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<ChatWorkflow, DatabaseError> {
        chat_workflows::table
            .filter(chat_workflows::id.eq(id))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load chat workflow")
    }

    pub fn destroy(&self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<(), DatabaseError> {
        // To avoid triggering validation on cascading delete, remove initial chat workflow item
        let initial_chat_workflow_item_id: Option<Uuid> = None;
        diesel::update(self)
            .set(chat_workflows::initial_chat_workflow_item_id.eq(initial_chat_workflow_item_id))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update chat workflow")?;

        let display_chat_workflow = self.for_display(conn)?;
        diesel::delete(self)
            .execute(conn)
            .to_db_error(ErrorCode::DeleteError, "Failed to destroy workflow")?;

        DomainEvent::create(
            DomainEventTypes::ChatWorkflowDeleted,
            format!("Chat workflow '{}' deleted", &self.name),
            Tables::ChatWorkflows,
            Some(self.id),
            current_user_id,
            Some(json!(display_chat_workflow)),
        )
        .commit(conn)?;

        Ok(())
    }

    pub fn all(conn: &PgConnection) -> Result<Vec<ChatWorkflow>, DatabaseError> {
        chat_workflows::table
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load chat workflow")
    }

    pub fn for_display(&self, conn: &PgConnection) -> Result<DisplayChatWorkflow, DatabaseError> {
        Ok(DisplayChatWorkflow {
            id: self.id,
            name: self.name.clone(),
            status: self.status,
            tree: self.tree(conn)?,
            created_at: self.created_at,
            updated_at: self.updated_at,
            initial_chat_workflow_item_id: self.initial_chat_workflow_item_id,
        })
    }

    fn tree(&self, conn: &PgConnection) -> Result<Value, DatabaseError> {
        let mut rendered_chat_workflow_item_ids = Vec::new();
        match self.initial_chat_workflow_item_id {
            Some(initial_chat_workflow_item_id) => {
                let initial_chat_workflow_item = ChatWorkflowItem::find(initial_chat_workflow_item_id, conn)?;
                Ok(json!(
                    initial_chat_workflow_item.for_display(&mut rendered_chat_workflow_item_ids, conn)?
                ))
            }
            None => Ok(json!({})),
        }
    }

    pub fn update(
        &self,
        attributes: ChatWorkflowEditableAttributes,
        conn: &PgConnection,
    ) -> Result<ChatWorkflow, DatabaseError> {
        if self.status == ChatWorkflowStatus::Published {
            // Value is being removed on a published chat workflow
            if attributes.initial_chat_workflow_item_id == Some(None) {
                return DatabaseError::business_process_error(
                    "Initial chat workflow item cannot be removed on published chat workflow",
                );
            }
        }

        diesel::update(self)
            .set((attributes, chat_workflows::updated_at.eq(dsl::now)))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update chat workflow")
    }
}

impl NewChatWorkflow {
    pub fn commit(&self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<ChatWorkflow, DatabaseError> {
        let chat_workflow: ChatWorkflow = diesel::insert_into(chat_workflows::table)
            .values((self, chat_workflows::status.eq(ChatWorkflowStatus::Draft)))
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create chat workflow")?;

        DomainEvent::create(
            DomainEventTypes::ChatWorkflowCreated,
            format!("Chat workflow '{}' created", &self.name),
            Tables::ChatWorkflows,
            Some(chat_workflow.id),
            current_user_id,
            Some(json!(chat_workflow.for_display(conn)?)),
        )
        .commit(conn)?;

        Ok(chat_workflow)
    }
}
