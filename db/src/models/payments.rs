use chrono::NaiveDateTime;
use diesel;
use diesel::expression::dsl;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use models::orders::Order;
use models::{PaymentMethods, PaymentStatus};
use schema::payments;
use utils::errors::ConvertToDatabaseError;
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use uuid::Uuid;

#[derive(Identifiable, Queryable)]
pub struct Payment {
    pub id: Uuid,
    order_id: Uuid,
    created_by: Uuid,
    status: String,
    payment_method: String,
    amount: i64,
    provider: String,
    external_reference: String,
    raw_data: Option<String>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl Payment {
    pub(crate) fn create(
        order_id: Uuid,
        created_by: Uuid,
        status: PaymentStatus,
        method: PaymentMethods,
        provider: String,
        external_reference: String,
        amount: i64,
        raw_data: Option<String>,
    ) -> NewPayment {
        NewPayment {
            order_id,
            created_by,
            status: status.to_string(),
            payment_method: method.to_string(),
            provider,
            external_reference,
            amount,
            raw_data,
        }
    }

    pub fn mark_complete(
        &self,
        _raw_data: String,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        diesel::update(
            payments::table.filter(
                payments::id
                    .eq(self.id)
                    .and(payments::updated_at.eq(self.updated_at)),
            ),
        ).set((
            payments::status.eq(PaymentStatus::Completed.to_string()),
            payments::updated_at.eq(dsl::now),
        )).execute(conn)
        .to_db_error(
            ErrorCode::UpdateError,
            "Could not change the status of payment to completed.",
        )?;

        self.order(conn)?.complete_if_fully_paid(conn)?;

        Ok(())
    }

    fn order(&self, conn: &PgConnection) -> Result<Order, DatabaseError> {
        use schema::*;
        orders::table
            .find(self.order_id)
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not retrieve order")
    }
}

#[derive(Insertable)]
#[table_name = "payments"]
pub struct NewPayment {
    order_id: Uuid,
    created_by: Uuid,
    status: String,
    payment_method: String,
    external_reference: String,
    amount: i64,
    provider: String,
    raw_data: Option<String>,
}

impl NewPayment {
    pub(crate) fn commit(self, conn: &PgConnection) -> Result<Payment, DatabaseError> {
        diesel::insert_into(payments::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create payment")
    }
}