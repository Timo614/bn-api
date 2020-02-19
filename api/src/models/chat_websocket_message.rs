use actix::prelude::*;
use errors::*;
use models::*;
use serde_json::Value;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub enum ChatWebsocketType {
    TicketRedemption,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ChatWebsocketMessage {
    pub payload: Value,
}

impl ChatWebsocketMessage {
    pub fn new(payload: Value) -> Self {
        Self { payload }
    }
}

impl Message for ChatWebsocketMessage {
    type Result = Result<(), BigNeonError>;
}

impl Handler<ChatWebsocketMessage> for ChatWebsocket {
    type Result = Result<(), BigNeonError>;

    fn handle(&mut self, message: ChatWebsocketMessage, context: &mut Self::Context) -> Self::Result {
        context.text(serde_json::to_string(&message.payload)?);
        Ok(())
    }
}
