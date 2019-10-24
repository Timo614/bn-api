use actix_http::Payload;
use actix_web::error::*;
use actix_web::{FromRequest, HttpRequest};
use auth::user::User;
use futures::Future;
use uuid::Uuid;

#[derive(Clone)]
pub struct OptionalUser(pub Option<User>);

impl FromRequest for OptionalUser {
    type Error = Error;
    type Future = Result<Self, Self::Error>;
    type Config = ();

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        // If auth header exists pass authorization errors back to client
        if let Some(_auth_header) = req.headers().get("Authorization") {
            return User::from_request(req, payload).map(|u| OptionalUser(Some(u)));
        }
        Ok(OptionalUser(None))
    }
}

impl OptionalUser {
    pub fn into_inner(self) -> Option<User> {
        self.0
    }
    pub fn id(&self) -> Option<Uuid> {
        self.0.as_ref().map(|u| u.id())
    }
}
