use actix_http::Payload;
use actix_web::error::*;
use actix_web::{FromRequest, HttpRequest};
use auth::claims;
use auth::user::User;
use bigneon_db::models::User as DbUser;
use errors::*;
use futures::Future;
use jwt::{decode, Validation};
use middleware::RequestConnection;

impl FromRequest for User {
    type Error = Error;
    type Future = Result<Self, Self::Error>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        match req.headers().get("Authorization") {
            Some(auth_header) => {
                let mut parts = auth_header
                    .to_str()
                    .map_err(|e| BigNeonError::from(e))?
                    .split_whitespace();
                if str::ne(parts.next().unwrap_or("None"), "Bearer") {
                    return Err(ErrorUnauthorized("Authorization scheme not supported"));
                }

                match parts.next() {
                    Some(access_token) => {
                        let token = decode::<claims::AccessToken>(
                            &access_token,
                            (*req.state()).config.token_secret.as_bytes(),
                            &Validation::default(),
                        )
                        .map_err(|e| BigNeonError::from(e))?;
                        let connection = req.connection()?;
                        match DbUser::find(token.claims.get_id()?, connection.get()) {
                            Ok(user) => Ok(User::new(user, req)
                                .map_err(|_| ErrorUnauthorized("User has invalid role data"))?),
                            Err(e) => Err(ErrorInternalServerError(e)),
                        }
                    }
                    None => {
                        return Err(ErrorUnauthorized("No access token provided"));
                    }
                }
            }
            None => Err(ErrorUnauthorized("Missing auth token")),
        }
    }
}
