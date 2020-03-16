use diesel::prelude::*;
use models::*;
use rand::prelude::*;
use std::collections::HashMap;
use test::builders::fee_schedule_builder::FeeScheduleBuilder;
use uuid::Uuid;

pub struct OrganizationBuilder<'a> {
    name: String,
    members: HashMap<Uuid, Roles>,
    connection: &'a PgConnection,
    fee_schedule: Option<FeeSchedule>,
    event_fee_in_cents: Option<i64>,
    company_fee_in_cents: Option<i64>,
    client_fee_in_cents: Option<i64>,
    cc_fee_percent: Option<f32>,
    use_address: bool,
    additional_fee: i64,
    timezone: Option<String>,
    settlement_type: Option<SettlementTypes>,
    is_allowed_to_refund: bool,
}

impl<'a> OrganizationBuilder<'a> {
    pub fn new(connection: &'a PgConnection) -> OrganizationBuilder {
        let x: u32 = random();
        OrganizationBuilder {
            name: format!("test org{}", x).into(),
            members: HashMap::new(),
            fee_schedule: None,
            connection,
            use_address: false,
            event_fee_in_cents: None,
            company_fee_in_cents: None,
            client_fee_in_cents: None,
            cc_fee_percent: None,
            additional_fee: 0,
            timezone: None,
            settlement_type: None,
            is_allowed_to_refund: false,
        }
    }

    pub fn with_member(mut self, user: &User, role: Roles) -> OrganizationBuilder<'a> {
        self.members.insert(user.id.clone(), role);
        self
    }

    pub fn is_allowed_to_refund(mut self) -> OrganizationBuilder<'a> {
        self.is_allowed_to_refund = true;
        self
    }

    pub fn with_address(mut self) -> OrganizationBuilder<'a> {
        self.use_address = true;
        self
    }

    pub fn with_timezone(mut self, timezone: String) -> Self {
        self.timezone = Some(timezone);
        self
    }

    pub fn with_settlement_type(mut self, settlement_type: SettlementTypes) -> Self {
        self.settlement_type = Some(settlement_type);
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_fees(mut self) -> OrganizationBuilder<'a> {
        self.fee_schedule = Some(FeeScheduleBuilder::new(self.connection).finish(None));
        self
    }

    //    pub fn with_fee_schedule(mut self, fee_schedule: &FeeSchedule) -> OrganizationBuilder<'a> {
    //        self.fee_schedule = Some(fee_schedule.clone());
    //        self
    //    }

    pub fn with_event_fee(mut self) -> Self {
        self.event_fee_in_cents = Some(250);
        self.company_fee_in_cents = Some(100);
        self.client_fee_in_cents = Some(150);
        self
    }

    pub fn with_cc_fee(mut self, cc_fee_percent: f32) -> Self {
        self.cc_fee_percent = Some(cc_fee_percent);
        self
    }
    pub fn with_max_additional_fee(mut self, amount: i64) -> Self {
        self.additional_fee = amount;
        self
    }
    pub fn finish(mut self) -> Organization {
        if self.fee_schedule.is_none() {
            let x: u32 = random();
            let fee_schedule = FeeSchedule::create(
                Uuid::nil(),
                format!("{} fees.{}", self.name, x).into(),
                vec![NewFeeScheduleRange {
                    min_price_in_cents: 1,
                    company_fee_in_cents: 20,
                    client_fee_in_cents: 30,
                }],
            )
            .commit(None, self.connection);
            self.fee_schedule = Some(fee_schedule.unwrap());
        }

        let mut organization = Organization::create(&self.name, self.fee_schedule.unwrap().id);
        organization.settlement_type = self.settlement_type;
        let mut organization = organization
            .commit(None, "encryption_key", None, self.connection)
            .unwrap();

        let event_fee_update = OrganizationEditableAttributes {
            company_event_fee_in_cents: self.company_fee_in_cents,
            client_event_fee_in_cents: self.client_fee_in_cents,
            cc_fee_percent: self.cc_fee_percent,
            max_additional_fee_in_cents: Some(self.additional_fee),
            timezone: self.timezone,
            is_allowed_to_refund: Some(self.is_allowed_to_refund),
            ..Default::default()
        };

        organization = organization
            .update(event_fee_update, None, &"encryption_key".to_string(), self.connection)
            .unwrap();

        for (user_id, role) in self.members {
            OrganizationUser::create(organization.id, user_id, vec![role])
                .commit(self.connection)
                .unwrap();
        }

        Wallet::create_for_organization(organization.id, String::from("Default wallet"), self.connection).unwrap();

        if self.use_address {
            let mut attrs: OrganizationEditableAttributes = Default::default();

            attrs.address = Some(<String>::from("Test Address"));
            attrs.city = Some(<String>::from("Test Address"));
            attrs.state = Some(<String>::from("Test state"));
            attrs.country = Some(<String>::from("Test country"));
            attrs.postal_code = Some(<String>::from("0124"));
            attrs.phone = Some(<String>::from("+27123456789"));

            organization = organization
                .update(attrs, None, &"encryption_key".to_string(), self.connection)
                .unwrap();
        }
        organization
    }
}
