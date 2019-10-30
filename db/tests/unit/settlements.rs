use bigneon_db::dev::TestProject;
use bigneon_db::models::*;
use bigneon_db::schema::orders;
use bigneon_db::utils::dates;
use bigneon_db::utils::errors::ErrorCode::ValidationError;
use chrono::prelude::*;
use chrono::Duration;
use diesel;
use diesel::prelude::*;
use diesel::sql_types;
use diesel::RunQueryDsl;

#[test]
fn find_last_settlement_for_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let organization2 = project.create_organization().finish();
    assert!(
        Settlement::find_last_settlement_for_organization(&organization, connection)
            .unwrap()
            .is_none()
    );
    assert!(
        Settlement::find_last_settlement_for_organization(&organization2, connection)
            .unwrap()
            .is_none()
    );

    let settlement = project
        .create_settlement()
        .with_organization(&organization)
        .finish();
    assert_eq!(
        Settlement::find_last_settlement_for_organization(&organization, connection).unwrap(),
        Some(settlement)
    );
    assert!(
        Settlement::find_last_settlement_for_organization(&organization2, connection)
            .unwrap()
            .is_none()
    );
}

#[test]
fn process_settlement_for_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();

    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(1)
        .with_organization(&organization)
        .finish();
    project.create_order().for_event(&event).is_paid().finish();

    let domain_events = DomainEvent::find(
        Tables::Organizations,
        Some(organization.id),
        Some(DomainEventTypes::SettlementReportProcessed),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    let settlement =
        Settlement::process_settlement_for_organization(&organization, None, connection).unwrap();
    let domain_events = DomainEvent::find(
        Tables::Organizations,
        Some(organization.id),
        Some(DomainEventTypes::SettlementReportProcessed),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
    assert_eq!(settlement.organization_id, organization.id);

    let end_time =
        organization.next_settlement_date(None).unwrap() - Duration::days(7) - Duration::seconds(1);
    assert_eq!(
        settlement.start_time,
        end_time - Duration::days(7) + Duration::seconds(1)
    );
    assert_eq!(settlement.end_time, end_time);

    // Set old end time which affects start time of new settlement (to handle timezone updates)
    let old_start_time = dates::now().add_days(-14).add_hours(-3).finish();
    let old_end_time = dates::now().add_days(-7).add_hours(-3).finish();
    diesel::sql_query(
        r#"
        UPDATE settlements
        SET start_time = $1,
        end_time = $2
        WHERE id = $3;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(old_start_time)
    .bind::<sql_types::Timestamp, _>(old_end_time)
    .bind::<sql_types::Uuid, _>(settlement.id)
    .execute(connection)
    .unwrap();

    let settlement =
        Settlement::process_settlement_for_organization(&organization, None, connection).unwrap();
    assert_eq!(
        settlement.start_time.timestamp(),
        (old_end_time + Duration::seconds(1)).timestamp()
    );
    let end_time =
        organization.next_settlement_date(None).unwrap() - Duration::days(7) - Duration::seconds(1);
    assert_eq!(settlement.end_time.timestamp(), end_time.timestamp());
}

#[test]
fn create_post_event_entries() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_cc_fee(5f32)
        .with_settlement_type(SettlementTypes::PostEvent)
        .finish();
    let organization2 = project
        .create_organization()
        .with_event_fee()
        .with_settlement_type(SettlementTypes::PostEvent)
        .finish();
    let past_event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(-15).finish())
        .with_event_end(dates::now().add_days(-6).finish())
        .finish();
    let ticket_type = &past_event.ticket_types(true, None, connection).unwrap()[0];
    let past_event_2 = project
        .create_event()
        .with_organization(&organization2)
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(-15).finish())
        .with_event_end(dates::now().add_days(-6).finish())
        .finish();
    let ending_future_event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(-15).finish())
        .with_event_end(dates::now().add_days(1).finish())
        .finish();

    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_discount_in_cents(10)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let hold2 = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_discount_in_cents(20)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let hold3 = project
        .create_hold()
        .with_hold_type(HoldTypes::Comp)
        .with_quantity(10)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let code = project
        .create_code()
        .with_event(&past_event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_type)
        .with_discount_in_cents(Some(10))
        .finish();
    project
        .create_order()
        .for_event(&past_event)
        .quantity(3)
        .is_paid()
        .finish();
    project
        .create_order()
        .for_event(&past_event_2)
        .is_paid()
        .finish();
    project
        .create_order()
        .for_event(&ending_future_event)
        .is_paid()
        .finish();

    // Hold order, 10 discount
    project
        .create_order()
        .for_event(&past_event)
        .quantity(2)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .is_paid()
        .finish();

    // Hold order, 20 discount
    project
        .create_order()
        .for_event(&past_event)
        .quantity(2)
        .with_redemption_code(hold2.redemption_code.clone().unwrap())
        .is_paid()
        .finish();

    // Hold order, comp
    project
        .create_order()
        .for_event(&past_event)
        .quantity(2)
        .with_redemption_code(hold3.redemption_code.clone().unwrap())
        .is_paid()
        .finish();

    // Code order, 10 discount
    project
        .create_order()
        .for_event(&past_event)
        .quantity(3)
        .with_redemption_code(code.redemption_code.clone())
        .is_paid()
        .finish();

    // Transaction from before settlement period
    let mut order = project
        .create_order()
        .for_event(&past_event)
        .quantity(5)
        .is_paid()
        .finish();
    diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set((orders::paid_at.eq(dates::now().add_days(-8).finish()),))
        .execute(connection)
        .unwrap();

    // Refund one ticket from order
    let items = order.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    order
        .refund(&refund_items, user.id, None, connection)
        .unwrap();

    let settlement = Settlement::create(
        organization.id,
        dates::now().add_days(-14).finish(),
        dates::now().add_days(-7).finish(),
        SettlementStatus::PendingSettlement,
        Some("test comment".to_string()),
        true,
    )
    .commit(None, connection)
    .unwrap();

    let display_settlement = settlement.clone().for_display(connection).unwrap();
    assert!(display_settlement.event_entries.is_empty());

    let settlement = Settlement::create(
        organization.id,
        dates::now().add_days(-7).finish(),
        dates::now().finish(),
        SettlementStatus::PendingSettlement,
        Some("test comment".to_string()),
        true,
    )
    .commit(None, connection)
    .unwrap();

    let display_settlement = settlement.clone().for_display(connection).unwrap();
    assert_eq!(display_settlement.event_entries.len(), 1);
    assert_eq!(
        display_settlement.event_entries[0].event,
        past_event.for_display(connection).unwrap()
    );
    let event_entries = &display_settlement.event_entries[0].entries;
    assert_eq!(event_entries.len(), 4);

    // No code entry
    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType
                && e.face_value_in_cents == 150
        })
        .unwrap();
    let ticket_type = &past_event.ticket_types(true, None, connection).unwrap()[0];
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 150);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 7);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 7);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    // Code and redemption code orders have same price due to identical discount so they are grouped
    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType
                && e.face_value_in_cents == 140
        })
        .unwrap();
    let ticket_type = &past_event.ticket_types(true, None, connection).unwrap()[0];
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 140);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 5);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 5);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    // Redemption code with different discount
    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType
                && e.face_value_in_cents == 130
        })
        .unwrap();
    let ticket_type = &past_event.ticket_types(true, None, connection).unwrap()[0];
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 130);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 2);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 2);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    let event_fee_entry = event_entries
        .into_iter()
        .find(|e| e.settlement_entry_type == SettlementEntryTypes::EventFees)
        .unwrap();
    assert_eq!(event_fee_entry.settlement_id, settlement.id);
    assert_eq!(event_fee_entry.event_id, past_event.id);
    assert_eq!(event_fee_entry.ticket_type_id, None);
    assert_eq!(event_fee_entry.face_value_in_cents, 0);
    assert_eq!(event_fee_entry.revenue_share_value_in_cents, 150);
    assert_eq!(event_fee_entry.online_sold_quantity, 0);
    assert_eq!(event_fee_entry.fee_sold_quantity, 5);
    assert_eq!(event_fee_entry.total_sales_in_cents, 750);
}

#[test]
fn create_rolling_entries() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_cc_fee(5f32)
        .with_settlement_type(SettlementTypes::Rolling)
        .finish();
    let organization2 = project
        .create_organization()
        .with_event_fee()
        .with_settlement_type(SettlementTypes::Rolling)
        .finish();
    let past_event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(-15).finish())
        .with_event_end(dates::now().add_days(-14).finish())
        .finish();
    let ticket_type = &past_event.ticket_types(true, None, connection).unwrap()[0];
    let past_event_2 = project
        .create_event()
        .with_organization(&organization2)
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(-15).finish())
        .with_event_end(dates::now().add_days(-14).finish())
        .finish();
    let ending_future_event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .with_event_start(dates::now().add_days(-15).finish())
        .with_event_end(dates::now().add_days(1).finish())
        .finish();
    let ticket_type2 = &ending_future_event
        .ticket_types(true, None, connection)
        .unwrap()[0];

    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_discount_in_cents(10)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let hold2 = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_discount_in_cents(20)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let hold3 = project
        .create_hold()
        .with_hold_type(HoldTypes::Comp)
        .with_quantity(10)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let code = project
        .create_code()
        .with_event(&past_event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_type)
        .with_discount_in_cents(Some(10))
        .finish();

    // Transaction from before settlement period
    let order = project
        .create_order()
        .for_event(&past_event)
        .is_paid()
        .finish();
    diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set((orders::paid_at.eq(dates::now().add_days(-8).finish()),))
        .execute(connection)
        .unwrap();

    // Transaction from after settlement period
    let order = project
        .create_order()
        .for_event(&past_event)
        .is_paid()
        .finish();
    diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set((orders::paid_at.eq(dates::now().add_minutes(1).finish()),))
        .execute(connection)
        .unwrap();

    let mut order = project
        .create_order()
        .quantity(4)
        .for_event(&past_event)
        .is_paid()
        .finish();
    // Refund one ticket from order
    let items = order.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    order
        .refund(&refund_items, user.id, None, connection)
        .unwrap();

    project
        .create_order()
        .for_event(&past_event_2)
        .is_paid()
        .finish();
    project
        .create_order()
        .quantity(5)
        .for_event(&ending_future_event)
        .is_paid()
        .finish();

    // Hold order, 10 discount
    project
        .create_order()
        .for_event(&past_event)
        .quantity(2)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .is_paid()
        .finish();

    // Hold order, 20 discount
    project
        .create_order()
        .for_event(&past_event)
        .quantity(2)
        .with_redemption_code(hold2.redemption_code.clone().unwrap())
        .is_paid()
        .finish();

    // Hold order, comp
    project
        .create_order()
        .for_event(&past_event)
        .quantity(2)
        .with_redemption_code(hold3.redemption_code.clone().unwrap())
        .is_paid()
        .finish();

    // Code order, 10 discount
    project
        .create_order()
        .for_event(&past_event)
        .quantity(3)
        .with_redemption_code(code.redemption_code.clone())
        .is_paid()
        .finish();

    let settlement = Settlement::create(
        organization.id,
        dates::now().add_days(-7).finish(),
        dates::now().finish(),
        SettlementStatus::PendingSettlement,
        Some("test comment".to_string()),
        false,
    )
    .commit(None, connection)
    .unwrap();

    let display_settlement = settlement.clone().for_display(connection).unwrap();
    assert_eq!(display_settlement.event_entries.len(), 2);
    let event_entries_data = display_settlement
        .event_entries
        .iter()
        .find(|event_entry| event_entry.event == past_event.for_display(connection).unwrap())
        .unwrap();
    let event_entries = &event_entries_data.entries;
    assert_eq!(event_entries.len(), 4);

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType
                && e.ticket_type_id == Some(ticket_type.id)
                && e.face_value_in_cents == 150
        })
        .unwrap();
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 150);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 3);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 3);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType
                && e.ticket_type_id == Some(ticket_type.id)
                && e.face_value_in_cents == 140
        })
        .unwrap();
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 140);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 5);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 5);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType
                && e.ticket_type_id == Some(ticket_type.id)
                && e.face_value_in_cents == 130
        })
        .unwrap();
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, past_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 130);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 2);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 2);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    let event_fee_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::EventFees
                && e.event_id == past_event.id
        })
        .unwrap();
    assert_eq!(event_fee_entry.settlement_id, settlement.id);
    assert_eq!(event_fee_entry.event_id, past_event.id);
    assert_eq!(event_fee_entry.ticket_type_id, None);
    assert_eq!(event_fee_entry.face_value_in_cents, 0);
    assert_eq!(event_fee_entry.revenue_share_value_in_cents, 150);
    assert_eq!(event_fee_entry.online_sold_quantity, 0);
    assert_eq!(event_fee_entry.fee_sold_quantity, 4);
    assert_eq!(event_fee_entry.total_sales_in_cents, 600);

    let event_entries_data = display_settlement
        .event_entries
        .iter()
        .find(|event_entry| {
            event_entry.event == ending_future_event.for_display(connection).unwrap()
        })
        .unwrap();
    let event_entries = &event_entries_data.entries;
    assert_eq!(event_entries.len(), 2);

    let ticket_type_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::TicketType
                && e.ticket_type_id == Some(ticket_type2.id)
        })
        .unwrap();
    assert_eq!(ticket_type_entry.settlement_id, settlement.id);
    assert_eq!(ticket_type_entry.event_id, ending_future_event.id);
    assert_eq!(ticket_type_entry.ticket_type_id, Some(ticket_type2.id));
    assert_eq!(ticket_type_entry.face_value_in_cents, 150);
    assert_eq!(ticket_type_entry.revenue_share_value_in_cents, 30);
    assert_eq!(ticket_type_entry.online_sold_quantity, 5);
    assert_eq!(ticket_type_entry.fee_sold_quantity, 5);
    assert_eq!(
        ticket_type_entry.total_sales_in_cents,
        ticket_type_entry.online_sold_quantity * ticket_type_entry.face_value_in_cents
            + ticket_type_entry.fee_sold_quantity * ticket_type_entry.revenue_share_value_in_cents
    );

    let event_fee_entry = event_entries
        .clone()
        .into_iter()
        .find(|e| {
            e.settlement_entry_type == SettlementEntryTypes::EventFees
                && e.event_id == ending_future_event.id
        })
        .unwrap();
    assert_eq!(event_fee_entry.settlement_id, settlement.id);
    assert_eq!(event_fee_entry.event_id, ending_future_event.id);
    assert_eq!(event_fee_entry.ticket_type_id, None);
    assert_eq!(event_fee_entry.face_value_in_cents, 0);
    assert_eq!(event_fee_entry.revenue_share_value_in_cents, 150);
    assert_eq!(event_fee_entry.online_sold_quantity, 0);
    assert_eq!(event_fee_entry.fee_sold_quantity, 1);
    assert_eq!(event_fee_entry.total_sales_in_cents, 150);
}

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project.create_organization().finish();
    project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .finish();
    let domain_events = DomainEvent::find(
        Tables::Organizations,
        Some(organization.id),
        Some(DomainEventTypes::SettlementReportProcessed),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());
    let settlement = Settlement::create(
        organization.id,
        NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11),
        NaiveDate::from_ymd(2020, 7, 8).and_hms(4, 10, 11),
        SettlementStatus::PendingSettlement,
        Some("test comment".to_string()),
        true,
    )
    .commit(Some(user.clone()), connection)
    .unwrap();

    assert_eq!(settlement.organization_id, organization.id);
    assert_eq!(settlement.comment, Some("test comment".to_string()));

    let domain_events = DomainEvent::find(
        Tables::Organizations,
        Some(organization.id),
        Some(DomainEventTypes::SettlementReportProcessed),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
    assert_eq!(domain_events[0].user_id, Some(user.id));
}

#[test]
fn create_with_validation_errors() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .finish();
    let result = Settlement::create(
        organization.id,
        NaiveDate::from_ymd(2020, 7, 8).and_hms(4, 10, 11),
        NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11),
        SettlementStatus::PendingSettlement,
        Some("test comment".to_string()),
        true,
    )
    .commit(None, connection);

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("start_time"));
                assert_eq!(errors["start_time"].len(), 1);
                assert_eq!(errors["start_time"][0].code, "end_time_before_start_time");
                assert_eq!(
                    &errors["start_time"][0]
                        .message
                        .clone()
                        .unwrap()
                        .into_owned(),
                    "End time must be after start time"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn adjustments() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let settlement = project.create_settlement().finish();
    assert!(settlement.adjustments(connection).unwrap().is_empty());

    let settlement_adjustment = project
        .create_settlement_adjustment()
        .with_settlement(&settlement)
        .finish();
    assert_eq!(
        settlement.adjustments(connection).unwrap(),
        vec![settlement_adjustment]
    );
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let settlement = project.create_settlement().finish();
    settlement.clone().destroy(connection).unwrap();
    assert!(Settlement::find(settlement.id, connection).is_err());
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let _event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let settlement = project
        .create_settlement()
        .with_organization(&organization)
        .finish();
    let read_settlement = Settlement::find(settlement.id, connection).unwrap();
    assert_eq!(settlement.id, read_settlement.id);
    assert_eq!(settlement.comment, read_settlement.comment);
    assert_eq!(settlement.start_time, read_settlement.start_time);
}

#[test]
fn for_display() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .with_ticket_pricing()
        .finish();
    let settlement = project
        .create_settlement()
        .with_organization(&organization)
        .finish();
    let entry = project
        .create_settlement_entry()
        .with_settlement(&settlement)
        .with_event(&event)
        .finish();
    let adjustment = project
        .create_settlement_adjustment()
        .with_settlement(&settlement)
        .finish();
    let display_settlement = settlement.clone().for_display(connection).unwrap();
    assert_eq!(display_settlement.settlement.id, settlement.id);
    let display_event = event.for_display(connection).unwrap();
    assert_eq!(display_settlement.event_entries.len(), 1);
    assert_eq!(display_settlement.event_entries[0].event, display_event);
    assert_eq!(display_settlement.event_entries[0].entries[0].id, entry.id);
    assert_eq!(display_settlement.adjustments[0], adjustment);
}

#[test]
fn find_for_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let _event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let _event2 = project
        .create_event()
        .with_name("NewEvent2".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let settlement = project
        .create_settlement()
        .with_organization(&organization)
        .finish();
    let settlements = Settlement::find_for_organization(organization.id, None, None, connection)
        .unwrap()
        .data;
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements[0].id, settlement.id);
}
