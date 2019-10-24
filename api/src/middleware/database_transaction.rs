use actix_service::{Service, Transform};
use actix_web::error::Error as ActixWebError;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use actix_web::{FromRequest, HttpRequest};
use db::Connection;
use diesel::connection::TransactionManager;
use diesel::Connection as DieselConnection;
use errors::BigNeonError;
use futures::future::{ok, FutureResult};
use futures::{Future, Poll};

pub trait RequestConnection {
    fn connection(&self) -> Result<Connection, ActixWebError>;
}

impl RequestConnection for HttpRequest {
    fn connection(&self) -> Result<Connection, ActixWebError> {
        Ok(Connection::from_request(&self, &())?)
    }
}

pub struct DatabaseTransaction {}

impl DatabaseTransaction {
    pub fn new() -> DatabaseTransaction {
        DatabaseTransaction {}
    }
}
impl<S, B> Transform<S> for DatabaseTransaction
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = DatabaseTransactionMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(DatabaseTransactionMiddleware { service })
    }
}

pub struct DatabaseTransactionMiddleware<S> {
    service: S,
}

impl<S, B> Service for DatabaseTransactionMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, request: ServiceRequest) -> Self::Future {
        Box::new(self.service.call(request).and_then(|response| {
            if let Some(connection) = request.extensions().get::<Connection>() {
                let connection_object = connection.get();

                let transaction_response = match response.error() {
                    Some(_) => connection_object
                        .transaction_manager()
                        .rollback_transaction(connection_object),
                    None => connection_object
                        .transaction_manager()
                        .commit_transaction(connection_object),
                };

                match transaction_response {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Diesel Error: {}", e.description());
                        let error: BigNeonError = e.into();
                        return Ok(error.error_response());
                    }
                }
            };

            Ok(response)
        }))
    }
}
