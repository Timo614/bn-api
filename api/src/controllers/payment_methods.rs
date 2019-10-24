use actix_web::HttpResponse;
use auth::user::User;
use bigneon_db::models::ForDisplay;
use db::Connection;
use errors::*;

pub fn index(connection: Connection, auth_user: User) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let payment_methods = &auth_user.user.payment_methods(connection).for_display()?;
    Ok(HttpResponse::Ok().json(payment_methods))
}
