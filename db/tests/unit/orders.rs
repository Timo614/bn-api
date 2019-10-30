use bigneon_db::dev::times;
use bigneon_db::dev::TestProject;
use bigneon_db::models::*;
use bigneon_db::schema::{order_items, orders, ticket_instances};
use bigneon_db::utils::errors::DatabaseError;
use bigneon_db::utils::errors::ErrorCode::ValidationError;
use chrono::prelude::*;
use diesel;
use diesel::prelude::*;
use std::collections::HashMap;
use time::Duration;
use uuid::Uuid;

#[test]
fn transfers() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let order = project
        .create_order()
        .for_user(&user)
        .quantity(1)
        .is_paid()
        .finish();
    let ticket = &TicketInstance::find_for_user(user.id, connection).unwrap()[0];

    let transfer = Transfer::create(user.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    transfer.add_transfer_ticket(ticket.id, connection).unwrap();
    assert!(transfer.update_associated_orders(connection).is_ok());
    assert_eq!(vec![transfer], order.transfers(connection).unwrap());
}

#[test]
fn create_note() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let order = project.create_order().is_paid().finish();
    let note_text = "Note goes here".to_string();
    let note = order
        .create_note(note_text.clone(), user.id, connection)
        .unwrap();
    assert_eq!(note.note, note_text);
    assert_eq!(note.created_by, user.id);
    assert_eq!(note.main_id, order.id);
    assert_eq!(note.main_table, Tables::Orders);
}

#[test]
fn activity() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_fees()
        .finish();
    let organization2 = project
        .create_organization()
        .with_event_fee()
        .with_fees()
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(1)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_organization(&organization2)
        .with_ticket_pricing()
        .finish();
    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_ticket_type_id(event.ticket_types(true, None, connection).unwrap()[0].id)
        .finish();
    let code = project
        .create_code()
        .with_event(&event2)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&event2.ticket_types(true, None, connection).unwrap()[0])
        .with_discount_in_cents(Some(10))
        .finish();
    let order = project
        .create_order()
        .for_event(&event)
        .on_behalf_of_user(&user)
        .for_user(&user2)
        .quantity(2)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .is_paid()
        .finish();
    let order2 = project
        .create_order()
        .for_event(&event2)
        .for_user(&user)
        .quantity(3)
        .is_paid()
        .finish();
    let order3 = project
        .create_order()
        .for_event(&event2)
        .for_user(&user)
        .quantity(3)
        .with_redemption_code(code.redemption_code.clone())
        .is_paid()
        .finish();

    assert_eq!(
        ActivityItem::load_for_order(&order, connection).unwrap(),
        order.activity(connection).unwrap().data
    );
    assert_eq!(
        ActivityItem::load_for_order(&order2, connection).unwrap(),
        order2.activity(connection).unwrap().data
    );
    assert_eq!(
        ActivityItem::load_for_order(&order3, connection).unwrap(),
        order3.activity(connection).unwrap().data
    );
}

#[test]
fn main_event_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    assert!(cart
        .update_quantities(
            user.id,
            &vec![UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 1,
                redemption_code: None,
            }],
            false,
            true,
            connection,
        )
        .is_ok());

    assert_eq!(event.id, cart.main_event_id(connection).unwrap());
}

/// This test does the following:
/// - Create an event with 100 tickets
/// - Test case 1:
///   - Creates a cart for 99 tickets
///   - Updates the cart to cancelled
///   - Tries to refresh the cart and expects a failure
/// - Test case 2:
///   - Updates the cart to draft with an expiry date in the past
///   - Ticket instances are also updated with a reserved_until in the past
///   - Refreshes the cart. Expect cart to be re-instated
/// - Test case 3:
///   - Expires the order and tickets
///   - Buys 1 ticket with a second order
///   - Pays for order 2
///   - Refreshes the cart. Expect success
/// - Test case 4:
///   - Expires the order and tickets
///   - Buys 1 ticket with a third order
///   - Pays for order 3
///   - Refreshes the cart, fails because there are not enough free tickets
#[test]
fn try_refresh_expired_cart() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    // Order must be expired
    let mut order = Order::find_or_create_cart(&user, connection).unwrap();
    assert_eq!(
        order.try_refresh_expired_cart(Some(user.id), connection),
        DatabaseError::business_process_error("Cart is not expired",)
    );

    // Order must be in draft or pending payment status
    order
        .update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 99,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();
    let ticket_ids: Vec<Uuid> = order
        .tickets(ticket_type.id, connection)
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect();
    diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set((orders::status.eq(OrderStatus::Cancelled),))
        .execute(connection)
        .unwrap();
    let mut order = Order::find(order.id, connection).unwrap();
    assert_eq!(
        order.try_refresh_expired_cart(Some(user.id), connection),
        DatabaseError::business_process_error(
            "Can't refresh expired cart unless the order is in draft or pending payment statuses",
        )
    );

    // Past expiration should succeed to refresh if tickets available
    let past_expiry = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(5));
    diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set((
            orders::expires_at.eq(past_expiry),
            orders::status.eq(OrderStatus::Draft),
        ))
        .execute(connection)
        .unwrap();
    diesel::update(ticket_instances::table.filter(ticket_instances::id.eq_any(ticket_ids.clone())))
        .set(ticket_instances::reserved_until.eq(past_expiry))
        .execute(connection)
        .unwrap();
    let mut order = Order::find(order.id, connection).unwrap();
    assert_eq!(
        order.expires_at.map(|e| e.timestamp()),
        Some(past_expiry.timestamp())
    );
    order
        .try_refresh_expired_cart(Some(user.id), connection)
        .unwrap();

    let default_expiry =
        NaiveDateTime::from(Utc::now().naive_utc() + Duration::minutes(CART_EXPIRY_TIME_MINUTES));
    let order = Order::find(order.id, connection).unwrap();
    assert!((default_expiry.timestamp() - order.expires_at.unwrap().timestamp()).abs() < 2);
    for ticket in order.tickets(ticket_type.id, connection).unwrap() {
        assert!(
            (default_expiry.timestamp() - ticket.reserved_until.unwrap().timestamp()).abs() < 2
        );
    }

    // Purchase 1 of the 100 remaining tickets does not cause an issue for refresh as 99 are available
    // Invalidate existing items to allow for purchase by other user
    diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set((orders::expires_at.eq(past_expiry),))
        .execute(connection)
        .unwrap();
    diesel::update(ticket_instances::table.filter(ticket_instances::id.eq_any(ticket_ids.clone())))
        .set(ticket_instances::reserved_until.eq(past_expiry))
        .execute(connection)
        .unwrap();
    let mut order = Order::find(order.id, connection).unwrap();
    let mut order2 = Order::find_or_create_cart(&user2, connection).unwrap();
    order2
        .update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 1,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();
    let total = order2.calculate_total(connection).unwrap();
    order2
        .add_external_payment(
            Some("Test".to_string()),
            ExternalPaymentType::CreditCard,
            user2.id,
            total,
            connection,
        )
        .unwrap();

    order
        .try_refresh_expired_cart(Some(user.id), connection)
        .unwrap();
    let default_expiry =
        NaiveDateTime::from(Utc::now().naive_utc() + Duration::minutes(CART_EXPIRY_TIME_MINUTES));
    let order = Order::find(order.id, connection).unwrap();
    assert!((default_expiry.timestamp() - order.expires_at.unwrap().timestamp()).abs() < 2);
    for ticket in order.tickets(ticket_type.id, connection).unwrap() {
        assert!(
            (default_expiry.timestamp() - ticket.reserved_until.unwrap().timestamp()).abs() < 2
        );
    }

    // Purchase 1 of the 99 remaining tickets leading to failure when original order refreshes
    // Invalidate existing items to allow for purchase by other user
    diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set((orders::expires_at.eq(past_expiry),))
        .execute(connection)
        .unwrap();
    diesel::update(ticket_instances::table.filter(ticket_instances::id.eq_any(ticket_ids)))
        .set(ticket_instances::reserved_until.eq(past_expiry))
        .execute(connection)
        .unwrap();
    let mut order = Order::find(order.id, connection).unwrap();
    let mut order2 = Order::find_or_create_cart(&user2, connection).unwrap();
    order2
        .update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 1,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();
    let total = order2.calculate_total(connection).unwrap();
    order2
        .add_external_payment(
            Some("Test".to_string()),
            ExternalPaymentType::CreditCard,
            user2.id,
            total,
            connection,
        )
        .unwrap();

    // Order fails given lack of available tickets
    let result = order.try_refresh_expired_cart(Some(user.id), connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("quantity"));
                assert_eq!(errors["quantity"].len(), 1);
                assert_eq!(
                    errors["quantity"][0].code,
                    "Could not reserve tickets, not enough tickets are available"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn user() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let order = Order::find_or_create_cart(&user, connection).unwrap();
    let user = User::find(user.id, connection).unwrap();
    let user2 = User::find(user.id, connection).unwrap();

    let order2 = project
        .create_order()
        .for_user(&user2)
        .on_behalf_of_user(&user)
        .finish();

    assert_eq!(order.user(connection), Ok(user));
    assert_eq!(order2.user(connection), Ok(user2));
}

#[test]
fn is_expired() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let order = Order::find_or_create_cart(&user, connection).unwrap();

    // No expiration date set
    assert_eq!(order.expires_at, None);
    assert!(!order.is_expired());

    // Past expiration date
    let past_expiry = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(5));
    diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set((orders::expires_at.eq(past_expiry),))
        .execute(connection)
        .unwrap();
    let order = Order::find(order.id, connection).unwrap();
    assert!(order.is_expired());

    // Future expiration date
    let future_expiry = NaiveDateTime::from(Utc::now().naive_utc() + Duration::minutes(5));
    diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set((orders::expires_at.eq(future_expiry),))
        .execute(connection)
        .unwrap();
    let order = Order::find(order.id, connection).unwrap();
    assert!(!order.is_expired());
}

#[test]
fn set_user_agent() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let user_agent = Some("Fake User Agent 1".to_string());
    let user_agent2 = Some("Fake User Agent 2".to_string());
    let user_agent3 = Some("Fake User Agent 3".to_string());
    assert!(cart.create_user_agent.is_none());
    assert!(cart.purchase_user_agent.is_none());

    cart.set_user_agent(user_agent.clone(), false, connection)
        .unwrap();
    assert_eq!(user_agent.clone(), cart.create_user_agent);
    assert!(cart.purchase_user_agent.is_none());

    cart.set_user_agent(user_agent2.clone(), false, connection)
        .unwrap();
    assert_eq!(user_agent2.clone(), cart.create_user_agent);
    assert!(cart.purchase_user_agent.is_none());

    cart.set_user_agent(user_agent3.clone(), true, connection)
        .unwrap();
    assert_eq!(user_agent2, cart.create_user_agent);
    assert_eq!(user_agent3, cart.purchase_user_agent);
}

#[test]
fn find_or_create_cart() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    // No cart yet
    let cart = Order::find_or_create_cart(&user, connection).unwrap();
    let cart_id = cart.id;
    assert_eq!(user.id, cart.user_id);
    assert_eq!(OrderStatus::Draft, cart.status);

    // Cart now exists
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    assert_eq!(cart_id, cart.id);
    assert_eq!(user.id, cart.user_id);
    assert_eq!(OrderStatus::Draft, cart.status);

    // Purchase existing cart creates new cart
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();
    assert_eq!(OrderStatus::Paid, cart.status);

    let cart = Order::find_or_create_cart(&user, connection).unwrap();
    assert_ne!(cart_id, cart.id);
    assert_eq!(user.id, cart.user_id);
    assert_eq!(OrderStatus::Draft, cart.status);
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let cart = Order::find_or_create_cart(&user, connection).unwrap();

    let found_cart = Order::find(cart.id, connection).unwrap();
    assert_eq!(found_cart, cart);
}

#[test]
fn set_remove_expiry() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let domain_events = DomainEvent::find(
        Tables::Orders,
        Some(cart.id),
        Some(DomainEventTypes::OrderUpdated),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    // set_expiry, with provided expiration date
    let expiration = NaiveDate::from_ymd(2055, 7, 8).and_hms(7, 8, 10);
    cart.set_expiry(Some(user.id), Some(expiration.clone()), false, connection)
        .unwrap();
    let domain_events = DomainEvent::find(
        Tables::Orders,
        Some(cart.id),
        Some(DomainEventTypes::OrderUpdated),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
    assert_eq!(
        cart.expires_at.map(|e| e.timestamp()),
        Some(expiration.timestamp())
    );

    // set_expiry, Default expiration
    cart.set_expiry(Some(user.id), None, false, connection)
        .unwrap();
    let domain_events = DomainEvent::find(
        Tables::Orders,
        Some(cart.id),
        Some(DomainEventTypes::OrderUpdated),
        connection,
    )
    .unwrap();
    assert_eq!(2, domain_events.len());
    let default_expiry =
        NaiveDateTime::from(Utc::now().naive_utc() + Duration::minutes(CART_EXPIRY_TIME_MINUTES));
    assert!((default_expiry.timestamp() - cart.expires_at.unwrap().timestamp()).abs() < 2);

    // remove_expiry
    cart.remove_expiry(user.id, connection).unwrap();
    assert!(cart.expires_at.is_none());
    let domain_events = DomainEvent::find(
        Tables::Orders,
        Some(cart.id),
        Some(DomainEventTypes::OrderUpdated),
        connection,
    )
    .unwrap();
    assert_eq!(3, domain_events.len());

    // Set past expiry
    let past_expiry = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(5));
    cart.set_expiry(Some(user.id), Some(past_expiry), false, connection)
        .unwrap();
    assert_eq!(
        cart.expires_at.map(|e| e.timestamp()),
        Some(past_expiry.timestamp())
    );

    // Cart fails to set expiry due to past date currently set
    assert!(cart
        .set_expiry(Some(user.id), None, false, connection)
        .is_err());

    // Cart succeeds forcing expiry
    assert!(cart
        .set_expiry(Some(user.id), None, true, connection)
        .is_ok());
    assert!((default_expiry.timestamp() - cart.expires_at.unwrap().timestamp()).abs() < 2);
}

#[test]
fn order_number() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();

    cart.id = Uuid::parse_str("01234567-1234-1234-1234-1234567890ab").unwrap();
    assert_eq!("567890ab".to_string(), cart.order_number());
}

#[test]
fn update_fees() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_fees()
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = event
        .ticket_types(true, None, connection)
        .unwrap()
        .remove(0);
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    // Remove all fees
    let items = cart.items(connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    diesel::delete(
        order_items::table.filter(
            order_items::parent_id
                .eq(order_item.id)
                .or(order_items::item_type.eq(OrderItemTypes::EventFees)),
        ),
    )
    .execute(connection)
    .unwrap();
    let items = cart.items(connection).unwrap();
    assert!(items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees)
        .is_none());
    assert!(order_item.find_fee_item(connection).unwrap().is_none());

    // Trigger fee
    cart.update_fees_and_discounts(connection).unwrap();
    let items = cart.items(connection).unwrap();
    let event_fee_item = items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees);
    assert!(event_fee_item.is_some());

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    assert_eq!(fee_item.parent_id, Some(order_item.id));
    assert_eq!(fee_item.item_type, OrderItemTypes::PerUnitFees);

    // Using a Comp, no fees
    let comp = project
        .create_hold()
        .with_hold_type(HoldTypes::Comp)
        .finish();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: comp.ticket_type_id,
            quantity: 2,
            redemption_code: comp.redemption_code,
        }],
        false,
        true,
        connection,
    )
    .unwrap();

    cart.update_fees_and_discounts(connection).unwrap();
}

#[test]
fn refund_can_refund_previously_refunded_and_repurchased_tickets() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_a_specific_number_of_tickets(1)
        .with_ticket_pricing()
        .finish();
    let ticket_types = &event.ticket_types(true, None, connection).unwrap();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let user3 = project.create_user().finish();

    for user in vec![user.clone(), user2, user3, user] {
        let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
        cart.update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_types[0].id,
                quantity: 1,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();
        let total = cart.calculate_total(connection).unwrap();
        cart.add_external_payment(
            Some("Test".to_string()),
            ExternalPaymentType::CreditCard,
            user.id,
            total,
            connection,
        )
        .unwrap();
        let items = cart.items(&connection).unwrap();
        let order_item = items
            .iter()
            .find(|i| i.ticket_type_id == Some(ticket_types[0].id))
            .unwrap();
        let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
        let refund_items = vec![RefundItemRequest {
            order_item_id: order_item.id,
            ticket_instance_id: Some(tickets[0].id),
        }];

        assert!(cart
            .refund(&refund_items, user.id, None, connection)
            .is_ok());
        let ticket = TicketInstance::find(tickets[0].id, connection).unwrap();
        assert!(ticket.order_item_id.is_none());
        let order_item = OrderItem::find_in_order(cart.id, order_item.id, connection).unwrap();
        assert_eq!(order_item.refunded_quantity, 1);
    }
}

#[test]
fn quantity_for_user_for_event() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_types = &event.ticket_types(true, None, connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let expected = HashMap::new();
    let ticket_type_quantities =
        Order::quantity_for_user_for_event(user.id, event.id, connection).unwrap();
    assert_eq!(expected, ticket_type_quantities);

    // Two in cart
    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 2,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 1,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();
    let ticket_type_quantities =
        Order::quantity_for_user_for_event(user.id, event.id, connection).unwrap();
    let mut expected = HashMap::new();
    expected.insert(ticket_type.id, 2);
    expected.insert(ticket_type2.id, 1);
    assert_eq!(expected, ticket_type_quantities);

    // Two purchased
    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();

    let ticket_type_quantities =
        Order::quantity_for_user_for_event(user.id, event.id, connection).unwrap();
    let mut expected = HashMap::new();
    expected.insert(ticket_type.id, 2);
    expected.insert(ticket_type2.id, 1);
    assert_eq!(expected, ticket_type_quantities);

    // Two in cart, two purchased
    let mut last_order = cart;
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let ticket_type_quantities =
        Order::quantity_for_user_for_event(user.id, event.id, connection).unwrap();
    let mut expected = HashMap::new();
    expected.insert(ticket_type.id, 4);
    expected.insert(ticket_type2.id, 1);
    assert_eq!(expected, ticket_type_quantities);

    // Two in cart, two purchased, one refunded
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    last_order
        .refund(&refund_items, user.id, None, connection)
        .unwrap();
    let ticket_type_quantities =
        Order::quantity_for_user_for_event(user.id, event.id, connection).unwrap();
    let mut expected = HashMap::new();
    expected.insert(ticket_type.id, 3);
    expected.insert(ticket_type2.id, 1);
    assert_eq!(expected, ticket_type_quantities);

    // Box office user purchasing an additional two tickets for this user
    let user2 = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user2)
        .on_behalf_of_user(&user)
        .quantity(2)
        .is_paid()
        .finish();
    let ticket_type_quantities =
        Order::quantity_for_user_for_event(user.id, event.id, connection).unwrap();
    let mut expected = HashMap::new();
    expected.insert(ticket_type.id, 5);
    expected.insert(ticket_type2.id, 1);
    assert_eq!(expected, ticket_type_quantities);

    let ticket_type_quantities =
        Order::quantity_for_user_for_event(user2.id, event.id, connection).unwrap();
    let expected = HashMap::new();
    assert_eq!(expected, ticket_type_quantities);
}

#[test]
fn items() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_fees()
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_types = &event.ticket_types(true, None, connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 2,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 1,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();

    let items = cart.items(connection).unwrap();
    assert_eq!(5, items.len());
    assert_eq!(
        1,
        items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::EventFees)
            .count()
    );
    assert_eq!(
        2,
        items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::PerUnitFees)
            .count()
    );
    assert_eq!(
        2,
        items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets)
            .count()
    );
}

#[test]
fn tickets() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_types = &event.ticket_types(true, None, connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let expected = HashMap::new();
    let ticket_type_quantities =
        Order::quantity_for_user_for_event(user.id, event.id, connection).unwrap();
    assert_eq!(expected, ticket_type_quantities);

    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 2,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 1,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();
    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();

    let items: Vec<OrderItem> = cart
        .items(connection)
        .unwrap()
        .into_iter()
        .filter(|t| t.item_type == OrderItemTypes::Tickets)
        .collect();
    let order_item = &items[0];
    let order_item2 = &items[1];

    let tickets = cart.tickets(ticket_type.id, connection).unwrap();
    assert_eq!(2, tickets.len());
    assert_eq!(
        TicketInstance::find_for_order_item(order_item.id, connection).unwrap(),
        tickets
    );

    let tickets = cart.tickets(ticket_type2.id, connection).unwrap();
    assert_eq!(1, tickets.len());
    assert_eq!(
        TicketInstance::find_for_order_item(order_item2.id, connection).unwrap(),
        tickets
    );
}

#[test]
fn items_for_display() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_types = &event.ticket_types(true, None, connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 2,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 1,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let display_items = Order::items_for_display(vec![cart.id], None, user.id, connection)
        .unwrap()
        .remove(&cart.id)
        .unwrap();
    assert_eq!(4, display_items.len());
    assert!(display_items.iter().find(|i| i.id == items[0].id).is_some());
    assert!(display_items.iter().find(|i| i.id == items[1].id).is_some());
    assert!(display_items.iter().find(|i| i.id == items[2].id).is_some());
    assert!(display_items.iter().find(|i| i.id == items[3].id).is_some());
}

#[test]
fn find_item_by_type() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_types = &event.ticket_types(true, None, connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 2,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 1,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();
    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();

    let result = cart
        .find_item_by_type(ticket_type.id, OrderItemTypes::Tickets, connection)
        .unwrap();
    assert_eq!(result.item_type, OrderItemTypes::Tickets);
    assert_eq!(result.ticket_type_id, Some(ticket_type.id));

    let result = cart
        .find_item_by_type(ticket_type2.id, OrderItemTypes::Tickets, connection)
        .unwrap();
    assert_eq!(result.item_type, OrderItemTypes::Tickets);
    assert_eq!(result.ticket_type_id, Some(ticket_type2.id));
}

#[test]
fn add_provider_payment() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_types = &event.ticket_types(true, None, connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 2,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 1,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();

    let external_reference = "External Reference".to_string();
    cart.add_provider_payment(
        Some(external_reference.clone()),
        PaymentProviders::Stripe,
        Some(user.id),
        500,
        PaymentStatus::Completed,
        None,
        serde_json::Value::Null,
        connection,
    )
    .unwrap();
    assert_eq!(OrderStatus::Draft, cart.status);
    let payment = &cart.payments(connection).unwrap()[0];
    assert_eq!(Some(external_reference.clone()), payment.external_reference);
    assert_eq!(PaymentProviders::Stripe, payment.provider);
    assert_eq!(PaymentStatus::Completed, payment.status);

    let remaining = cart.calculate_total(connection).unwrap() - 500;
    cart.add_provider_payment(
        Some(external_reference.clone()),
        PaymentProviders::Stripe,
        Some(user.id),
        remaining,
        PaymentStatus::Completed,
        None,
        serde_json::Value::Null,
        connection,
    )
    .unwrap();
    assert_eq!(OrderStatus::Paid, cart.status);
    let payment = &cart.payments(connection).unwrap()[1];
    assert_eq!(Some(external_reference.clone()), payment.external_reference);
    assert_eq!(PaymentProviders::Stripe, payment.provider);
    assert_eq!(PaymentStatus::Completed, payment.status);
}

#[test]
fn add_credit_card_payment() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_types = &event.ticket_types(true, None, connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 2,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 1,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();

    let external_reference = "External Reference".to_string();
    cart.add_credit_card_payment(
        user.id,
        500,
        PaymentProviders::Stripe,
        external_reference.clone(),
        PaymentStatus::Completed,
        serde_json::Value::Null,
        connection,
    )
    .unwrap();
    assert_eq!(OrderStatus::Draft, cart.status);
    let payment = &cart.payments(connection).unwrap()[0];
    assert_eq!(Some(external_reference.clone()), payment.external_reference);
    assert_eq!(PaymentProviders::Stripe, payment.provider);
    assert_eq!(PaymentStatus::Completed, payment.status);

    let remaining = cart.calculate_total(connection).unwrap() - 500;
    cart.add_credit_card_payment(
        user.id,
        remaining,
        PaymentProviders::Stripe,
        external_reference.clone(),
        PaymentStatus::Completed,
        serde_json::Value::Null,
        connection,
    )
    .unwrap();
    assert_eq!(OrderStatus::Paid, cart.status);
    let payment = &cart.payments(connection).unwrap()[1];
    assert_eq!(Some(external_reference.clone()), payment.external_reference);
    assert_eq!(PaymentProviders::Stripe, payment.provider);
    assert_eq!(PaymentStatus::Completed, payment.status);
}

#[test]
fn add_checkout_url() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let checkout_url = "http://example.com".to_string();
    let expires = NaiveDateTime::from(Utc::now().naive_utc() + Duration::minutes(10));
    let domain_event_count = DomainEvent::find(
        Tables::Orders,
        Some(cart.id),
        Some(DomainEventTypes::OrderUpdated),
        connection,
    )
    .unwrap()
    .len();
    assert_eq!(0, domain_event_count);

    cart.add_checkout_url(user.id, checkout_url.clone(), expires.clone(), connection)
        .unwrap();
    let domain_event_count = DomainEvent::find(
        Tables::Orders,
        Some(cart.id),
        Some(DomainEventTypes::OrderUpdated),
        connection,
    )
    .unwrap()
    .len();
    assert_eq!(2, domain_event_count);
    assert_eq!(Some(checkout_url.clone()), cart.checkout_url);
    assert_eq!(
        Some(expires.clone().timestamp()),
        cart.expires_at.map(|e| e.timestamp())
    );
}

#[test]
fn order_items_in_invalid_state() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let code = project
        .create_code()
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_type)
        .with_discount_in_cents(Some(10))
        .finish();

    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    assert!(cart
        .order_items_in_invalid_state(connection)
        .unwrap()
        .is_empty());

    // Ticket associated was nullified
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .into_iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    diesel::update(
        ticket_instances::table.filter(ticket_instances::order_item_id.eq(order_item.id)),
    )
    .set((ticket_instances::status.eq(TicketInstanceStatus::Nullified),))
    .execute(connection)
    .unwrap();
    assert_eq!(
        cart.order_items_in_invalid_state(connection).unwrap(),
        vec![order_item.clone()]
    );

    // Past reservation date on ticket
    cart.clear_cart(user.id, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .into_iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    let one_minute_ago = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(1));
    diesel::update(
        ticket_instances::table.filter(ticket_instances::order_item_id.eq(order_item.id)),
    )
    .set((ticket_instances::reserved_until.eq(one_minute_ago),))
    .execute(connection)
    .unwrap();
    assert_eq!(
        cart.order_items_in_invalid_state(connection).unwrap(),
        vec![order_item.clone()]
    );

    // Code end date has passed
    cart.clear_cart(user.id, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: Some(code.redemption_code.clone()),
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .into_iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    code.update(
        UpdateCodeAttributes {
            end_date: Some(one_minute_ago),
            ..Default::default()
        },
        None,
        connection,
    )
    .unwrap();
    assert_eq!(
        cart.order_items_in_invalid_state(connection).unwrap(),
        vec![order_item.clone()]
    );

    // Hold end date has passed
    cart.clear_cart(user.id, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: hold.redemption_code.clone(),
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .into_iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    hold.update(
        UpdateHoldAttributes {
            end_at: Some(Some(one_minute_ago)),
            ..Default::default()
        },
        connection,
    )
    .unwrap();
    assert_eq!(
        cart.order_items_in_invalid_state(connection).unwrap(),
        vec![order_item.clone()]
    );

    // Ticket quantity does not equal order item count
    cart.clear_cart(user.id, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .into_iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    diesel::delete(
        ticket_instances::table.filter(ticket_instances::order_item_id.eq(order_item.id)),
    )
    .execute(connection)
    .unwrap();
    assert_eq!(
        cart.order_items_in_invalid_state(connection).unwrap(),
        vec![order_item.clone()]
    );
}

#[test]
fn reset_to_draft() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();

    // Paid fails with error
    let cart = Order::find_or_create_cart(&user, connection).unwrap();
    diesel::update(orders::table.filter(orders::id.eq(cart.id)))
        .set((orders::status.eq(OrderStatus::Paid),))
        .execute(connection)
        .unwrap();

    let mut cart = Order::find(cart.id, connection).unwrap();
    assert_eq!(
        cart.reset_to_draft(Some(user.id), connection),
        DatabaseError::business_process_error("Cannot reset to draft, the order is already paid",)
    );

    // Cancelled fails with error
    diesel::update(orders::table.filter(orders::id.eq(cart.id)))
        .set((orders::status.eq(OrderStatus::Cancelled),))
        .execute(connection)
        .unwrap();
    let mut cart = Order::find(cart.id, connection).unwrap();
    assert_eq!(
        cart.reset_to_draft(Some(user.id), connection),
        DatabaseError::business_process_error(
            "Cannot reset this order because it has been cancelled",
        )
    );

    // Draft does nothing
    diesel::update(orders::table.filter(orders::id.eq(cart.id)))
        .set((orders::status.eq(OrderStatus::Draft),))
        .execute(connection)
        .unwrap();
    let mut cart = Order::find(cart.id, connection).unwrap();
    assert!(cart.reset_to_draft(Some(user.id), connection).is_ok());
    assert_eq!(cart.status, OrderStatus::Draft);

    // PendingPayment is set back to draft
    diesel::update(orders::table.filter(orders::id.eq(cart.id)))
        .set((orders::status.eq(OrderStatus::PendingPayment),))
        .execute(connection)
        .unwrap();
    let mut cart = Order::find(cart.id, connection).unwrap();
    assert!(cart.reset_to_draft(Some(user.id), connection).is_ok());
    assert_eq!(cart.status, OrderStatus::Draft);
}

#[test]
fn total_paid() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_types = &event.ticket_types(true, None, connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 2,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 1,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();
    assert_eq!(cart.total_paid(connection).unwrap(), 0);

    let external_reference = "External Reference".to_string();
    cart.add_provider_payment(
        Some(external_reference.clone()),
        PaymentProviders::Stripe,
        Some(user.id),
        500,
        PaymentStatus::Completed,
        None,
        serde_json::Value::Null,
        connection,
    )
    .unwrap();
    assert_eq!(OrderStatus::Draft, cart.status);
    assert_eq!(cart.total_paid(connection).unwrap(), 500);

    let remaining = cart.calculate_total(connection).unwrap() - 500;
    cart.add_provider_payment(
        Some(external_reference.clone()),
        PaymentProviders::Stripe,
        Some(user.id),
        remaining,
        PaymentStatus::Completed,
        None,
        serde_json::Value::Null,
        connection,
    )
    .unwrap();
    assert_eq!(OrderStatus::Paid, cart.status);
    assert_eq!(
        cart.total_paid(connection).unwrap(),
        cart.calculate_total(connection).unwrap()
    );
}

#[test]
fn clear_invalid_items() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_types = event.ticket_types(true, None, connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];

    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    assert!(cart
        .order_items_in_invalid_state(connection)
        .unwrap()
        .is_empty());

    // Ticket associated was nullified
    cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 2,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 2,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    )
    .unwrap();
    let order_item = cart
        .items(&connection)
        .unwrap()
        .into_iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    diesel::update(
        ticket_instances::table.filter(ticket_instances::order_item_id.eq(order_item.id)),
    )
    .set((ticket_instances::status.eq(TicketInstanceStatus::Nullified),))
    .execute(connection)
    .unwrap();
    assert_eq!(
        cart.order_items_in_invalid_state(connection).unwrap(),
        vec![order_item.clone()]
    );
    cart.clear_invalid_items(user.id, connection).unwrap();

    assert!(OrderItem::find(order_item.id, connection).is_err());
    assert!(cart
        .items(connection)
        .unwrap()
        .into_iter()
        .find(|i| i.ticket_type_id == Some(ticket_type2.id))
        .is_some());

    diesel::update(orders::table.filter(orders::id.eq(cart.id)))
        .set((orders::status.eq(OrderStatus::Paid),))
        .execute(connection)
        .unwrap();
    let mut cart = Order::find(cart.id, connection).unwrap();
    assert_eq!(
        cart.clear_invalid_items(user.id, connection),
        DatabaseError::validation_error(
            "status",
            "Cannot clear invalid items unless the order is in draft status",
        )
    );
}

#[test]
fn calculate_total() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    assert_eq!(0, cart.calculate_total(connection).unwrap());

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let mut total = 0;
    for item in items {
        total += item.quantity * item.unit_price_in_cents;
    }
    assert_eq!(total, cart.calculate_total(connection).unwrap());
}

#[test]
fn lock_version() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    assert_eq!(0, cart.version);

    assert!(cart.lock_version(connection).is_ok());
    assert_eq!(1, cart.version);

    assert!(cart.lock_version(connection).is_ok());
    assert_eq!(2, cart.version);
}

#[test]
fn set_behalf_of_user() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let behalf_of_user = project.create_user().finish();
    let behalf_of_user2 = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    assert!(cart.on_behalf_of_user_id.is_none());
    let domain_events = DomainEvent::find(
        Tables::Orders,
        Some(cart.id),
        Some(DomainEventTypes::OrderBehalfOfUserChanged),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    cart.set_behalf_of_user(behalf_of_user.clone(), user.id, connection)
        .unwrap();
    assert_eq!(Some(behalf_of_user.id), cart.on_behalf_of_user_id);
    let domain_events = DomainEvent::find(
        Tables::Orders,
        Some(cart.id),
        Some(DomainEventTypes::OrderBehalfOfUserChanged),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());

    cart.set_behalf_of_user(behalf_of_user2.clone(), user.id, connection)
        .unwrap();
    assert_eq!(Some(behalf_of_user2.id), cart.on_behalf_of_user_id);
    let domain_events = DomainEvent::find(
        Tables::Orders,
        Some(cart.id),
        Some(DomainEventTypes::OrderBehalfOfUserChanged),
        connection,
    )
    .unwrap();
    assert_eq!(2, domain_events.len());

    diesel::update(orders::table.filter(orders::id.eq(cart.id)))
        .set((orders::status.eq(OrderStatus::Paid),))
        .execute(connection)
        .unwrap();
    let mut cart = Order::find(cart.id, connection).unwrap();
    assert_eq!(
        cart.set_behalf_of_user(behalf_of_user.clone(), user.id, connection),
        DatabaseError::validation_error(
            "status",
            "Cannot change the order user unless the order is in draft status",
        )
    );
}

#[test]
fn create() {
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
    let order = Order::find_or_create_cart(&user, project.get_connection()).unwrap();
    assert_eq!(order.user_id, user.id);
    assert_eq!(order.id.to_string().is_empty(), false);
    let domain_events = DomainEvent::find(
        Tables::Orders,
        Some(order.id),
        Some(DomainEventTypes::OrderCreated),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
}

#[test]
fn set_external_payment_type() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut order = project.create_order().finish();
    let user = project.create_user().finish();
    assert_eq!(None, order.external_payment_type);

    let domain_event_count = DomainEvent::find(
        Tables::Orders,
        Some(order.id),
        Some(DomainEventTypes::OrderUpdated),
        connection,
    )
    .unwrap()
    .len();

    assert!(order
        .set_external_payment_type(ExternalPaymentType::CreditCard, user.id, connection)
        .is_ok());
    assert_eq!(
        Some(ExternalPaymentType::CreditCard),
        order.external_payment_type
    );

    let new_domain_event_count = DomainEvent::find(
        Tables::Orders,
        Some(order.id),
        Some(DomainEventTypes::OrderUpdated),
        connection,
    )
    .unwrap()
    .len();
    assert_eq!(domain_event_count + 1, new_domain_event_count);
}

#[test]
fn update_quantities_check_limits() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = event
        .ticket_types(true, None, connection)
        .unwrap()
        .remove(0);

    // Ticket type with no limit
    assert_eq!(ticket_type.limit_per_person, 0);
    assert!(cart
        .update_quantities(
            user.id,
            &vec![UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 10,
                redemption_code: None,
            }],
            false,
            true,
            connection,
        )
        .is_ok());

    // Ticket type with limit
    let ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                limit_per_person: Some(4),
                ..Default::default()
            },
            None,
            connection,
        )
        .unwrap();
    assert_eq!(ticket_type.limit_per_person, 4);
    assert!(cart
        .update_quantities(
            user.id,
            &vec![UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 4,
                redemption_code: None,
            }],
            false,
            true,
            connection,
        )
        .is_ok());
    let result = cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 5,
            redemption_code: None,
        }],
        false,
        true,
        connection,
    );
    assert!(result.is_err());
    let error = result.unwrap_err();
    match &error.error_code {
        ValidationError { errors } => {
            assert!(errors.contains_key("quantity"));
            assert_eq!(errors["quantity"].len(), 1);
            assert_eq!(errors["quantity"][0].code, "limit_per_person_exceeded");
            assert_eq!(
                &errors["quantity"][0].message.clone().unwrap().into_owned(),
                "You have exceeded the max tickets per customer limit."
            );
        }
        _ => panic!("Expected validation error"),
    }

    // Hold with no limit (under ticket type limit)
    let hold = project
        .create_hold()
        .with_ticket_type_id(ticket_type.id)
        .finish();
    assert!(hold.max_per_user.is_none());
    assert!(cart
        .update_quantities(
            user.id,
            &vec![UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 3,
                redemption_code: hold.redemption_code,
            }],
            false,
            true,
            connection,
        )
        .is_ok());

    // Hold with limit of 0
    let hold = project
        .create_hold()
        .with_ticket_type_id(ticket_type.id)
        .with_max_per_user(0)
        .finish();
    assert_eq!(hold.max_per_user, Some(0));
    assert!(cart
        .update_quantities(
            user.id,
            &vec![UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 3,
                redemption_code: hold.redemption_code,
            }],
            false,
            true,
            connection,
        )
        .is_ok());

    // Hold with limit above 0
    let hold = project
        .create_hold()
        .with_ticket_type_id(ticket_type.id)
        .with_max_per_user(3)
        .finish();
    assert_eq!(hold.max_per_user, Some(3));
    assert!(cart
        .update_quantities(
            user.id,
            &vec![UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 3,
                redemption_code: hold.redemption_code.clone(),
            }],
            false,
            true,
            connection,
        )
        .is_ok());
    let result = cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 4,
            redemption_code: hold.redemption_code.clone(),
        }],
        false,
        true,
        connection,
    );
    assert!(result.is_err());
    let error = result.unwrap_err();
    match &error.error_code {
        ValidationError { errors } => {
            assert!(errors.contains_key("quantity"));
            assert_eq!(errors["quantity"].len(), 1);
            assert_eq!(errors["quantity"][0].code, "limit_per_person_exceeded");
            assert_eq!(
                &errors["quantity"][0].message.clone().unwrap().into_owned(),
                &format!(
                    "Max of {} uses for code {} exceeded",
                    3,
                    hold.redemption_code.unwrap()
                )
            );
        }
        _ => panic!("Expected validation error"),
    }

    // Hold order but ticket type limit reached prior to hold limit
    let hold = project
        .create_hold()
        .with_ticket_type_id(ticket_type.id)
        .with_max_per_user(3)
        .finish();
    assert_eq!(hold.max_per_user, Some(3));
    assert!(cart
        .update_quantities(
            user.id,
            &vec![UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 3,
                redemption_code: hold.redemption_code.clone(),
            }],
            false,
            true,
            connection,
        )
        .is_ok());
    let ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                limit_per_person: Some(2),
                ..Default::default()
            },
            None,
            connection,
        )
        .unwrap();
    cart.clear_cart(user.id, connection).unwrap();
    let result = cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 3,
            redemption_code: hold.redemption_code.clone(),
        }],
        false,
        true,
        connection,
    );
    assert!(result.is_err());
    let error = result.unwrap_err();
    match &error.error_code {
        ValidationError { errors } => {
            assert!(errors.contains_key("quantity"));
            assert_eq!(errors["quantity"].len(), 1);
            assert_eq!(errors["quantity"][0].code, "limit_per_person_exceeded");
            assert_eq!(
                &errors["quantity"][0].message.clone().unwrap().into_owned(),
                "You have exceeded the max tickets per customer limit."
            );
        }
        _ => panic!("Expected validation error"),
    }
    let ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                limit_per_person: Some(4),
                ..Default::default()
            },
            None,
            connection,
        )
        .unwrap();

    // Code with no limit (under ticket type limit)
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .with_max_tickets_per_user(None)
        .finish();
    assert!(code.max_tickets_per_user.is_none());
    assert!(cart
        .update_quantities(
            user.id,
            &vec![UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 3,
                redemption_code: Some(code.redemption_code),
            }],
            false,
            true,
            connection,
        )
        .is_ok());

    // Code with limit of 0
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .with_max_tickets_per_user(Some(0))
        .finish();
    assert_eq!(code.max_tickets_per_user, Some(0));
    assert!(cart
        .update_quantities(
            user.id,
            &vec![UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 3,
                redemption_code: Some(code.redemption_code),
            }],
            false,
            true,
            connection,
        )
        .is_ok());

    // Code with limit above 0
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .with_max_tickets_per_user(Some(3))
        .finish();
    assert_eq!(code.max_tickets_per_user, Some(3));
    assert!(cart
        .update_quantities(
            user.id,
            &vec![UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 3,
                redemption_code: Some(code.redemption_code.clone()),
            }],
            false,
            true,
            connection,
        )
        .is_ok());
    let result = cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 4,
            redemption_code: Some(code.redemption_code.clone()),
        }],
        false,
        true,
        connection,
    );
    assert!(result.is_err());
    let error = result.unwrap_err();
    match &error.error_code {
        ValidationError { errors } => {
            assert!(errors.contains_key("quantity"));
            assert_eq!(errors["quantity"].len(), 1);
            assert_eq!(errors["quantity"][0].code, "limit_per_person_exceeded");
            assert_eq!(
                &errors["quantity"][0].message.clone().unwrap().into_owned(),
                &format!(
                    "Max of {} uses for code {} exceeded",
                    3, code.redemption_code
                )
            );
        }
        _ => panic!("Expected validation error"),
    }

    // Code order but ticket type limit reached prior to hold limit
    let code = project
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .with_max_tickets_per_user(Some(3))
        .finish();
    assert_eq!(code.max_tickets_per_user, Some(3));
    assert!(cart
        .update_quantities(
            user.id,
            &vec![UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 3,
                redemption_code: Some(code.redemption_code.clone()),
            }],
            false,
            true,
            connection,
        )
        .is_ok());
    let ticket_type = ticket_type
        .update(
            TicketTypeEditableAttributes {
                limit_per_person: Some(2),
                ..Default::default()
            },
            None,
            connection,
        )
        .unwrap();
    cart.clear_cart(user.id, connection).unwrap();
    let result = cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 3,
            redemption_code: Some(code.redemption_code.clone()),
        }],
        false,
        true,
        connection,
    );
    assert!(result.is_err());
    let error = result.unwrap_err();
    match &error.error_code {
        ValidationError { errors } => {
            assert!(errors.contains_key("quantity"));
            assert_eq!(errors["quantity"].len(), 1);
            assert_eq!(errors["quantity"][0].code, "limit_per_person_exceeded");
            assert_eq!(
                &errors["quantity"][0].message.clone().unwrap().into_owned(),
                "You have exceeded the max tickets per customer limit."
            );
        }
        _ => panic!("Expected validation error"),
    }
}

#[test]
fn add_tickets() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket.id))
        .unwrap();

    let ticket_pricing =
        TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.calculate_quantity(connection), Ok(10));
    assert_eq!(
        order_item.unit_price_in_cents,
        ticket_pricing.price_in_cents
    );

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range =
        FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(
        fee_item.unit_price_in_cents,
        fee_schedule_range.fee_in_cents
    );
    assert_eq!(fee_item.quantity, 10);

    // Add some more
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 15,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(connection).unwrap();

    assert_eq!(items.len(), 2);
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket.id))
        .unwrap();

    assert_eq!(order_item.calculate_quantity(connection), Ok(15));
}

#[test]
fn add_tickets_below_min_fee() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .finish();

    let ticket_type = event
        .add_ticket_type(
            "Free Tix".to_string(),
            None,
            10,
            Some(times::zero()),
            None,
            TicketTypeEndDateType::EventEnd,
            Some(event.issuer_wallet(connection).unwrap().id),
            Some(1),
            10,
            0,
            TicketTypeVisibility::Always,
            None,
            0,
            true,
            true,
            true,
            None,
            connection,
        )
        .unwrap();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();

    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    assert_eq!(order_item.unit_price_in_cents, 0);
    assert_eq!(items.len(), 1);

    let fee_item = order_item.find_fee_item(connection).unwrap();
    assert_eq!(fee_item, None);
}

#[test]
fn events() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_name("Event1".into())
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_type_count(2)
        .with_ticket_pricing()
        .finish();
    let mut ticket_types = event.ticket_types(true, None, connection).unwrap();
    let ticket_type = ticket_types.remove(0);
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    assert_eq!(vec![event], cart.events(connection).unwrap());
}

#[test]
fn purchase_metadata() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_fees()
        .finish();
    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection).unwrap();
    let event = project
        .create_event()
        .with_name("EventName".into())
        .with_organization(&organization)
        .with_tickets()
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(12, 0, 0))
        .with_ticket_pricing()
        .finish();
    let ticket_type = event
        .ticket_types(true, None, connection)
        .unwrap()
        .remove(0);
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let cost_per_ticket = 150;
    let fee_in_cents = fee_schedule
        .get_range(cost_per_ticket, connection)
        .unwrap()
        .fee_in_cents;
    let items = cart.items(connection).unwrap();
    let event_fee_item = items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees)
        .unwrap();
    let event_fee = event_fee_item.unit_price_in_cents;

    let mut expected: Vec<(String, String)> = Vec::new();
    expected.push(("order_id".to_string(), cart.id.to_string()));
    expected.push(("event_names".to_string(), "EventName".to_string()));
    expected.push(("event_dates".to_string(), "2016-07-08".to_string()));
    expected.push(("venue_names".to_string(), "".to_string()));
    expected.push(("user_id".to_string(), user.id.to_string()));
    expected.push(("user_name".to_string(), user.full_name()));
    expected.push(("ticket_quantity".to_string(), 2.to_string()));
    expected.push((
        "face_value_in_cents".to_string(),
        (cost_per_ticket * 2).to_string(),
    ));
    expected.push((
        "fees_in_cents".to_string(),
        (event_fee + fee_in_cents * 2).to_string(),
    ));
    assert_eq!(expected, cart.purchase_metadata(connection).unwrap());
}

#[test]
fn items_valid_for_purchase() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = event
        .ticket_types(true, None, connection)
        .unwrap()
        .remove(0);
    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(1)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let code = project
        .create_code()
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_type)
        .with_discount_in_cents(Some(10))
        .finish();

    // Normal order for ticket type
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    assert!(cart.items_valid_for_purchase(connection).unwrap());

    // Order item with no associated ticket_instances record
    let items = cart.items(connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    let none_uuid: Option<Uuid> = None;
    diesel::update(
        ticket_instances::table.filter(ticket_instances::order_item_id.eq(order_item.id)),
    )
    .set(ticket_instances::order_item_id.eq(none_uuid))
    .execute(connection)
    .unwrap();
    assert!(!cart.items_valid_for_purchase(connection).unwrap());

    // Order item with past reserved until date
    let one_minute_ago = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(1));
    diesel::update(
        ticket_instances::table.filter(ticket_instances::order_item_id.eq(order_item.id)),
    )
    .set((
        ticket_instances::order_item_id.eq(order_item.id),
        ticket_instances::reserved_until.eq(one_minute_ago),
    ))
    .execute(connection)
    .unwrap();
    assert!(!cart.items_valid_for_purchase(connection).unwrap());

    // Order item ticket nullified
    let one_minute_from_now = NaiveDateTime::from(Utc::now().naive_utc() + Duration::minutes(1));
    diesel::update(
        ticket_instances::table.filter(ticket_instances::order_item_id.eq(order_item.id)),
    )
    .set((
        ticket_instances::reserved_until.eq(one_minute_from_now),
        ticket_instances::status.eq(TicketInstanceStatus::Nullified),
    ))
    .execute(connection)
    .unwrap();
    assert!(!cart.items_valid_for_purchase(connection).unwrap());

    // Code with end date in the past
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: Some(code.redemption_code.clone()),
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert!(cart.items_valid_for_purchase(connection).unwrap());

    code.update(
        UpdateCodeAttributes {
            end_date: Some(one_minute_ago),
            ..Default::default()
        },
        None,
        connection,
    )
    .unwrap();
    assert!(!cart.items_valid_for_purchase(connection).unwrap());

    // Hold with end at date in the past
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: hold.redemption_code.clone(),
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert!(cart.items_valid_for_purchase(connection).unwrap());

    hold.update(
        UpdateHoldAttributes {
            end_at: Some(Some(one_minute_ago)),
            ..Default::default()
        },
        connection,
    )
    .unwrap();
    assert!(!cart.items_valid_for_purchase(connection).unwrap());
}

#[test]
fn partially_visible_order() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_event_fee().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    // With no events filter and no filter of organizations
    OrganizationUser::create(organization.id, user2.id, vec![Roles::OrgMember])
        .commit(connection)
        .unwrap();
    assert!(!cart
        .partially_visible_order(&vec![organization.id], user2.id, connection)
        .unwrap());

    // With access and event filter
    OrganizationUser::create(organization.id, user2.id, vec![Roles::Promoter])
        .commit(connection)
        .unwrap();
    assert!(cart
        .partially_visible_order(&vec![organization.id], user2.id, connection)
        .unwrap());

    // With access and event filter
    organization
        .add_user(user2.id, vec![Roles::Promoter], vec![event.id], connection)
        .unwrap();

    assert!(!cart
        .partially_visible_order(&vec![organization.id], user2.id, connection)
        .unwrap());

    assert!(cart
        .partially_visible_order(&vec![], user2.id, connection)
        .unwrap());
}

#[test]
fn details() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_fees()
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_a_specific_number_of_tickets(2)
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    OrganizationUser::create(organization.id, user2.id, vec![Roles::OrgMember])
        .commit(connection)
        .unwrap();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();

    let items = cart.items(connection).unwrap();
    let order_item = OrderItem::find(
        items
            .iter()
            .find(|i| i.ticket_type_id == Some(ticket_type.id))
            .unwrap()
            .id,
        connection,
    )
    .unwrap();

    let event_fee_item = items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees)
        .unwrap();

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    let ticket2 = &tickets[1];

    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    let refund_amount = order_item.unit_price_in_cents + fee_item.unit_price_in_cents;
    let (_refund, amount) = cart
        .refund(&refund_items, user.id, None, connection)
        .unwrap();
    assert_eq!(amount, refund_amount);

    let mut expected_order_details = vec![
        OrderDetailsLineItem {
            ticket_instance_id: Some(ticket2.id),
            order_item_id: order_item.id,
            description: format!("{} - {}", event.name, ticket_type.name),
            ticket_price_in_cents: order_item.unit_price_in_cents,
            fees_price_in_cents: fee_item.unit_price_in_cents,
            total_price_in_cents: order_item.unit_price_in_cents + fee_item.unit_price_in_cents,
            status: "Purchased".to_string(),
            refundable: true,
            attendee_email: user.email.clone(),
            attendee_id: Some(user.id),
            attendee_first_name: user.first_name.clone(),
            attendee_last_name: user.last_name.clone(),
            ticket_type_id: Some(ticket_type.id),
            ticket_type_name: Some(ticket_type.name.clone()),
            code: None,
            code_type: None,
            pending_transfer_id: None,
            discount_price_in_cents: None,
        },
        OrderDetailsLineItem {
            ticket_instance_id: Some(ticket.id),
            order_item_id: order_item.id,
            description: format!("{} - {}", event.name, ticket_type.name),
            ticket_price_in_cents: 150,
            fees_price_in_cents: 20,
            total_price_in_cents: 170,
            status: "Refunded".to_string(),
            refundable: false,
            attendee_email: None,
            attendee_id: None,
            attendee_first_name: None,
            attendee_last_name: None,
            ticket_type_id: Some(ticket_type.id),
            ticket_type_name: Some(ticket_type.name.clone()),
            code: None,
            code_type: None,
            pending_transfer_id: None,
            discount_price_in_cents: None,
        },
    ];

    expected_order_details.sort_by(|a, b| {
        a.ticket_instance_id
            .unwrap()
            .cmp(&b.ticket_instance_id.unwrap())
    });
    expected_order_details.push(OrderDetailsLineItem {
        ticket_instance_id: None,
        order_item_id: event_fee_item.id,
        description: format!("Event Fees - {}", event.name),
        ticket_price_in_cents: 0,
        fees_price_in_cents: event_fee_item.unit_price_in_cents,
        total_price_in_cents: event_fee_item.unit_price_in_cents,
        status: "Purchased".to_string(),
        refundable: true,
        attendee_email: None,
        attendee_id: None,
        attendee_first_name: None,
        attendee_last_name: None,
        ticket_type_id: None,
        ticket_type_name: None,
        code: None,
        code_type: None,
        pending_transfer_id: None,
        discount_price_in_cents: None,
    });

    let order_details = cart
        .details(&vec![organization.id], user2.id, connection)
        .unwrap();
    assert_eq!(expected_order_details, order_details);

    // No details when this organization is not specified
    assert!(cart
        .details(&vec![], user2.id, connection)
        .unwrap()
        .is_empty());

    // Refund already refunded ticket which doesn't change anything
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    assert!(cart
        .refund(&refund_items, user.id, None, connection)
        .is_err());
    let order_details = cart
        .details(&vec![organization.id], user2.id, connection)
        .unwrap();
    assert_eq!(expected_order_details, order_details);

    // Refund last item triggering event fee to refund as well
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket2.id),
    }];
    let refund_amount = order_item.unit_price_in_cents + fee_item.unit_price_in_cents;
    let (_refund, amount) = cart
        .refund(&refund_items, user.id, None, connection)
        .unwrap();
    assert_eq!(amount, refund_amount);

    let mut expected_order_details = vec![
        OrderDetailsLineItem {
            ticket_instance_id: Some(ticket2.id),
            order_item_id: order_item.id,
            description: format!("{} - {}", event.name, ticket_type.name),
            ticket_price_in_cents: 150,
            fees_price_in_cents: 20,
            total_price_in_cents: 170,
            status: "Refunded".to_string(),
            refundable: false,
            attendee_email: None,
            attendee_id: None,
            attendee_first_name: None,
            attendee_last_name: None,
            ticket_type_id: Some(ticket_type.id),
            ticket_type_name: Some(ticket_type.name.clone()),
            code: None,
            code_type: None,
            pending_transfer_id: None,
            discount_price_in_cents: None,
        },
        OrderDetailsLineItem {
            ticket_instance_id: Some(ticket.id),
            order_item_id: order_item.id,
            description: format!("{} - {}", event.name, ticket_type.name),
            ticket_price_in_cents: 150,
            fees_price_in_cents: 20,
            total_price_in_cents: 170,
            status: "Refunded".to_string(),
            refundable: false,
            attendee_email: None,
            attendee_id: None,
            attendee_first_name: None,
            attendee_last_name: None,
            ticket_type_id: Some(ticket_type.id),
            ticket_type_name: Some(ticket_type.name.clone()),
            code: None,
            code_type: None,
            pending_transfer_id: None,
            discount_price_in_cents: None,
        },
    ];

    expected_order_details.sort_by(|a, b| {
        a.ticket_instance_id
            .unwrap()
            .cmp(&b.ticket_instance_id.unwrap())
    });
    expected_order_details.push(OrderDetailsLineItem {
        ticket_instance_id: None,
        order_item_id: event_fee_item.id,
        description: format!("Event Fees - {}", event.name),
        ticket_price_in_cents: 0,
        fees_price_in_cents: 250,
        total_price_in_cents: 250,
        status: "Purchased".to_string(),
        refundable: true,
        attendee_email: None,
        attendee_id: None,
        attendee_first_name: None,
        attendee_last_name: None,
        ticket_type_id: None,
        ticket_type_name: None,
        code: None,
        code_type: None,
        pending_transfer_id: None,
        discount_price_in_cents: None,
    });

    let order_details = cart
        .details(&vec![organization.id], user2.id, connection)
        .unwrap();
    assert_eq!(expected_order_details, order_details);

    // With events filter
    OrganizationUser::create(organization.id, user2.id, vec![Roles::Promoter])
        .commit(connection)
        .unwrap();
    let order_details = cart
        .details(&vec![organization.id], user2.id, connection)
        .unwrap();
    assert!(order_details.is_empty());

    // With access and event filter
    organization
        .add_user(user2.id, vec![Roles::Promoter], vec![event.id], connection)
        .unwrap();
    let order_details = cart
        .details(&vec![organization.id], user2.id, connection)
        .unwrap();
    assert_eq!(expected_order_details, order_details);

    // Order details for box office purchase
    let user3 = project.create_user().finish();
    let box_office_order = project
        .create_order()
        .quantity(1)
        .for_event(&event)
        .for_user(&user2)
        .on_behalf_of_user(&user3)
        .is_paid()
        .finish();
    let items = box_office_order.items(connection).unwrap();
    let order_item = OrderItem::find(
        items
            .iter()
            .find(|i| i.ticket_type_id == Some(ticket_type.id))
            .unwrap()
            .id,
        connection,
    )
    .unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];
    let expected_order_details = vec![OrderDetailsLineItem {
        ticket_instance_id: Some(ticket.id),
        order_item_id: order_item.id,
        description: format!("{} - {}", event.name, ticket_type.name),
        ticket_price_in_cents: 150,
        fees_price_in_cents: 0,
        total_price_in_cents: 150,
        status: "Purchased".to_string(),
        refundable: true,
        attendee_email: user3.email.clone(),
        attendee_id: Some(user3.id),
        attendee_first_name: user3.first_name.clone(),
        attendee_last_name: user3.last_name.clone(),
        ticket_type_id: Some(ticket_type.id),
        ticket_type_name: Some(ticket_type.name.clone()),
        code: None,
        code_type: None,
        pending_transfer_id: None,
        discount_price_in_cents: None,
    }];

    let order_details = box_office_order
        .details(&vec![organization.id], user2.id, connection)
        .unwrap();
    assert_eq!(expected_order_details, order_details);

    // Test behavior around reused ticket instances
    // Only one ticket remaining
    assert_eq!(
        ticket_type
            .valid_available_ticket_count(connection)
            .unwrap(),
        1
    );
    let mut new_order = project
        .create_order()
        .quantity(1)
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .finish();
    let items = new_order.items(connection).unwrap();
    let order_item = OrderItem::find(
        items
            .iter()
            .find(|i| i.ticket_type_id == Some(ticket_type.id))
            .unwrap()
            .id,
        connection,
    )
    .unwrap();
    let event_fee_item = items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees)
        .unwrap();
    let ticket = &TicketInstance::find_for_order_item(order_item.id, connection).unwrap()[0];
    let expected_order_details = vec![
        OrderDetailsLineItem {
            ticket_instance_id: Some(ticket.id),
            order_item_id: order_item.id,
            description: format!("{} - {}", event.name, ticket_type.name),
            ticket_price_in_cents: 150,
            fees_price_in_cents: 20,
            total_price_in_cents: 170,
            status: "Purchased".to_string(),
            refundable: true,
            attendee_email: user.email.clone(),
            attendee_id: Some(user.id),
            attendee_first_name: user.first_name.clone(),
            attendee_last_name: user.last_name.clone(),
            ticket_type_id: Some(ticket_type.id),
            ticket_type_name: Some(ticket_type.name.clone()),
            code: None,
            code_type: None,
            pending_transfer_id: None,
            discount_price_in_cents: None,
        },
        OrderDetailsLineItem {
            ticket_instance_id: None,
            order_item_id: event_fee_item.id,
            description: format!("Event Fees - {}", event.name),
            ticket_price_in_cents: 0,
            fees_price_in_cents: 250,
            total_price_in_cents: 250,
            status: "Purchased".to_string(),
            refundable: true,
            attendee_email: None,
            attendee_id: None,
            attendee_first_name: None,
            attendee_last_name: None,
            ticket_type_id: None,
            ticket_type_name: None,
            code: None,
            code_type: None,
            pending_transfer_id: None,
            discount_price_in_cents: None,
        },
    ];
    let order_details = new_order
        .details(&vec![organization.id], user.id, connection)
        .unwrap();
    assert_eq!(expected_order_details, order_details);

    // Refund order and create a new order
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(ticket.id),
    }];
    new_order
        .refund(&refund_items, user.id, None, connection)
        .unwrap();

    let new_order2 = project
        .create_order()
        .quantity(1)
        .for_event(&event)
        .for_user(&user2)
        .is_paid()
        .finish();
    let items = new_order2.items(connection).unwrap();
    let order_item2 = OrderItem::find(
        items
            .iter()
            .find(|i| i.ticket_type_id == Some(ticket_type.id))
            .unwrap()
            .id,
        connection,
    )
    .unwrap();
    let event_fee_item2 = items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees)
        .unwrap();
    let ticket2 = &TicketInstance::find_for_order_item(order_item2.id, connection).unwrap()[0];
    assert_eq!(ticket.id, ticket2.id);

    // Old order's details show refunded
    let expected_order_details = vec![
        OrderDetailsLineItem {
            ticket_instance_id: Some(ticket.id),
            order_item_id: order_item.id,
            description: format!("{} - {}", event.name, ticket_type.name),
            ticket_price_in_cents: 150,
            fees_price_in_cents: 20,
            total_price_in_cents: 170,
            status: "Refunded".to_string(),
            refundable: false,
            attendee_email: None,
            attendee_id: None,
            attendee_first_name: None,
            attendee_last_name: None,
            ticket_type_id: Some(ticket_type.id),
            ticket_type_name: Some(ticket_type.name.clone()),
            code: None,
            code_type: None,
            pending_transfer_id: None,
            discount_price_in_cents: None,
        },
        OrderDetailsLineItem {
            ticket_instance_id: None,
            order_item_id: event_fee_item.id,
            description: format!("Event Fees - {}", event.name),
            ticket_price_in_cents: 0,
            fees_price_in_cents: 250,
            total_price_in_cents: 250,
            status: "Purchased".to_string(),
            refundable: true,
            attendee_email: None,
            attendee_id: None,
            attendee_first_name: None,
            attendee_last_name: None,
            ticket_type_id: None,
            ticket_type_name: None,
            code: None,
            code_type: None,
            pending_transfer_id: None,
            discount_price_in_cents: None,
        },
    ];
    let order_details = new_order
        .details(&vec![organization.id], user.id, connection)
        .unwrap();
    assert_eq!(expected_order_details, order_details);

    // New order's details show purchased
    let expected_order_details2 = vec![
        OrderDetailsLineItem {
            ticket_instance_id: Some(ticket2.id),
            order_item_id: order_item2.id,
            description: format!("{} - {}", event.name, ticket_type.name),
            ticket_price_in_cents: 150,
            fees_price_in_cents: 20,
            total_price_in_cents: 170,
            status: "Purchased".to_string(),
            refundable: true,
            attendee_email: user2.email.clone(),
            attendee_id: Some(user2.id),
            attendee_first_name: user2.first_name.clone(),
            attendee_last_name: user2.last_name.clone(),
            ticket_type_id: Some(ticket_type.id),
            ticket_type_name: Some(ticket_type.name.clone()),
            code: None,
            code_type: None,
            pending_transfer_id: None,
            discount_price_in_cents: None,
        },
        OrderDetailsLineItem {
            ticket_instance_id: None,
            order_item_id: event_fee_item2.id,
            description: format!("Event Fees - {}", event.name),
            ticket_price_in_cents: 0,
            fees_price_in_cents: 250,
            total_price_in_cents: 250,
            status: "Purchased".to_string(),
            refundable: true,
            attendee_email: None,
            attendee_id: None,
            attendee_first_name: None,
            attendee_last_name: None,
            ticket_type_id: None,
            ticket_type_name: None,
            code: None,
            code_type: None,
            pending_transfer_id: None,
            discount_price_in_cents: None,
        },
    ];
    let order_details = new_order2
        .details(&vec![organization.id], user2.id, connection)
        .unwrap();
    assert_eq!(expected_order_details2, order_details);

    // Transfer new_order2's ticket to user3
    TicketInstance::direct_transfer(
        user2.id,
        &vec![ticket2.id],
        "nowhere",
        TransferMessageType::Email,
        user3.id,
        connection,
    )
    .unwrap();

    // First order's details unchanged by transfer of second order's ticket
    let order_details = new_order
        .details(&vec![organization.id], user.id, connection)
        .unwrap();
    assert_eq!(expected_order_details, order_details);

    // Second order shows transferred for status as a result of ticket transfer
    let expected_order_details2 = vec![
        OrderDetailsLineItem {
            ticket_instance_id: Some(ticket2.id),
            order_item_id: order_item2.id,
            description: format!("{} - {}", event.name, ticket_type.name),
            ticket_price_in_cents: 150,
            fees_price_in_cents: 20,
            total_price_in_cents: 170,
            status: "Transferred".to_string(),
            refundable: false,
            attendee_email: user3.email.clone(),
            attendee_id: Some(user3.id),
            attendee_first_name: user3.first_name.clone(),
            attendee_last_name: user3.last_name.clone(),
            ticket_type_id: Some(ticket_type.id),
            ticket_type_name: Some(ticket_type.name.clone()),
            code: None,
            code_type: None,
            pending_transfer_id: None,
            discount_price_in_cents: None,
        },
        OrderDetailsLineItem {
            ticket_instance_id: None,
            order_item_id: event_fee_item2.id,
            description: format!("Event Fees - {}", event.name),
            ticket_price_in_cents: 0,
            fees_price_in_cents: 250,
            total_price_in_cents: 250,
            status: "Purchased".to_string(),
            refundable: true,
            attendee_email: None,
            attendee_id: None,
            attendee_first_name: None,
            attendee_last_name: None,
            ticket_type_id: None,
            ticket_type_name: None,
            code: None,
            code_type: None,
            pending_transfer_id: None,
            discount_price_in_cents: None,
        },
    ];
    let order_details = new_order2
        .details(&vec![organization.id], user2.id, connection)
        .unwrap();
    assert_eq!(expected_order_details2, order_details);
}

#[test]
fn refund() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_fees()
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();

    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();

    let event_fee_item = items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees)
        .unwrap();

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let ticket = &tickets[0];

    // Refund first ticket and event fee (leaving one ticket + one fee item for that ticket)
    let refund_items = vec![
        RefundItemRequest {
            order_item_id: order_item.id,
            ticket_instance_id: Some(ticket.id),
        },
        RefundItemRequest {
            order_item_id: event_fee_item.id,
            ticket_instance_id: None,
        },
    ];
    let refund_amount = event_fee_item.unit_price_in_cents
        + order_item.unit_price_in_cents
        + fee_item.unit_price_in_cents;
    let (refund, amount) = cart
        .refund(&refund_items, user.id, None, connection)
        .unwrap();
    assert_eq!(amount, refund_amount);
    assert_eq!(refund.user_id, user.id);
    assert_eq!(refund.order_id, cart.id);

    // Each order item has a corresponding refund_item record
    let refund_items = refund.items(connection).unwrap();
    assert_eq!(refund_items.len(), 3);
    let found_item = refund_items
        .iter()
        .find(|ri| ri.order_item_id == order_item.id)
        .unwrap();
    let found_fee_item = refund_items
        .iter()
        .find(|ri| ri.order_item_id == fee_item.id)
        .unwrap();
    let found_event_fee_item = refund_items
        .iter()
        .find(|ri| ri.order_item_id == event_fee_item.id)
        .unwrap();

    assert_eq!(found_item.order_item_id, order_item.id);
    assert_eq!(found_item.amount, order_item.unit_price_in_cents);
    assert_eq!(found_item.quantity, 1);

    assert_eq!(found_fee_item.order_item_id, fee_item.id);
    assert_eq!(found_fee_item.amount, fee_item.unit_price_in_cents);
    assert_eq!(found_fee_item.quantity, 1);

    assert_eq!(found_event_fee_item.order_item_id, event_fee_item.id);
    assert_eq!(
        found_event_fee_item.amount,
        event_fee_item.unit_price_in_cents
    );
    assert_eq!(found_event_fee_item.quantity, 1);

    // Reload ticket
    let ticket = TicketInstance::find(ticket.id, connection).unwrap();
    assert!(ticket.order_item_id.is_none());

    // Reload order item
    let order_item = OrderItem::find_in_order(cart.id, order_item.id, connection).unwrap();
    assert_eq!(order_item.refunded_quantity, 1);

    // Reload fee item
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    assert_eq!(fee_item.refunded_quantity, 1);
}

#[test]
fn organizations() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_name("Organization1".into())
        .with_fees()
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    assert_eq!(cart.organizations(connection).unwrap(), vec![organization]);
}

#[test]
fn payments() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert_eq!(cart.calculate_total(connection).unwrap(), 2000);

    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        500,
        connection,
    )
    .unwrap();
    cart.add_external_payment(
        Some("Test2".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        1500,
        connection,
    )
    .unwrap();

    let payments = cart.payments(connection).unwrap();
    assert_eq!(payments.len(), 2);

    let mut payments = payments.iter().map(|p| p.amount).collect::<Vec<i64>>();
    payments.sort();
    assert_eq!(payments, vec![500, 1500]);
}

#[test]
fn add_tickets_with_increment() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = event
        .ticket_types(true, None, connection)
        .unwrap()
        .remove(0);
    let update_parameters = TicketTypeEditableAttributes {
        increment: Some(4),
        ..Default::default()
    };
    let ticket_type = ticket_type
        .update(update_parameters, None, connection)
        .unwrap();

    let add_tickets_result = cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    );
    assert!(add_tickets_result.is_err());
    let error = add_tickets_result.unwrap_err();
    match &error.error_code {
        ValidationError { errors } => {
            assert!(errors.contains_key("quantity"));
            assert_eq!(errors["quantity"].len(), 1);
            assert_eq!(errors["quantity"][0].code, "quantity_invalid_increment");
            assert_eq!(
                &errors["quantity"][0].message.clone().unwrap().into_owned(),
                "Order item quantity invalid for ticket pricing increment"
            );
        }
        _ => panic!("Expected validation error"),
    }

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 4,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 12,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();

    assert_eq!(order_item.quantity, 12);
}

#[test]
fn clear_cart() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert!(!cart.items(&connection).unwrap().is_empty());

    cart.clear_cart(user.id, connection).unwrap();
    assert!(cart.items(&connection).unwrap().is_empty());
}

#[test]
fn replace_tickets_for_box_office() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_event_fee()
        .with_fees()
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    assert!(!cart.box_office_pricing);

    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    let box_office_pricing = ticket_type
        .add_ticket_pricing(
            "Box office".into(),
            NaiveDate::from_ymd(2016, 7, 8).and_hms(7, 8, 10),
            NaiveDate::from_ymd(9999, 7, 8).and_hms(7, 8, 10),
            5000,
            true,
            None,
            None,
            connection,
        )
        .unwrap();

    // Add normal tickets to cart (box_office_pricing = false)
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    let ticket_pricing =
        TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.calculate_quantity(connection), Ok(10));
    assert_eq!(
        order_item.unit_price_in_cents,
        ticket_pricing.price_in_cents
    );

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range =
        FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(
        fee_item.unit_price_in_cents,
        fee_schedule_range.fee_in_cents
    );
    assert_eq!(fee_item.quantity, 10);
    assert!(items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees)
        .is_some());

    // Add box office priced tickets to cart (box_office_pricing = true)
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        true,
        true,
        connection,
    )
    .unwrap();
    assert!(cart.box_office_pricing);
    let items = cart.items(connection).unwrap();
    assert_eq!(items.len(), 1);
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    assert_eq!(order_item.calculate_quantity(connection), Ok(10));
    assert_eq!(
        order_item.unit_price_in_cents,
        box_office_pricing.price_in_cents
    );

    // No fee for box office items
    assert!(order_item.find_fee_item(connection).unwrap().is_none());
    assert!(items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees)
        .is_none());
}

#[test]
fn replace_tickets() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket.id))
        .unwrap();

    let ticket_pricing =
        TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.calculate_quantity(connection), Ok(10));
    assert_eq!(
        order_item.unit_price_in_cents,
        ticket_pricing.price_in_cents
    );

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range =
        FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(
        fee_item.unit_price_in_cents,
        fee_schedule_range.fee_in_cents
    );
    assert_eq!(fee_item.quantity, 10);

    // Add some more
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 15,
            redemption_code: None,
        }],
        false,
        true,
        connection,
    )
    .unwrap();
    let items = cart.items(connection).unwrap();

    assert_eq!(items.len(), 2);
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket.id))
        .unwrap();

    assert_eq!(order_item.calculate_quantity(connection), Ok(15));
}

#[test]
fn replace_tickets_with_code_pricing() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let discount_in_cents: i64 = 20;
    let code = project
        .create_code()
        .with_discount_in_cents(Some(discount_in_cents as u32))
        .with_code_type(CodeTypes::Access)
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .finish();
    cart.update_quantities(
        user.id,
        &vec![UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: Some(code.redemption_code.clone()),
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();

    let ticket_pricing =
        TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.calculate_quantity(connection), Ok(10));
    assert_eq!(
        order_item.unit_price_in_cents,
        ticket_pricing.price_in_cents
    );

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range =
        FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(
        fee_item.unit_price_in_cents,
        fee_schedule_range.fee_in_cents
    );
    assert_eq!(fee_item.quantity, 10);

    let discount_item = order_item.find_discount_item(connection).unwrap().unwrap();
    assert_eq!(discount_item.unit_price_in_cents, -20);
    assert_eq!(discount_item.quantity, 10);

    // Add some more
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 15,
            redemption_code: Some(code.redemption_code.clone()),
        }],
        false,
        true,
        connection,
    )
    .unwrap();
    let items = cart.items(connection).unwrap();

    assert_eq!(items.len(), 3);
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();

    assert_eq!(order_item.calculate_quantity(connection), Ok(15));
}

#[test]
fn deleted_hold_not_applied() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Comp)
        .finish();
    let redemption_code = hold.redemption_code.clone();
    let ticket_type_id = hold.ticket_type_id;
    hold.destroy(None, connection).unwrap();

    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let result = cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id,
            quantity: 1,
            redemption_code,
        }],
        false,
        true,
        connection,
    );

    assert!(result.is_err());
}

#[test]
fn remove_tickets() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();

    let ticket_pricing =
        TicketPricing::find(order_item.ticket_pricing_id.unwrap(), connection).unwrap();
    assert_eq!(order_item.quantity, 10);
    assert_eq!(
        order_item.unit_price_in_cents,
        ticket_pricing.price_in_cents
    );

    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    let fee_schedule_range =
        FeeScheduleRange::find(fee_item.fee_schedule_range_id.unwrap(), connection).unwrap();
    assert_eq!(
        fee_item.unit_price_in_cents,
        fee_schedule_range.fee_in_cents
    );
    assert_eq!(fee_item.quantity, 10);

    // Remove tickets
    assert!(cart
        .update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 6,
                redemption_code: None,
            }],
            false,
            false,
            connection
        )
        .is_ok());
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    assert_eq!(order_item.quantity, 6);
    assert_eq!(
        order_item.unit_price_in_cents,
        ticket_pricing.price_in_cents
    );
    let fee_item = order_item.find_fee_item(connection).unwrap().unwrap();
    assert_eq!(
        fee_item.unit_price_in_cents,
        fee_schedule_range.fee_in_cents
    );
    assert_eq!(fee_item.quantity, 6);

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 0,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    // Item removed from cart completely
    assert!(cart.items(connection).unwrap().is_empty());
}

#[test]
fn clear_tickets() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    // Remove tickets
    assert!(cart
        .update_quantities(user.id, &[], false, true, connection)
        .is_ok());

    // Item removed from cart completely
    assert!(cart.items(connection).unwrap().is_empty());
}

#[test]
fn remove_tickets_with_increment() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket_type = event
        .ticket_types(true, None, connection)
        .unwrap()
        .remove(0);
    let update_parameters = TicketTypeEditableAttributes {
        increment: Some(4),
        ..Default::default()
    };
    let ticket_type = ticket_type
        .update(update_parameters, None, connection)
        .unwrap();
    let add_tickets_result = cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 8,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    );
    assert!(add_tickets_result.is_ok());
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    assert_eq!(order_item.quantity, 8);

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 4,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();

    assert_eq!(order_item.quantity, 4);

    let remove_tickets_result = cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 5,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    );
    assert!(remove_tickets_result.is_err());
    let error = remove_tickets_result.unwrap_err();
    match &error.error_code {
        ValidationError { errors } => {
            assert!(errors.contains_key("quantity"));
            assert_eq!(errors["quantity"].len(), 1);
            assert_eq!(errors["quantity"][0].code, "quantity_invalid_increment");
            assert_eq!(
                &errors["quantity"][0].message.clone().unwrap().into_owned(),
                "Order item quantity invalid for ticket pricing increment"
            );
        }
        _ => panic!("Expected validation error"),
    }
}

#[test]
fn find_item() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let mut order = Order::find_or_create_cart(&user, connection).unwrap();
    order
        .update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 5,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();
    let mut order2 = Order::find_or_create_cart(&user2, connection).unwrap();
    order2
        .update_quantities(
            user.id,
            &[UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 10,
                redemption_code: None,
            }],
            false,
            false,
            connection,
        )
        .unwrap();

    let items = order.items(&connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    let items = order2.items(&connection).unwrap();
    let order_item2 = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();

    let found_item = order.find_item(order_item.id.clone(), connection).unwrap();
    assert_eq!(order_item, &found_item);

    let found_item = order2
        .find_item(order_item2.id.clone(), connection)
        .unwrap();
    assert_eq!(order_item2, &found_item);

    let find_results = order.find_item(order_item2.id.clone(), connection);
    assert_eq!(
        find_results.unwrap_err().cause,
        Some("Could not retrieve order item, NotFound".into())
    );

    let find_results = order2.find_item(order_item.id.clone(), connection);
    assert_eq!(
        find_results.unwrap_err().cause,
        Some("Could not retrieve order item, NotFound".into())
    );

    let find_results = order.find_item(Uuid::new_v4(), connection);
    assert_eq!(
        find_results.unwrap_err().cause,
        Some("Could not retrieve order item, NotFound".into())
    );
}

#[test]
fn find_cart_for_user() {
    let project = TestProject::new();
    let user = project.create_user().finish();

    // No cart
    let conn = project.get_connection();
    let cart_result = Order::find_cart_for_user(user.id, conn).unwrap();
    assert!(cart_result.is_none());

    // Cart exists, is not expired
    let cart = Order::find_or_create_cart(&user, conn).unwrap();
    let cart_result = Order::find_cart_for_user(user.id, conn);
    assert_eq!(cart_result.unwrap().unwrap(), cart);

    // Expired cart
    let one_minute_ago = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(1));
    diesel::update(&cart)
        .set(orders::expires_at.eq(one_minute_ago))
        .get_result::<Order>(conn)
        .unwrap();
    let cart_result = Order::find_cart_for_user(user.id, conn).unwrap();
    assert!(cart_result.is_none());
}

#[test]
fn has_items() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();

    // Without items
    assert!(!cart.has_items(connection).unwrap());

    // With items
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    assert!(cart.has_items(connection).unwrap());
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let cart = Order::find_or_create_cart(&user, connection).unwrap();
    let cart_result = Order::find_cart_for_user(user.id, connection).unwrap();
    assert_eq!(cart_result.unwrap(), cart);

    cart.destroy(connection).unwrap();
    let cart_result = Order::find_cart_for_user(user.id, connection).unwrap();
    assert!(cart_result.is_none());
}

#[test]
fn calculate_cart_total() {
    let project = TestProject::new();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let conn = project.get_connection();
    let mut cart = Order::find_or_create_cart(&user, conn).unwrap();
    let ticket = &event.ticket_types(true, None, conn).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        conn,
    )
    .unwrap();

    let total = cart.calculate_total(conn).unwrap();
    assert_eq!(total, 1700);

    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 30,
            redemption_code: None,
        }],
        false,
        false,
        conn,
    )
    .unwrap();
    let total = cart.calculate_total(conn).unwrap();
    assert_eq!(total, 5100);
}

#[test]
fn add_external_payment() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let user = project.create_user().finish();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let artist = project.create_artist().finish();
    project
        .create_event_artist()
        .with_event(&event)
        .with_artist(&artist)
        .finish();
    artist
        .set_genres(
            &vec!["emo".to_string(), "hard-rock".to_string()],
            None,
            conn,
        )
        .unwrap();
    event.update_genres(None, conn).unwrap();
    assert!(user.genres(conn).unwrap().is_empty());

    let mut cart = Order::find_or_create_cart(&user, conn).unwrap();
    let ticket = &event.ticket_types(true, None, conn).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        conn,
    )
    .unwrap();
    assert_eq!(cart.calculate_total(conn).unwrap(), 2000);
    assert!(cart.paid_at.is_none());

    // Partially paid
    cart.add_external_payment(
        Some("test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        1500,
        conn,
    )
    .unwrap();
    assert_eq!(cart.status, OrderStatus::Draft);
    assert!(cart.paid_at.is_none());
    assert!(user.genres(conn).unwrap().is_empty());

    // Fully paid
    cart.add_external_payment(
        Some("test2".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        500,
        conn,
    )
    .unwrap();
    assert_eq!(cart.status, OrderStatus::Paid);
    assert_eq!(
        Some(ExternalPaymentType::CreditCard),
        cart.external_payment_type
    );
    assert!(cart.paid_at.is_some());

    let domain_events = DomainEvent::find(
        Tables::Orders,
        Some(cart.id),
        Some(DomainEventTypes::OrderCompleted),
        conn,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
    assert_eq!(
        user.genres(conn).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string()]
    );
}

#[test]
fn add_external_payment_for_expired_code() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let conn = project.get_connection();
    let mut cart = Order::find_or_create_cart(&user, conn).unwrap();
    let ticket_type = &event.ticket_types(true, None, conn).unwrap()[0];
    let code = project
        .create_code()
        .with_code_type(CodeTypes::Access)
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .finish();
    assert_eq!(code.discount_in_cents, Some(100));
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 10,
            redemption_code: Some(code.redemption_code.clone()),
        }],
        false,
        false,
        conn,
    )
    .unwrap();
    assert_eq!(cart.calculate_total(conn).unwrap(), 1000);
    assert!(cart.paid_at.is_none());

    // Update code so it's expired
    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(3));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));
    code.update(
        UpdateCodeAttributes {
            start_date: Some(start_date),
            end_date: Some(end_date),
            ..Default::default()
        },
        None,
        conn,
    )
    .unwrap();

    // Attempting to pay triggers error
    let result = cart.add_external_payment(
        Some("test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        1000,
        conn,
    );

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("code_id"));
                assert_eq!(errors["code_id"].len(), 1);
                assert_eq!(errors["code_id"][0].code, "invalid");
                assert_eq!(
                    &errors["code_id"][0].message.clone().unwrap().into_owned(),
                    "Code not valid for current datetime"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn add_free_payment() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    // Free order
    let mut order = project
        .create_order()
        .with_free_items()
        .for_user(&user)
        .for_event(&event)
        .finish();
    assert_eq!(0, order.calculate_total(connection).unwrap());
    assert_eq!(order.status, OrderStatus::Draft);
    assert!(order.payments(connection).unwrap().is_empty());
    order.add_free_payment(false, user.id, connection).unwrap();
    assert_eq!(order.status, OrderStatus::Paid);

    let payments = order.payments(connection).unwrap();
    assert_eq!(1, payments.len());
    let payment = &payments[0];
    assert_eq!(payment.payment_method, PaymentMethods::Free);
    assert_eq!(payment.provider, PaymentProviders::Free);
    order = Order::find(order.id, connection).unwrap();
    assert_eq!(order.external_payment_type, None);

    // External free order
    let mut order = project
        .create_order()
        .with_free_items()
        .for_user(&user)
        .for_event(&event)
        .finish();
    assert_eq!(0, order.calculate_total(connection).unwrap());
    assert_eq!(order.status, OrderStatus::Draft);
    assert!(order.payments(connection).unwrap().is_empty());
    order.add_free_payment(true, user.id, connection).unwrap();
    assert_eq!(order.status, OrderStatus::Paid);

    let payments = order.payments(connection).unwrap();
    assert_eq!(1, payments.len());
    let payment = &payments[0];
    assert_eq!(payment.payment_method, PaymentMethods::Free);
    assert_eq!(payment.provider, PaymentProviders::External);
    order = Order::find(order.id, connection).unwrap();
    assert_eq!(
        order.external_payment_type,
        Some(ExternalPaymentType::Voucher)
    );
}

#[test]
fn find_for_user_for_display() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let mut order1 = project.create_order().for_user(&user).finish();
    order1
        .add_external_payment(
            Some("test".to_string()),
            ExternalPaymentType::CreditCard,
            user.id,
            2000,
            project.get_connection(),
        )
        .unwrap();
    let mut order2 = project.create_order().for_user(&user).finish();
    order2
        .add_external_payment(
            Some("test".to_string()),
            ExternalPaymentType::CreditCard,
            user.id,
            500,
            project.get_connection(),
        )
        .unwrap();

    assert_eq!(order1.status, OrderStatus::Paid);
    assert_eq!(order2.status, OrderStatus::Draft);

    let display_orders =
        Order::find_for_user_for_display(user.id, project.get_connection()).unwrap();
    let ids: Vec<Uuid> = display_orders.iter().map(|o| o.id).collect();
    //The order of the ids is not certain so this test fails from time to time.
    //It is ordered by updated_at which is the same for the two orders

    assert_eq!(order1.id, ids[0]);

    // User list so items shown in full
    assert!(!&display_orders[0].order_contains_other_tickets);
}

#[test]
fn for_display_seconds_until_expiry() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let order = project.create_order().finish();

    // 1 minute from now expires
    let one_minute_from_now = NaiveDateTime::from(Utc::now().naive_utc() + Duration::minutes(1));
    let order = diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set(orders::expires_at.eq(one_minute_from_now))
        .get_result::<Order>(connection)
        .unwrap();
    let display_order = order.for_display(None, order.user_id, connection).unwrap();
    // Check both 59 and 60 for the purposes of the test to avoid timing errors
    assert!(vec![59, 60].contains(&display_order.seconds_until_expiry.unwrap()));

    // No organization filtering
    assert!(!display_order.order_contains_other_tickets);

    // 1 minute ago expires
    let one_minute_ago = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(1));
    let order = diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set(orders::expires_at.eq(one_minute_ago))
        .get_result::<Order>(connection)
        .unwrap();
    let display_order = order.for_display(None, order.user_id, connection).unwrap();
    assert_eq!(Some(0), display_order.seconds_until_expiry);

    // Draft order won't expire until items added
    let user = project.create_user().finish();
    let order = Order::find_or_create_cart(&user, connection).unwrap();
    let display_order = order.for_display(None, user.id, connection).unwrap();
    assert_eq!(None, display_order.seconds_until_expiry);
}

#[test]
fn for_display() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_fees()
        .with_event_fee()
        .finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type_count(1)
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(10)
        .with_ticket_type_id(ticket_type.id)
        .finish();
    let mut order = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .quantity(2)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .is_paid()
        .finish();
    let items = order.items(connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type.id))
        .unwrap();
    let event_fee_item = items
        .iter()
        .find(|i| i.item_type == OrderItemTypes::EventFees)
        .unwrap();
    let tickets = TicketInstance::find_for_order_item(order_item.id, connection).unwrap();
    let display_order = order.for_display(None, user.id, connection).unwrap();
    let order_total = order.calculate_total(connection).unwrap();
    assert_eq!(order_total, display_order.total_in_cents);
    assert_eq!(0, display_order.total_refunded_in_cents);

    // Refund one ticket
    let order_item = OrderItem::find(tickets[0].order_item_id.unwrap(), connection).unwrap();
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(tickets[0].id),
    }];
    let (_refund, refund_ticket1_amount) = order
        .refund(&refund_items, user.id, None, connection)
        .unwrap();
    let display_order = order.for_display(None, user.id, connection).unwrap();
    assert_eq!(order_total, display_order.total_in_cents);
    assert_ne!(
        display_order.total_refunded_in_cents,
        display_order.total_in_cents
    );
    assert_eq!(refund_ticket1_amount, display_order.total_refunded_in_cents);

    // Refund remaining ticket
    let order_item = OrderItem::find(tickets[0].order_item_id.unwrap(), connection).unwrap();
    let refund_items = vec![RefundItemRequest {
        order_item_id: order_item.id,
        ticket_instance_id: Some(tickets[1].id),
    }];
    let (_refund, refund_ticket2_amount) = order
        .refund(&refund_items, user.id, None, connection)
        .unwrap();
    let display_order = order.for_display(None, user.id, connection).unwrap();
    assert_eq!(order_total, display_order.total_in_cents);
    assert_ne!(
        display_order.total_refunded_in_cents,
        display_order.total_in_cents
    );
    assert_eq!(
        refund_ticket1_amount + refund_ticket2_amount,
        display_order.total_refunded_in_cents
    );

    // Refund remaining event fee item
    let refund_items = vec![RefundItemRequest {
        order_item_id: event_fee_item.id,
        ticket_instance_id: None,
    }];
    let (_refund, event_fee_refund_amount) = order
        .refund(&refund_items, user.id, None, connection)
        .unwrap();
    let display_order = order.for_display(None, user.id, connection).unwrap();
    assert_eq!(order_total, display_order.total_in_cents);
    assert_eq!(
        refund_ticket1_amount + refund_ticket2_amount + event_fee_refund_amount,
        display_order.total_refunded_in_cents
    );
    assert_eq!(
        display_order.total_refunded_in_cents,
        display_order.total_in_cents
    );
}

#[test]
fn for_display_with_invalid_items() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_type_count(5)
        .with_ticket_pricing()
        .finish();

    let ticket_types = event.ticket_types(true, None, connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let ticket_type3 = &ticket_types[2];
    let ticket_type4 = &ticket_types[3];
    let ticket_type5 = &ticket_types[4];
    let hold = project
        .create_hold()
        .with_hold_type(HoldTypes::Discount)
        .with_quantity(1)
        .with_ticket_type_id(ticket_type5.id)
        .finish();
    let code = project
        .create_code()
        .with_event(&event)
        .with_code_type(CodeTypes::Discount)
        .for_ticket_type(&ticket_type5)
        .with_discount_in_cents(Some(10))
        .finish();

    // Normal order for ticket type
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    // Order item with no associated ticket_instances record
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type2.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type2.id))
        .unwrap();
    let none_uuid: Option<Uuid> = None;
    let ticket_instance = TicketInstance::find_for_order_item(order_item.id, connection)
        .unwrap()
        .remove(0);
    diesel::update(ticket_instances::table.filter(ticket_instances::id.eq(ticket_instance.id)))
        .set(ticket_instances::order_item_id.eq(none_uuid))
        .execute(connection)
        .unwrap();

    let display_order = cart.for_display(None, user.id, connection).unwrap();
    assert_eq!(
        1,
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets
                && i.cart_item_status == Some(CartItemStatus::TicketNotReserved))
            .count()
    );

    // Order item with past reserved until date
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type3.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type3.id))
        .unwrap();
    let one_minute_ago = NaiveDateTime::from(Utc::now().naive_utc() - Duration::minutes(1));
    diesel::update(
        ticket_instances::table.filter(ticket_instances::order_item_id.eq(order_item.id)),
    )
    .set((ticket_instances::reserved_until.eq(one_minute_ago),))
    .execute(connection)
    .unwrap();

    let display_order = cart.for_display(None, user.id, connection).unwrap();
    assert_eq!(
        2,
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets
                && i.cart_item_status == Some(CartItemStatus::TicketNotReserved))
            .count()
    );

    // Order item with nullified ticket status
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type4.id,
            quantity: 2,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    let items = cart.items(connection).unwrap();
    let order_item = items
        .iter()
        .find(|i| i.ticket_type_id == Some(ticket_type4.id))
        .unwrap();

    let ticket = TicketInstance::find_for_order_item(order_item.id, connection)
        .unwrap()
        .remove(0);
    diesel::update(ticket_instances::table.filter(ticket_instances::id.eq(ticket.id)))
        .set((ticket_instances::status.eq(TicketInstanceStatus::Nullified),))
        .execute(connection)
        .unwrap();

    // Code with end date in the past
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type5.id,
            quantity: 1,
            redemption_code: Some(code.redemption_code.clone()),
        }],
        false,
        false,
        connection,
    )
    .unwrap();
    code.update(
        UpdateCodeAttributes {
            end_date: Some(one_minute_ago),
            ..Default::default()
        },
        None,
        connection,
    )
    .unwrap();

    // Hold with end at date in the past
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type5.id,
            quantity: 1,
            redemption_code: hold.redemption_code.clone(),
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    hold.update(
        UpdateHoldAttributes {
            end_at: Some(Some(one_minute_ago)),
            ..Default::default()
        },
        connection,
    )
    .unwrap();

    let display_order = cart.for_display(None, user.id, connection).unwrap();

    // Check against expected counts for status based on above
    assert_eq!(
        1,
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets
                && i.cart_item_status == Some(CartItemStatus::Valid))
            .count()
    );
    assert_eq!(
        2,
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets
                && i.cart_item_status == Some(CartItemStatus::TicketNotReserved))
            .count()
    );
    assert_eq!(
        1,
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets
                && i.cart_item_status == Some(CartItemStatus::TicketNullified))
            .count()
    );
    assert_eq!(
        1,
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets
                && i.cart_item_status == Some(CartItemStatus::CodeExpired))
            .count()
    );
    assert_eq!(
        1,
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets
                && i.cart_item_status == Some(CartItemStatus::HoldExpired))
            .count()
    );

    // Unpaid order includes valid_for_purchase
    let generated_json = json!(display_order).to_string();
    assert!(generated_json.contains("valid_for_purchase"));
    let deserialized_display_order: DisplayOrder = serde_json::from_str(&generated_json).unwrap();
    assert_eq!(Some(false), deserialized_display_order.valid_for_purchase);

    // Clear invalid items
    cart.clear_invalid_items(user.id, connection).unwrap();

    let display_order = cart.for_display(None, user.id, connection).unwrap();
    assert_eq!(
        1,
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets
                && i.cart_item_status == Some(CartItemStatus::Valid))
            .count()
    );
    assert_eq!(
        0,
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets
                && i.cart_item_status == Some(CartItemStatus::TicketNotReserved))
            .count()
    );
    assert_eq!(
        0,
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets
                && i.cart_item_status == Some(CartItemStatus::TicketNullified))
            .count()
    );
    assert_eq!(
        0,
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets
                && i.cart_item_status == Some(CartItemStatus::CodeExpired))
            .count()
    );
    assert_eq!(
        0,
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets
                && i.cart_item_status == Some(CartItemStatus::HoldExpired))
            .count()
    );
    let generated_json = json!(display_order).to_string();
    assert!(generated_json.contains("valid_for_purchase"));
    let deserialized_display_order: DisplayOrder = serde_json::from_str(&generated_json).unwrap();
    assert_eq!(Some(true), deserialized_display_order.valid_for_purchase);

    // Pay off cart
    let total = cart.calculate_total(connection).unwrap();
    cart.add_external_payment(
        Some("Test".to_string()),
        ExternalPaymentType::CreditCard,
        user.id,
        total,
        connection,
    )
    .unwrap();

    // Paid order does not include valid_for_purchase
    let display_order = cart.for_display(None, user.id, connection).unwrap();
    let generated_json = json!(display_order).to_string();
    assert!(!generated_json.contains("valid_for_purchase"));
    let deserialized_display_order: DisplayOrder = serde_json::from_str(&generated_json).unwrap();
    assert_eq!(None, deserialized_display_order.valid_for_purchase);
}

#[test]
fn validate_record() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];
    let ticket_type2 = &event2.ticket_types(true, None, connection).unwrap()[0];
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let result = cart.update_quantities(
        user.id,
        &[
            UpdateOrderItem {
                ticket_type_id: ticket_type.id,
                quantity: 1,
                redemption_code: None,
            },
            UpdateOrderItem {
                ticket_type_id: ticket_type2.id,
                quantity: 1,
                redemption_code: None,
            },
        ],
        false,
        false,
        connection,
    );
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("event_id"));
                assert_eq!(errors["event_id"].len(), 1);
                assert_eq!(errors["event_id"][0].code, "cart_event_limit_reached");
                assert_eq!(
                    &errors["event_id"][0].message.clone().unwrap().into_owned(),
                    "Cart limited to one event for purchasing"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn for_display_with_organization_id_and_event_id_filters() {
    let project = TestProject::new();
    let connection = project.get_connection();

    // Events with different organizations
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let organization = event.organization(connection).unwrap();
    let organization2 = event2.organization(connection).unwrap();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    let user = project.create_user().finish();
    let user2 = project.create_user().finish();

    OrganizationUser::create(event.organization_id, user2.id, vec![Roles::OrgMember])
        .commit(connection)
        .unwrap();
    OrganizationUser::create(event2.organization_id, user2.id, vec![Roles::OrgMember])
        .commit(connection)
        .unwrap();

    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket_type.id,
            quantity: 1,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    // No filtering
    let display_order = cart.for_display(None, user2.id, connection).unwrap();
    assert!(!display_order.order_contains_other_tickets);
    assert_eq!(display_order.items.len(), 2); // 1 tickets, 1 fees

    // With filtering
    let display_order = cart
        .for_display(Some(vec![event.organization_id]), user2.id, connection)
        .unwrap();
    assert!(!display_order.order_contains_other_tickets);
    assert_eq!(display_order.items.len(), 2); // 1 ticket, 1 fee
    let order_item: &DisplayOrderItem = display_order
        .items
        .iter()
        .find(|i| i.parent_id.is_none())
        .unwrap();
    assert_eq!(order_item.ticket_type_id, Some(ticket_type.id));

    // Filtered by event_id
    organization
        .add_user(user2.id, vec![Roles::Promoter], vec![event.id], connection)
        .unwrap();
    OrganizationUser::create(event2.organization_id, user2.id, vec![Roles::Promoter])
        .commit(connection)
        .unwrap();
    let display_order = cart
        .for_display(
            Some(vec![event.organization_id, event2.organization_id]),
            user2.id,
            connection,
        )
        .unwrap();
    assert!(!display_order.order_contains_other_tickets);
    assert_eq!(display_order.items.len(), 2); // 1 tickets, 1 fees

    OrganizationUser::create(event.organization_id, user2.id, vec![Roles::Promoter])
        .commit(connection)
        .unwrap();

    organization2
        .add_user(user2.id, vec![Roles::Promoter], vec![event2.id], connection)
        .unwrap();
    let display_order = cart
        .for_display(
            Some(vec![event.organization_id, event2.organization_id]),
            user2.id,
            connection,
        )
        .unwrap();
    assert!(!display_order.order_contains_other_tickets);
    assert_eq!(display_order.items.len(), 2); // 1 tickets, 1 fees
}

#[test]
fn adding_event_fees() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project
        .create_organization()
        .with_fees()
        .with_event_fee()
        .finish();
    let event1 = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let organization2 = project.create_organization().finish();
    let event2 = project
        .create_event()
        .with_organization(&organization2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let organization3 = project
        .create_organization()
        .with_fees()
        .with_event_fee()
        .finish();
    let event3 = project
        .create_event()
        .with_organization(&organization3)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    let mut cart = Order::find_or_create_cart(&user, connection).unwrap();
    let ticket1 = &event1.ticket_types(true, None, connection).unwrap()[0];
    let ticket2 = &event2.ticket_types(true, None, connection).unwrap()[0];
    let ticket3 = &event3.ticket_types(true, None, connection).unwrap()[0];
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket1.id,
            quantity: 10,
            redemption_code: None,
        }],
        false,
        false,
        connection,
    )
    .unwrap();

    let order_items = OrderItem::find_for_order(cart.id, connection).unwrap();

    let mut event_fees_count = 0;
    for o in &order_items {
        if o.item_type == OrderItemTypes::EventFees {
            event_fees_count += 1;
        }
    }
    assert_eq!(event_fees_count, 1);

    //Add tickets with null event fee and null organization event_fee
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket2.id,
            quantity: 5,
            redemption_code: None,
        }],
        false,
        true,
        connection,
    )
    .unwrap();
    let order_items = OrderItem::find_for_order(cart.id, connection).unwrap();

    let mut event_fees_count = 0;
    for o in &order_items {
        if o.item_type == OrderItemTypes::EventFees {
            event_fees_count += 1;
        }
    }
    assert_eq!(event_fees_count, 0);

    //Add tickets with null event fee and but default organization event_fee
    cart.update_quantities(
        user.id,
        &[UpdateOrderItem {
            ticket_type_id: ticket3.id,
            quantity: 5,
            redemption_code: None,
        }],
        false,
        true,
        connection,
    )
    .unwrap();
    let order_items = OrderItem::find_for_order(cart.id, connection).unwrap();

    let mut event_fees_count = 0;
    for o in &order_items {
        if o.item_type == OrderItemTypes::EventFees {
            event_fees_count += 1;
        }
    }
    assert_eq!(event_fees_count, 1);
}
#[test]
pub fn search_by_general_query() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let _order1 = project.create_order().is_paid().finish();
    let event = project.create_event().with_tickets().finish();
    let order2 = project.create_order().for_event(&event).is_paid().finish();

    let actual = Order::search(
        None,
        None,
        Some(&order2.id.to_string()[4..8]),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order2.id, actual.0[0].id);
}

#[test]
pub fn search_by_event_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let _order1 = project.create_order().is_paid().finish();
    let event = project.create_event().with_tickets().finish();
    let order2 = project.create_order().for_event(&event).is_paid().finish();

    let actual = Order::search(
        Some(event.id),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order2.id, actual.0[0].id);
}

#[test]
pub fn search_by_organization_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let _order1 = project.create_order().is_paid().finish();
    let org1 = project
        .create_organization()
        .with_event_fee()
        .with_fees()
        .finish();
    let event = project
        .create_event()
        .with_organization(&org1)
        .with_tickets()
        .finish();
    let order2 = project.create_order().for_event(&event).is_paid().finish();

    let org2 = project
        .create_organization()
        .with_event_fee()
        .with_fees()
        .finish();
    let event2 = project
        .create_event()
        .with_organization(&org2)
        .with_tickets()
        .finish();
    let _order3 = project.create_order().for_event(&event2).is_paid().finish();

    let actual = Order::search(
        None,
        Some(org1.id),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order2.id, actual.0[0].id);
}

#[test]
pub fn search_by_partial_order_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let _order1 = project.create_order().is_paid().finish();
    let event = project.create_event().with_tickets().finish();
    let order2 = project.create_order().for_event(&event).is_paid().finish();

    let actual = Order::search(
        None,
        None,
        None,
        Some(&order2.id.to_string()[4..8]),
        None,
        None,
        None,
        None,
        None,
        None,
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order2.id, actual.0[0].id);
}

#[test]
pub fn search_by_partial_ticket_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let _order1 = project.create_order().is_paid().finish();
    let event = project.create_event().with_tickets().finish();
    let order2 = project.create_order().for_event(&event).is_paid().finish();

    let ticket = &order2
        .tickets(
            event.ticket_types(false, None, connection).unwrap()[0].id,
            connection,
        )
        .unwrap()[0];
    let actual = Order::search(
        None,
        None,
        None,
        None,
        Some(&ticket.id.to_string()[4..8]),
        None,
        None,
        None,
        None,
        None,
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order2.id, actual.0[0].id);
}

#[test]
pub fn search_by_email() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let _order1 = project.create_order().is_paid().finish();
    let event = project.create_event().with_tickets().finish();
    let order2 = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .finish();

    let actual = Order::search(
        None,
        None,
        None,
        None,
        None,
        Some(&user.email.unwrap()[2..6]),
        None,
        None,
        None,
        None,
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &Default::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order2.id, actual.0[0].id);
}

#[test]
pub fn search_by_email_on_behalf_of() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let _order1 = project.create_order().is_paid().finish();
    let event = project.create_event().with_tickets().finish();
    let order2 = project
        .create_order()
        .for_event(&event)
        .on_behalf_of_user(&user)
        .is_paid()
        .finish();

    let actual = Order::search(
        None,
        None,
        None,
        None,
        None,
        Some(&user.email.unwrap()[2..6]),
        None,
        None,
        None,
        None,
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order2.id, actual.0[0].id);
}

#[test]
pub fn search_by_name() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().with_first_name("search").finish();
    let _order1 = project.create_order().is_paid().finish();
    let event = project.create_event().with_tickets().finish();
    let order2 = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .finish();

    let actual = Order::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(&user.first_name.as_ref().unwrap().to_string()),
        None,
        None,
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order2.id, actual.0[0].id);
}

#[test]
pub fn search_by_last_name_first() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project
        .create_user()
        .with_first_name("search")
        .with_last_name("lasT")
        .finish();
    let _order1 = project.create_order().is_paid().finish();
    let event = project.create_event().with_tickets().finish();
    let order2 = project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .finish();

    let actual = Order::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some("last search"),
        None,
        None,
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order2.id, actual.0[0].id);
}

#[test]
pub fn search_by_ticket_type() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let _order1 = project.create_order().is_paid().finish();
    let event = project.create_event().with_tickets().finish();
    let order2 = project.create_order().for_event(&event).is_paid().finish();

    let actual = Order::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(event.ticket_types(false, None, connection).unwrap()[0].id),
        None,
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order2.id, actual.0[0].id);
}

#[test]
pub fn search_by_promo_code_hold() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let _order1 = project.create_order().is_paid().finish();
    let event = project.create_event().with_tickets().finish();

    let hold = project
        .create_hold()
        .with_ticket_type_id(event.ticket_types(false, None, connection).unwrap()[0].id)
        .finish();
    let order2 = project
        .create_order()
        .for_event(&event)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .is_paid()
        .finish();

    let actual = Order::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(&hold.redemption_code.unwrap()[2..4]),
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order2.id, actual.0[0].id);
}

#[test]
pub fn search_by_promo_code_code() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let _order1 = project.create_order().is_paid().finish();
    let event = project.create_event().with_tickets().finish();
    let ticket_type = &event.ticket_types(false, None, connection).unwrap()[0];
    let code = project.create_code().for_ticket_type(ticket_type).finish();
    let order2 = project
        .create_order()
        .for_tickets(ticket_type.id)
        .with_redemption_code(code.redemption_code.clone())
        .is_paid()
        .finish();

    let actual = Order::search(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(&code.redemption_code[2..4]),
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order2.id, actual.0[0].id);
}

#[test]
pub fn search_by_all() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project
        .create_user()
        .with_first_name("search")
        .with_last_name("last_name")
        .finish();
    let _order1 = project.create_order().is_paid().finish();
    let event = project.create_event().with_tickets().finish();
    let ticket_type_id = event.ticket_types(false, None, connection).unwrap()[0].id;
    let hold = project
        .create_hold()
        .with_ticket_type_id(ticket_type_id)
        .finish();
    let order2 = project
        .create_order()
        .for_tickets(ticket_type_id)
        .with_redemption_code(hold.redemption_code.clone().unwrap())
        .for_user(&user)
        .is_paid()
        .finish();
    let ticket = &order2.tickets(ticket_type_id, connection).unwrap()[0];

    let actual = Order::search(
        Some(event.id),
        Some(event.organization_id),
        Some(&order2.id.to_string()[4..8]),
        Some(&order2.id.to_string()[4..8]),
        Some(&ticket.id.to_string()[4..8]),
        Some(&user.email.unwrap()[2..6]),
        None,
        Some(&format!(
            "{} {}",
            user.first_name.as_ref().unwrap_or(&"".to_string()),
            user.last_name.as_ref().unwrap_or(&"".to_string())
        )),
        Some(ticket_type_id),
        Some(&hold.redemption_code.unwrap()[2..4]),
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order2.id, actual.0[0].id);
}

#[test]
pub fn search_by_transferee_name() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let transferee = project
        .create_user()
        .with_first_name("Samantha")
        .with_last_name("Zorber")
        .with_email("missdaisy@yahoooooo.com".to_string())
        .finish();
    let order = project
        .create_order()
        .for_user(&user)
        .quantity(2)
        .is_paid()
        .finish();
    let _order2 = project
        .create_order()
        .for_user(&user2)
        .quantity(1)
        .is_paid()
        .finish();

    let tickets = TicketInstance::find_for_user(user.id, connection).unwrap();
    let ticket = &tickets[0];

    let mut transfer = Transfer::create(user.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    transfer.add_transfer_ticket(ticket.id, connection).unwrap();
    transfer.update_associated_orders(connection).unwrap();
    transfer
        .update(
            TransferEditableAttributes {
                destination_user_id: Some(transferee.id),
                ..Default::default()
            },
            connection,
        )
        .unwrap();

    let actual = Order::search(
        None,
        None,
        Some("Samantha"),
        None,
        None,
        None,
        None,
        Some(&user.first_name.as_ref().unwrap().to_string()),
        None,
        None,
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order.id, actual.0[0].id);

    let actual = Order::search(
        None,
        None,
        Some("Zorb"),
        None,
        None,
        None,
        None,
        Some(&user.first_name.as_ref().unwrap().to_string()),
        None,
        None,
        true,
        true,
        true,
        true,
        None,
        None,
        user.id,
        &PagingParameters::default(),
        connection,
    )
    .unwrap();
    assert_eq!(1, actual.1);
    assert_eq!(order.id, actual.0[0].id);
}

#[test]
pub fn additional_fee() {
    let project = TestProject::new();
    let connection = project.get_connection();

    let organization = project
        .create_organization()
        .with_event_fee()
        .with_max_additional_fee(9999999)
        .finish();

    let ticket_types = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_type()
        .with_additional_fees(10000)
        .finish()
        .ticket_types(false, None, connection)
        .unwrap();
    let ticket_type = ticket_types.get(0).unwrap();
    let order = project
        .create_order()
        .for_tickets(ticket_type.id)
        .is_paid()
        .finish();

    println!("{}", json!(order.items(connection)));

    let fees_item = order
        .find_item_by_type(ticket_type.id, OrderItemTypes::Tickets, connection)
        .unwrap()
        .find_fee_item(connection)
        .unwrap()
        .unwrap();

    assert_eq!(fees_item.unit_price_in_cents, 10050);
}
