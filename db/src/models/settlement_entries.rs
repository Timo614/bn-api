use chrono::NaiveDateTime;
use diesel;
use diesel::expression::sql_literal::sql;
use diesel::prelude::*;
use diesel::sql_types::{Nullable, Text};
use itertools::Itertools;
use models::*;
use schema::{settlement_entries, ticket_types};
use utils::errors::ConvertToDatabaseError;
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use uuid::Uuid;

#[derive(
    AsChangeset,
    Clone,
    Debug,
    Deserialize,
    Identifiable,
    PartialEq,
    Queryable,
    QueryableByName,
    Serialize,
)]
#[table_name = "settlement_entries"]
pub struct SettlementEntry {
    pub id: Uuid,
    pub settlement_id: Uuid,
    pub event_id: Uuid,
    pub ticket_type_id: Option<Uuid>,
    pub face_value_in_cents: i64,
    pub revenue_share_value_in_cents: i64,
    pub online_sold_quantity: i64,
    pub fee_sold_quantity: i64,
    pub total_sales_in_cents: i64,
    pub settlement_entry_type: SettlementEntryTypes,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Queryable, Serialize)]
pub struct DisplaySettlementEntry {
    pub id: Uuid,
    pub settlement_id: Uuid,
    pub event_id: Uuid,
    pub ticket_type_id: Option<Uuid>,
    pub ticket_type_name: Option<String>,
    pub face_value_in_cents: i64,
    pub revenue_share_value_in_cents: i64,
    pub online_sold_quantity: i64,
    pub fee_sold_quantity: i64,
    pub total_sales_in_cents: i64,
    pub settlement_entry_type: SettlementEntryTypes,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Queryable, Serialize)]
pub struct EventGroupedSettlementEntry {
    pub event: DisplayEvent,
    pub entries: Vec<DisplaySettlementEntry>,
}

impl SettlementEntry {
    pub fn find_for_settlement_by_event(
        settlement: &Settlement,
        conn: &PgConnection,
    ) -> Result<Vec<EventGroupedSettlementEntry>, DatabaseError> {
        let entries: Vec<DisplaySettlementEntry> = settlement_entries::table
            .left_join(
                ticket_types::table
                    .on(settlement_entries::ticket_type_id.eq(ticket_types::id.nullable())),
            )
            .filter(settlement_entries::settlement_id.eq(settlement.id))
            .select((
                settlement_entries::id,
                settlement_entries::settlement_id,
                settlement_entries::event_id,
                settlement_entries::ticket_type_id,
                sql::<Nullable<Text>>("ticket_types.name AS ticket_type_name"),
                settlement_entries::face_value_in_cents,
                settlement_entries::revenue_share_value_in_cents,
                settlement_entries::online_sold_quantity,
                settlement_entries::fee_sold_quantity,
                settlement_entries::total_sales_in_cents,
                settlement_entries::settlement_entry_type,
                settlement_entries::created_at,
                settlement_entries::updated_at,
            ))
            .order_by(settlement_entries::event_id)
            .then_order_by(settlement_entries::settlement_entry_type.nullable().desc())
            .then_order_by(ticket_types::rank)
            .then_order_by(settlement_entries::face_value_in_cents)
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load Settlement Entries")?;

        let mut grouped_entries: Vec<EventGroupedSettlementEntry> = Vec::new();
        for (event_id, settlement_entries) in &entries
            .into_iter()
            .group_by(|settlement_entry| settlement_entry.event_id)
        {
            let event = Event::find(event_id, conn)?.for_display(conn)?;
            grouped_entries.push(EventGroupedSettlementEntry {
                event,
                entries: settlement_entries.collect_vec(),
            });
        }

        Ok(grouped_entries)
    }

    pub fn create(
        settlement_id: Uuid,
        settlement_entry_type: SettlementEntryTypes,
        event_id: Uuid,
        ticket_type_id: Option<Uuid>,
        face_value_in_cents: i64,
        revenue_share_value_in_cents: i64,
        online_sold_quantity: i64,
        fee_sold_quantity: i64,
        total_sales_in_cents: i64,
    ) -> NewSettlementEntry {
        NewSettlementEntry {
            settlement_id,
            event_id,
            ticket_type_id,
            face_value_in_cents,
            revenue_share_value_in_cents,
            online_sold_quantity,
            settlement_entry_type,
            fee_sold_quantity,
            total_sales_in_cents,
        }
    }
}

#[derive(Clone, Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "settlement_entries"]
pub struct NewSettlementEntry {
    pub settlement_id: Uuid,
    pub event_id: Uuid,
    pub ticket_type_id: Option<Uuid>,
    pub face_value_in_cents: i64,
    pub revenue_share_value_in_cents: i64,
    pub online_sold_quantity: i64,
    pub fee_sold_quantity: i64,
    pub total_sales_in_cents: i64,
    pub settlement_entry_type: SettlementEntryTypes,
}
impl NewSettlementEntry {
    pub fn commit(&self, conn: &PgConnection) -> Result<SettlementEntry, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::InsertError,
            "Could not create new settlement entry",
            diesel::insert_into(settlement_entries::table)
                .values(self)
                .get_result::<SettlementEntry>(conn),
        )
    }
}
