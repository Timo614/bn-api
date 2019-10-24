use actix_http::Payload;
use actix_web::{FromRequest, HttpRequest, Result};
use db::*;
use diesel;
use diesel::connection::TransactionManager;
use diesel::Connection as DieselConnection;
use diesel::PgConnection;
use errors::BigNeonError;
use std::sync::Arc;

pub struct Connection {
    pub inner: Arc<ConnectionType>,
}

impl From<ConnectionType> for Connection {
    fn from(connection_type: ConnectionType) -> Self {
        Connection {
            inner: Arc::new(connection_type),
        }
    }
}

impl From<PgConnection> for Connection {
    fn from(connection: PgConnection) -> Self {
        ConnectionType::Pg(connection).into()
    }
}

impl Connection {
    pub fn get(&self) -> &PgConnection {
        match *self.inner {
            ConnectionType::Pg(ref connection) => connection,
            ConnectionType::R2D2(ref connection) => connection,
        }
    }

    pub fn commit_transaction(&self) -> Result<(), diesel::result::Error> {
        self.get()
            .transaction_manager()
            .commit_transaction(self.get())
    }

    pub fn begin_transaction(&self) -> Result<(), diesel::result::Error> {
        self.get()
            .transaction_manager()
            .begin_transaction(self.get())
    }

    pub fn rollback_transaction(&self) -> Result<(), diesel::result::Error> {
        self.get()
            .transaction_manager()
            .rollback_transaction(self.get())
    }
}

impl Clone for Connection {
    fn clone(&self) -> Self {
        Connection {
            inner: self.inner.clone(),
        }
    }
}

impl FromRequest for Connection {
    type Error = BigNeonError;
    type Future = Result<Self, Self::Error>;
    type Config = ();

    fn from_request(request: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        if let Some(connection) = request.extensions().get::<Connection>() {
            return Ok(connection.clone());
        }

        let state: Data<AppState> = request.app_data::<AppState>().unwrap();
        let connection = state.database.get_connection()?;
        {
            let connection_object = connection.get();
            connection_object
                .transaction_manager()
                .begin_transaction(connection_object)?;
        }

        request.extensions_mut().insert(connection.clone());
        Ok(connection)
    }
}
