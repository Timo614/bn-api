use actix_http::Payload;
use actix_web::error::*;
use actix_web::{FromRequest, HttpRequest};
use futures::Future;
use models::*;

impl FromRequest for RequestInfo {
    type Error = Error;
    type Future = Result<Self, Self::Error>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        Ok(match req.headers().get("User-Agent") {
            Some(user_agent_header) => RequestInfo {
                user_agent: user_agent_header.to_str().ok().map(|ua| ua.to_string()),
            },
            None => RequestInfo { user_agent: None },
        })
    }
}
