use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ChatWebsocketResponse {
    pub chat_workflow_response_id: Option<Uuid>,
    pub input: Option<String>,
}
