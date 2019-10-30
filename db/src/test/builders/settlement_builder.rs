use chrono::NaiveDateTime;
use diesel::prelude::*;
use prelude::*;
use test::builders::*;
use utils::dates::IntoDateBuilder;
use uuid::Uuid;

pub struct SettlementBuilder<'a> {
    organization_id: Option<Uuid>,
    start_time: Option<NaiveDateTime>,
    end_time: Option<NaiveDateTime>,
    comment: Option<String>,
    only_finished_events: bool,
    connection: &'a PgConnection,
}

impl<'a> SettlementBuilder<'a> {
    pub fn new(connection: &PgConnection) -> SettlementBuilder {
        SettlementBuilder {
            organization_id: None,
            start_time: None,
            end_time: None,
            comment: None,
            only_finished_events: true,
            connection: connection,
        }
    }

    pub fn with_start_time(mut self, start_time: NaiveDateTime) -> Self {
        self.start_time = Some(start_time);
        self
    }

    pub fn with_end_time(mut self, end_time: NaiveDateTime) -> Self {
        self.end_time = Some(end_time);
        self
    }

    pub fn only_finished_events(mut self, only_finished_events: bool) -> Self {
        self.only_finished_events = only_finished_events;
        self
    }

    pub fn with_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    pub fn with_organization(mut self, organization: &Organization) -> Self {
        self.organization_id = Some(organization.id);
        self
    }

    pub fn finish(&mut self) -> Settlement {
        let organization_id = self
            .organization_id
            .or_else(|| Some(OrganizationBuilder::new(self.connection).finish().id))
            .unwrap();
        let start_time = self
            .start_time
            .unwrap_or(dates::now().add_days(-5).finish());
        let end_time = self
            .end_time
            .unwrap_or(start_time.into_builder().add_days(5).finish());

        Settlement::create(
            organization_id,
            start_time,
            end_time,
            SettlementStatus::PendingSettlement,
            self.comment.clone(),
            self.only_finished_events,
        )
        .commit(None, self.connection)
        .unwrap()
    }
}
