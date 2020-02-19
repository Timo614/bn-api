// Websocket based on actix example https://github.com/actix/examples/blob/0.7/websocket/src/main.rs

use actix::prelude::*;
use actix_web::ws;
use bigneon_db::prelude::*;
use db::Connection;
use models::*;
use server::AppState;
use std::time::{Duration, Instant};
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

pub struct ChatWebsocket {
    pub heartbeat: Instant,
    pub chat_session_id: Uuid,
    pub connection: Connection,
}

impl Actor for ChatWebsocket {
    type Context = ws::WebsocketContext<Self, AppState>;

    fn started(&mut self, context: &mut Self::Context) {
        self.heartbeat(context);

        self.send_next_chat_message(context);
    }
}

impl ChatWebsocket {
    pub fn send_next_chat_message(&self, context: &mut <Self as Actor>::Context) {
        let connection = self.connection.get();

        if let Some(chat_session) = ChatSession::find(self.chat_session_id, connection).ok() {
            if let Some(Some(chat_workflow_item)) = chat_session.next_chat_workflow_item(connection).ok() {
                ChatWebsocket::send_message(
                    &context.address(),
                    ChatWebsocketMessage::new(json!({
                        "chat_workflow_item": chat_workflow_item,
                        "responses": chat_workflow_item.responses(connection).ok().unwrap_or(Vec::new())
                    })),
                );
            }
        }
    }

    pub fn send_message(listener: &Addr<ChatWebsocket>, message: ChatWebsocketMessage) {
        if listener.connected() {
            if let Err(err) = listener.try_send(message.clone()) {
                error!("Websocket send error: {:?}", err);
            }
        }
    }

    pub fn new(chat_session_id: Uuid, connection: Connection) -> Self {
        Self {
            heartbeat: Instant::now(),
            chat_session_id,
            connection,
        }
    }

    fn heartbeat(&self, context: &mut <Self as Actor>::Context) {
        context.run_interval(HEARTBEAT_INTERVAL, |act, context| {
            context.ping("");
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
                act.close(context);
            }
        });
    }

    pub fn close(&mut self, context: &mut <Self as Actor>::Context) {
        context.stop();
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for ChatWebsocket {
    fn started(&mut self, _context: &mut Self::Context) {}

    fn handle(&mut self, message: ws::Message, context: &mut Self::Context) {
        match message {
            ws::Message::Ping(message) => {
                self.heartbeat = Instant::now();
                context.pong(&message);
            }
            ws::Message::Pong(_) => {
                self.heartbeat = Instant::now();
            }
            ws::Message::Text(text) => {
                let conn = self.connection.get();
                let chat_websocket_response: Option<ChatWebsocketResponse> = serde_json::from_str(&text).ok();
                if let Some(chat_websocket_response) = chat_websocket_response {
                    if let Some(mut chat_session) = ChatSession::find(self.chat_session_id, conn).ok() {
                        if let Some(Some(chat_workflow_item)) = chat_session.next_chat_workflow_item(conn).ok() {
                            let mut chat_workflow_response: Option<ChatWorkflowResponse> = None;
                            if let Some(chat_workflow_response_id) = chat_websocket_response.chat_workflow_response_id {
                                chat_workflow_response =
                                    ChatWorkflowResponse::find(chat_workflow_response_id, conn).ok();
                            }

                            match chat_session.process_response(
                                &chat_workflow_item,
                                chat_workflow_response,
                                chat_websocket_response.input,
                                conn,
                            ) {
                                Ok(_) => self.send_next_chat_message(context),
                                Err(e) => error!("{:?}", e),
                            }
                        } else {
                            context.text(json!({"error": "no current chat workflow item"}).to_string())
                        }
                    } else {
                        context.text(json!({"error": "no current chat session"}).to_string())
                    }
                } else {
                    context.text(json!({"error": "unable to parse chat WebSocket response"}).to_string())
                }
            }
            ws::Message::Binary(bin) => context.binary(bin),
            ws::Message::Close(_) => {
                self.close(context);
            }
        }
    }
}
