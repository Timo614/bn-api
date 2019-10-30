use chrono::prelude::*;
use diesel;
use diesel::prelude::*;
use log::Level::{Error, Info};
use models::*;
use schema::domain_events;
use serde_json;
use std::cmp::Ordering;
use std::collections::HashMap;
use utils::errors::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
pub struct DomainEvent {
    pub id: Uuid,
    pub event_type: DomainEventTypes,
    pub display_text: String,
    pub event_data: Option<serde_json::Value>,
    pub main_table: Tables,
    pub main_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_id: Option<Uuid>,
    pub seq: i64,
    pub organization_id: Option<Uuid>,
}

impl PartialOrd for DomainEvent {
    fn partial_cmp(&self, other: &DomainEvent) -> Option<Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

impl DomainEvent {
    pub fn find_by_ids(
        ids: Vec<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<DomainEvent>, DatabaseError> {
        domain_events::table
            .filter(domain_events::id.eq_any(ids))
            .order_by(domain_events::created_at)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading domain events")
    }

    pub fn create(
        event_type: DomainEventTypes,
        display_text: String,
        main_table: Tables,
        main_id: Option<Uuid>,
        user_id: Option<Uuid>,
        event_data: Option<serde_json::Value>,
    ) -> NewDomainEvent {
        NewDomainEvent {
            event_type,
            display_text,
            event_data,
            main_table,
            main_id,
            user_id,
            created_at: None,
        }
    }

    pub fn find_after_seq(
        after_seq: i64,
        limit: u32,
        conn: &PgConnection,
    ) -> Result<Vec<DomainEvent>, DatabaseError> {
        domain_events::table
            .filter(domain_events::seq.gt(after_seq))
            .for_update()
            .skip_locked()
            .order_by(domain_events::seq.asc())
            .limit(limit as i64)
            .load(conn)
            .to_db_error(
                ErrorCode::QueryError,
                "Could not load domain events after seq",
            )
    }

    pub fn webhook_payloads(
        &self,
        front_end_url: &str,
        conn: &PgConnection,
    ) -> Result<Vec<HashMap<String, serde_json::Value>>, DatabaseError> {
        let mut result: Vec<HashMap<String, serde_json::Value>> = Vec::new();
        let main_id = self.main_id.ok_or_else(|| {
            DatabaseError::new(
                ErrorCode::BusinessProcessError,
                Some("Domain event id not present for webhook".to_string()),
            )
        })?;

        match self.event_type {
            DomainEventTypes::UserCreated => {
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                data.insert("timestamp".to_string(), json!(self.created_at.timestamp()));
                let user = User::find(main_id, conn)?;
                data.insert("webhook_event_type".to_string(), json!("user_created"));
                data.insert("user_id".to_string(), json!(user.id));
                data.insert("email".to_string(), json!(user.email));
                data.insert("phone".to_string(), json!(user.phone));
                result.push(data);
            }
            DomainEventTypes::TemporaryUserCreated => {
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                data.insert("timestamp".to_string(), json!(self.created_at.timestamp()));
                let temporary_user = TemporaryUser::find(main_id, conn)?;
                data.insert(
                    "webhook_event_type".to_string(),
                    json!("temporary_user_created"),
                );
                data.insert("user_id".to_string(), json!(temporary_user.id));
                data.insert("email".to_string(), json!(temporary_user.email));
                data.insert("phone".to_string(), json!(temporary_user.phone));
                result.push(data);
            }
            DomainEventTypes::PushNotificationTokenCreated => {
                // Guard against future publisher processing after deletion
                if let Some(push_notification_token) =
                    PushNotificationToken::find(main_id, conn).optional()?
                {
                    let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                    data.insert("timestamp".to_string(), json!(self.created_at.timestamp()));
                    data.insert(
                        "webhook_event_type".to_string(),
                        json!("user_device_tokens_added"),
                    );
                    data.insert(
                        "user_id".to_string(),
                        json!(push_notification_token.user_id),
                    );
                    data.insert(
                        "token_source".to_string(),
                        json!(push_notification_token.token_source),
                    );
                    data.insert("token".to_string(), json!(push_notification_token.token));
                    data.insert(
                        "last_used".to_string(),
                        json!(push_notification_token
                            .last_notification_at
                            .unwrap_or(push_notification_token.created_at)
                            .timestamp()),
                    );
                    result.push(data);
                }
            }
            DomainEventTypes::OrderCompleted => {
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                let order = Order::find(main_id, conn)?;
                if let Some(event) = order.events(conn)?.pop() {
                    DomainEvent::webhook_payload_event_data(&event, &mut data, conn)?;
                }
                data.insert("webhook_event_type".to_string(), json!("purchase_ticket"));
                data.insert(
                    "user_id".to_string(),
                    json!(order.on_behalf_of_user_id.unwrap_or(order.user_id)),
                );
                data.insert("timestamp".to_string(), json!(self.created_at.timestamp()));

                result.push(data);
            }
            DomainEventTypes::TransferTicketStarted
            | DomainEventTypes::TransferTicketCancelled
            | DomainEventTypes::TransferTicketCompleted => {
                // Sender is associated with their main account
                // Receiver is associated with the v3 UUID of their destination address
                // Receiver has a temp account made for them in the customer system on TransferTicketStarted
                let mut data: HashMap<String, serde_json::Value> = HashMap::new();
                let transfer = Transfer::find(main_id, conn).optional()?;
                // There is a historic bug where a transfer did not exist, unfortunately
                // will have to skip those
                if let Some(transfer) = transfer {
                    data.insert("direct_transfer".to_string(), json!(transfer.direct));
                    data.insert(
                        "number_of_tickets_transferred".to_string(),
                        json!(transfer.transfer_ticket_count(conn)?),
                    );

                    data.insert("timestamp".to_string(), json!(self.created_at.timestamp()));
                    let mut events = transfer.events(conn)?;
                    // TODO: lock down transfers to have only one event
                    if let Some(event) = events.pop() {
                        DomainEvent::webhook_payload_event_data(&event, &mut data, conn)?;
                    }
                    let mut recipient_data = data.clone();
                    let mut transferer_data = data;

                    DomainEvent::recipient_payload_data(
                        &transfer,
                        self.event_type,
                        &mut recipient_data,
                        front_end_url,
                        conn,
                    )?;
                    result.push(recipient_data);

                    DomainEvent::transferer_payload_data(
                        &transfer,
                        self.event_type,
                        &mut transferer_data,
                        conn,
                    )?;
                    result.push(transferer_data);
                } else {
                    jlog!(
                        Error,
                        "bigneon-db::models::domain_events",
                        "Could not find transfer for id",
                        { "domain_event": &self }
                    );
                }
            }
            _ => {
                return Err(DatabaseError::new(
                    ErrorCode::BusinessProcessError,
                    Some("Domain event type not supported".to_string()),
                ));
            }
        }

        Ok(result)
    }

    fn transferer_payload_data(
        transfer: &Transfer,
        event_type: DomainEventTypes,
        data: &mut HashMap<String, serde_json::Value>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        data.insert("user_id".to_string(), json!(transfer.source_user_id));
        data.insert(
            "recipient_id".to_string(),
            json!(transfer
                .destination_temporary_user_id
                .or(transfer.destination_user_id)),
        );

        let recipient = if let Some(destination_user_id) = transfer.destination_user_id {
            Some(User::find(destination_user_id, conn)?)
        } else {
            None
        };
        let mut email = recipient.clone().map(|r| r.email.clone()).unwrap_or(None);
        if let Some(transfer_message_type) = transfer.transfer_message_type {
            if transfer_message_type == TransferMessageType::Email {
                email = email.or(transfer.transfer_address.clone());
            }
        }
        let mut phone = recipient.clone().map(|r| r.phone.clone()).unwrap_or(None);
        if let Some(transfer_message_type) = transfer.transfer_message_type {
            if transfer_message_type == TransferMessageType::Phone {
                phone = phone.or(transfer.transfer_address.clone());
            }
        }

        data.insert(
            "webhook_event_type".to_string(),
            json!(match event_type {
                DomainEventTypes::TransferTicketCancelled => {
                    if transfer.cancelled_by_user_id == Some(transfer.source_user_id) {
                        "cancel_pending_transfer"
                    } else {
                        "initiated_transfer_declined"
                    }
                }
                DomainEventTypes::TransferTicketCompleted => "initiated_transfer_claimed",
                DomainEventTypes::TransferTicketStarted => "initiate_pending_transfer",
                _ => {
                    return Err(DatabaseError::new(
                        ErrorCode::BusinessProcessError,
                        Some("Domain event type not supported".to_string()),
                    ));
                }
            }),
        );

        data.insert(
            "recipient_first_name".to_string(),
            json!(recipient.map(|r| r.first_name)),
        );
        data.insert("recipient_email".to_string(), json!(email));
        data.insert("recipient_phone".to_string(), json!(phone));

        let transferer = User::find(transfer.source_user_id, conn)?;

        data.insert("transferer_email".to_string(), json!(transferer.email));
        data.insert("transferer_phone".to_string(), json!(transferer.phone));

        Ok(())
    }

    fn recipient_payload_data(
        transfer: &Transfer,
        event_type: DomainEventTypes,
        data: &mut HashMap<String, serde_json::Value>,
        front_end_url: &str,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let transferer = User::find(transfer.source_user_id, conn)?;
        let receive_tickets_url = transfer.receive_url(front_end_url.to_string(), conn)?;
        data.insert(
            "user_id".to_string(),
            json!(transfer
                .destination_temporary_user_id
                .or(transfer.destination_user_id)),
        );
        data.insert(
            "receive_tickets_url".to_string(),
            json!(receive_tickets_url),
        );
        data.insert(
            "transferer_first_name".to_string(),
            json!(transferer.first_name),
        );

        data.insert(
            "webhook_event_type".to_string(),
            json!(match event_type {
                DomainEventTypes::TransferTicketCancelled => {
                    if transfer.cancelled_by_user_id == Some(transfer.source_user_id) {
                        "received_transfer_cancelled"
                    } else {
                        "decline_pending_transfer"
                    }
                }
                DomainEventTypes::TransferTicketCompleted => "claim_pending_transfer",
                DomainEventTypes::TransferTicketStarted => "receive_pending_transfer",
                _ => {
                    return Err(DatabaseError::new(
                        ErrorCode::BusinessProcessError,
                        Some("Domain event type not supported".to_string()),
                    ));
                }
            }),
        );

        data.insert("transferer_email".to_string(), json!(transferer.email));
        data.insert("transferer_phone".to_string(), json!(transferer.phone));

        Ok(())
    }

    fn webhook_payload_event_data(
        event: &Event,
        data: &mut HashMap<String, serde_json::Value>,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        let organization = event.organization(conn)?;
        let venue = event.venue(conn)?;
        let localized_times = event.get_all_localized_times(venue.as_ref());
        data.insert("show_id".to_string(), json!(event.id));
        data.insert("show_event_name".to_string(), json!(event.name.clone()));

        data.insert(
            "show_start".to_string(),
            json!(event.event_start.map(|e| e.timestamp())),
        );

        data.insert(
            "show_end".to_string(),
            json!(event.event_end.map(|e| e.timestamp())),
        );

        if let Some(event_start) = localized_times.event_start {
            data.insert(
                "show_start_date".to_string(),
                json!(format!(
                    "{} {}",
                    event_start.format("%A,"),
                    event_start.format("%e %B %Y").to_string().trim()
                )),
            );
            data.insert(
                "show_start_time".to_string(),
                json!(event_start.format("%l:%M %p %Z").to_string().trim()),
            );
        }

        if let Some(door_time) = localized_times.door_time {
            data.insert(
                "show_doors_open_time".to_string(),
                json!(door_time.format("%l:%M %p %Z").to_string().trim()),
            );
        }

        if let Some(event_end) = localized_times.event_end {
            data.insert(
                "show_end_time".to_string(),
                json!(event_end.format("%l:%M %p %Z").to_string().trim()),
            );
        }
        if let Some(venue) = event.venue(conn)? {
            data.insert("show_venue_address".to_string(), json!(venue.address));
            data.insert("show_venue_city".to_string(), json!(venue.city));
            data.insert("show_venue_state".to_string(), json!(venue.state));
            data.insert("show_venue_country".to_string(), json!(venue.country));
            data.insert(
                "show_venue_postal_code".to_string(),
                json!(venue.postal_code),
            );
            data.insert(
                "show_venue_phone".to_string(),
                json!(venue.phone.unwrap_or("".to_string())),
            );
            data.insert("show_venue_name".to_string(), json!(venue.name));
            data.insert("show_timezone".to_string(), json!(venue.timezone));
        }
        data.insert("organization_id".to_string(), json!(organization.id));
        data.insert("organization_name".to_string(), json!(organization.name));
        Ok(())
    }

    pub fn find(
        main_table: Tables,
        main_id: Option<Uuid>,
        event_type: Option<DomainEventTypes>,
        conn: &PgConnection,
    ) -> Result<Vec<DomainEvent>, DatabaseError> {
        let mut query = domain_events::table
            .filter(domain_events::main_table.eq(main_table))
            .filter(domain_events::main_id.eq(main_id))
            .into_boxed();

        if let Some(event_type) = event_type {
            query = query.filter(domain_events::event_type.eq(event_type));
        }

        query
            .order_by(domain_events::created_at)
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load domain events")
    }

    pub fn post_processing(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        if let Some(main_id) = self.main_id {
            match self.event_type {
                DomainEventTypes::EventInterestCreated => {
                    let event = Event::find(main_id, conn)?;
                    let organization = event.organization(conn)?;

                    if let Some(user_id) = self.user_id {
                        organization.log_interaction(user_id, Utc::now().naive_utc(), conn)?;
                    }
                }
                DomainEventTypes::OrderRefund | DomainEventTypes::OrderCompleted => {
                    let order = Order::find(main_id, conn)?;
                    for organization in order.organizations(conn)? {
                        organization.log_interaction(
                            order.on_behalf_of_user_id.unwrap_or(order.user_id),
                            Utc::now().naive_utc(),
                            conn,
                        )?;
                    }
                }
                DomainEventTypes::TicketInstanceRedeemed => {
                    let ticket = TicketInstance::find(main_id, conn)?;
                    let wallet = Wallet::find(ticket.wallet_id, conn)?;
                    if let Some(user_id) = wallet.user_id {
                        ticket.organization(conn)?.log_interaction(
                            user_id,
                            Utc::now().naive_utc(),
                            conn,
                        )?;
                    }
                }
                DomainEventTypes::TransferTicketStarted
                | DomainEventTypes::TransferTicketCancelled
                | DomainEventTypes::TransferTicketCompleted => {
                    let transfer = Transfer::find(main_id, conn)?;

                    let mut temporary_user: Option<TemporaryUser> = None;
                    if !transfer.direct {
                        temporary_user =
                            TemporaryUser::find_or_build_from_transfer(&transfer, conn)?;
                    }

                    for organization in transfer.organizations(conn)? {
                        organization.log_interaction(
                            transfer.source_user_id,
                            Utc::now().naive_utc(),
                            conn,
                        )?;

                        if let Some(destination_user_id) = transfer.destination_user_id {
                            if let Some(temp_user) = temporary_user.clone() {
                                temp_user.associate_user(destination_user_id, conn)?;
                            }

                            organization.log_interaction(
                                destination_user_id,
                                Utc::now().naive_utc(),
                                conn,
                            )?;
                        }
                    }
                }
                _ => (),
            };
        }

        Ok(())
    }
}

#[derive(Insertable, Clone)]
#[table_name = "domain_events"]
pub struct NewDomainEvent {
    pub event_type: DomainEventTypes,
    pub display_text: String,
    pub event_data: Option<serde_json::Value>,
    pub main_table: Tables,
    pub main_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub created_at: Option<NaiveDateTime>,
}

impl NewDomainEvent {
    pub fn commit(self, conn: &PgConnection) -> Result<DomainEvent, DatabaseError> {
        let result: DomainEvent = diesel::insert_into(domain_events::table)
            .values(&self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not insert domain event")?;

        jlog!(Info, &format!("Domain Event: {} `{}` on {}:{}", self.event_type,
        self.display_text, self.main_table, self.main_id.map( |i| i.to_string()).unwrap_or_default())           ,{
            "domain_event_id": result.id,
            "event_type": self.event_type.clone(),
            "main_table": self.main_table.clone(),
            "main_id": self.main_id,
            "event_data": self.event_data
        });

        result.post_processing(conn)?;

        Ok(result)
    }
}
