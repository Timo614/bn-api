// Websocket based on actix example https://github.com/actix/examples/blob/0.7/websocket/src/main.rs

use actix::prelude::*;
use actix_web::ws;
use models::*;
use server::AppState;
use std::time::{Duration, Instant};
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

pub struct ChatWebsocket {
    pub heartbeat: Instant,
    pub user_id: Uuid,
}

impl Actor for ChatWebsocket {
    type Context = ws::WebsocketContext<Self, AppState>;

    fn started(&mut self, context: &mut Self::Context) {
        self.heartbeat(context);
    }
}

impl ChatWebsocket {
    pub fn send_message(listener: &Addr<ChatWebsocket>, message: ChatWebsocketMessage) {
        if listener.connected() {
            if let Err(err) = listener.try_send(message.clone()) {
                error!("Websocket send error: {:?}", err);
            }
        }
    }

    pub fn new(user_id: Uuid) -> Self {
        Self {
            heartbeat: Instant::now(),
            user_id,
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
    fn started(&mut self, _ontext: &mut Self::Context) {}

    fn handle(&mut self, message: ws::Message, context: &mut Self::Context) {
        match message {
            ws::Message::Ping(message) => {
                self.heartbeat = Instant::now();
                context.pong(&message);
            }
            ws::Message::Pong(_) => {
                self.heartbeat = Instant::now();
            }
            ws::Message::Text(text) => context.text(text),
            ws::Message::Binary(bin) => context.binary(bin),
            ws::Message::Close(_) => {
                self.close(context);
            }
        }
    }
}
