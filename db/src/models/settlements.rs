use chrono::{Datelike, Duration, NaiveDateTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use dev::times;
use diesel::dsl::select;
use diesel::prelude::*;
use diesel::sql_types::{Nullable, Timestamp, Uuid as dUuid};
use diesel::{self, dsl};
use models::*;
use schema::{settlement_adjustments, settlements};
use utils::errors::ConvertToDatabaseError;
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use utils::pagination::*;
use uuid::Uuid;
use validators;

sql_function!(fn process_settlement_for_event(settlement_id: dUuid, event_id: dUuid, start_time: Nullable<Timestamp>, end_time: Nullable<Timestamp>));

pub const DEFAULT_SETTLEMENT_PERIOD_IN_DAYS: i64 = 7;

#[derive(Associations, Debug, Identifiable, PartialEq, Queryable, Serialize, Deserialize, Clone)]
#[table_name = "settlements"]
pub struct Settlement {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub status: SettlementStatus,
    pub comment: Option<String>,
    pub only_finished_events: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "settlements"]
pub struct NewSettlement {
    pub organization_id: Uuid,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub status: SettlementStatus,
    pub comment: Option<String>,
    pub only_finished_events: bool,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct DisplaySettlement {
    pub settlement: Settlement,
    pub adjustments: Vec<SettlementAdjustment>,
    pub event_entries: Vec<EventGroupedSettlementEntry>,
}

impl NewSettlement {
    pub fn commit(&self, user: Option<User>, conn: &PgConnection) -> Result<Settlement, DatabaseError> {
        self.validate_record()?;

        let settlement = DatabaseError::wrap(
            ErrorCode::InsertError,
            "Could not create new settlement",
            diesel::insert_into(settlements::table)
                .values(self)
                .get_result::<Settlement>(conn),
        )?;

        settlement.create_entries(conn)?;

        DomainEvent::create(
            DomainEventTypes::SettlementReportProcessed,
            format!("Settlement processed"),
            Tables::Organizations,
            Some(settlement.organization_id),
            user.map(|u| u.id),
            Some(json!({"settlement_id": settlement.id})),
        )
        .commit(conn)?;

        Ok(settlement)
    }

    fn validate_record(&self) -> Result<(), DatabaseError> {
        let validation_errors = validators::append_validation_error(
            Ok(()),
            "start_time",
            validators::n_date_valid(
                Some(self.start_time),
                Some(self.end_time),
                "end_time_before_start_time",
                "End time must be after start time",
                "start_time",
                "end_time",
            ),
        );

        Ok(validation_errors?)
    }
}

impl Settlement {
    pub fn create(
        organization_id: Uuid,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
        status: SettlementStatus,
        comment: Option<String>,
        only_finished_events: bool,
    ) -> NewSettlement {
        NewSettlement {
            organization_id,
            start_time,
            end_time,
            status,
            comment,
            only_finished_events,
        }
    }

    pub fn create_next_finalize_settlements_domain_action(conn: &PgConnection) -> Result<(), DatabaseError> {
        let now = Utc::now().naive_utc();
        if let Some(upcoming_domain_action) =
            DomainAction::upcoming_domain_action(None, None, DomainActionTypes::FinalizeSettlements, conn)?
        {
            if upcoming_domain_action.scheduled_at > now {
                return DatabaseError::business_process_error("Finalize settlements domain action is already pending");
            }
        }

        let mut action = DomainAction::create(
            None,
            DomainActionTypes::FinalizeSettlements,
            None,
            json!({}),
            None,
            None,
        );
        action.schedule_at(Settlement::next_finalization_date()?);
        action.commit(conn)?;

        Ok(())
    }

    pub fn next_finalization_date() -> Result<NaiveDateTime, DatabaseError> {
        let timezone = "America/Los_Angeles"
            .to_string()
            .parse::<Tz>()
            .map_err(|e| DatabaseError::business_process_error::<Tz>(&e).unwrap_err())?;
        let now = timezone.from_utc_datetime(&Utc::now().naive_utc());
        let today = timezone.ymd(now.year(), now.month(), now.day()).and_hms(0, 0, 0);
        let days_since_monday = today.naive_local().weekday().num_days_from_monday();
        if days_since_monday < 2 || (days_since_monday == 2 && now.naive_local().hour() < 12) {
            let next_period_date = now + Duration::days(2 - days_since_monday as i64);
            Ok(timezone
                .ymd(
                    next_period_date.year(),
                    next_period_date.month(),
                    next_period_date.day(),
                )
                .and_hms(12, 0, 0)
                .naive_utc())
        } else {
            let next_period_date = now
                + Duration::days(
                    // Week - days since Monday + 2 days (Mon -> Wed)
                    7 - today.naive_local().weekday().num_days_from_monday() as i64 + 2,
                );
            Ok(timezone
                .ymd(
                    next_period_date.year(),
                    next_period_date.month(),
                    next_period_date.day(),
                )
                .and_hms(12, 0, 0)
                .naive_utc())
        }
    }

    pub fn find_last_settlement_for_organization(
        organization: &Organization,
        conn: &PgConnection,
    ) -> Result<Option<Settlement>, DatabaseError> {
        settlements::table
            .filter(settlements::organization_id.eq(organization.id))
            .order_by(settlements::end_time.desc())
            .get_result(conn)
            .optional()
            .to_db_error(ErrorCode::QueryError, "Could not load settlement")
    }

    pub fn process_settlement_for_organization(
        organization: &Organization,
        settlement_period_in_days: Option<u32>,
        conn: &PgConnection,
    ) -> Result<Settlement, DatabaseError> {
        let last_processed_settlement = Settlement::find_last_settlement_for_organization(organization, conn)?;

        let end_time = organization.next_settlement_date(settlement_period_in_days)?
            - Duration::days(
                settlement_period_in_days
                    .map(|p| p as i64)
                    .unwrap_or(DEFAULT_SETTLEMENT_PERIOD_IN_DAYS),
            )
            - Duration::seconds(1);
        let start_time = if let Some(settlement) = last_processed_settlement {
            settlement.end_time + Duration::seconds(1)
        } else {
            end_time
                - Duration::days(
                    settlement_period_in_days
                        .map(|p| p as i64)
                        .unwrap_or(DEFAULT_SETTLEMENT_PERIOD_IN_DAYS),
                )
                + Duration::seconds(1)
        };

        let settlement = Settlement::create(
            organization.id,
            start_time,
            end_time,
            SettlementStatus::PendingSettlement,
            None,
            organization.settlement_type == SettlementTypes::PostEvent,
        )
        .commit(None, conn)?;

        Ok(settlement)
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Settlement, DatabaseError> {
        settlements::table
            .filter(settlements::id.eq(id))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load Settlement")
    }

    pub fn for_display(&self, conn: &PgConnection) -> Result<DisplaySettlement, DatabaseError> {
        let adjustments = settlement_adjustments::table
            .filter(settlement_adjustments::settlement_id.eq(self.id))
            .get_results::<SettlementAdjustment>(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load Settlement Adjustments")?;

        Ok(DisplaySettlement {
            settlement: self.clone(),
            adjustments,
            event_entries: SettlementEntry::find_for_settlement_by_event(self, conn)?,
        })
    }

    pub fn adjustments(&self, conn: &PgConnection) -> Result<Vec<SettlementAdjustment>, DatabaseError> {
        settlement_adjustments::table
            .filter(settlement_adjustments::settlement_id.eq(self.id))
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load settlement adjustments")
    }

    pub fn destroy(self, conn: &PgConnection) -> Result<usize, DatabaseError> {
        diesel::delete(settlements::table.filter(settlements::id.eq(self.id)))
            .execute(conn)
            .to_db_error(ErrorCode::DeleteError, "Error removing settlement")
    }

    fn create_entries(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let ending_events = Event::get_all_events_ending_between(
            self.organization_id,
            self.start_time,
            self.end_time,
            EventStatus::Published,
            conn,
        )?;

        for event in ending_events {
            if self.only_finished_events {
                self.create_entries_from_event_transactions(&event, true, conn)?;
            }
            // Mark ending events as having been settled
            event.mark_settled(conn)?;
        }

        // Attempt to record any transactions from this period (ignoring any events that have not ended if applicable)
        // Transactions already settled above with the post event settlement will be ignored
        for event in
            Event::get_all_events_with_transactions_between(self.organization_id, self.start_time, self.end_time, conn)?
        {
            if !self.only_finished_events || event.event_end.unwrap_or(times::infinity()) < self.end_time {
                self.create_entries_from_event_transactions(&event, false, conn)?;
            }
        }

        Ok(())
    }

    pub fn create_entries_from_event_transactions(
        &self,
        event: &Event,
        include_all_transactions: bool,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        select(process_settlement_for_event(
            self.id,
            event.id,
            if include_all_transactions {
                None
            } else {
                Some(self.start_time)
            },
            if include_all_transactions {
                None
            } else {
                Some(self.end_time)
            },
        ))
        .execute(conn)
        .to_db_error(ErrorCode::InsertError, "Could not process settlement")?;

        Ok(())
    }

    pub fn finalize_settlements(conn: &PgConnection) -> Result<(), DatabaseError> {
        diesel::update(settlements::table.filter(settlements::status.eq(SettlementStatus::PendingSettlement)))
            .set((
                settlements::status.eq(SettlementStatus::FinalizedSettlement),
                settlements::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not finalize settlements")?;

        Ok(())
    }

    pub fn find_for_organization(
        organization_id: Uuid,
        limit: Option<u32>,
        page: Option<u32>,
        hide_early_settlements: bool,
        conn: &PgConnection,
    ) -> Result<Payload<Settlement>, DatabaseError> {
        let limit = limit.unwrap_or(20);
        let page = page.unwrap_or(0);

        let mut query = settlements::table
            .filter(settlements::organization_id.eq(organization_id))
            .into_boxed();

        if hide_early_settlements {
            query = query.filter(settlements::status.eq(SettlementStatus::FinalizedSettlement));
        }

        let (settlements, record_count): (Vec<Settlement>, i64) = query
            .order_by(settlements::start_time.desc())
            .select(settlements::all_columns)
            .paginate(page as i64)
            .per_page(limit as i64)
            .load_and_count_pages(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading settlement")?;

        let payload = Payload::from_data(settlements, page, limit, Some(record_count as u64));
        Ok(payload)
    }
}
