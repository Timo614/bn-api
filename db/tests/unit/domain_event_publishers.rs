use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use bigneon_db::schema::domain_events;
use bigneon_db::utils::dates;
use diesel;
use diesel::prelude::*;
use diesel::RunQueryDsl;
use uuid::Uuid;

#[test]
fn find_all() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut domain_event_publisher = project.create_domain_event_publisher().finish();
    domain_event_publisher
        .update_last_domain_event_seq(1, connection)
        .unwrap();
    let domain_event_publisher = DomainEventPublisher::find(domain_event_publisher.id, connection).unwrap();
    let mut domain_event_publisher2 = project.create_domain_event_publisher().finish();
    domain_event_publisher2
        .update_last_domain_event_seq(2, connection)
        .unwrap();
    let domain_event_publisher2 = DomainEventPublisher::find(domain_event_publisher2.id, connection).unwrap();
    assert_eq!(
        DomainEventPublisher::find_all(connection).unwrap(),
        vec![domain_event_publisher.clone(), domain_event_publisher2.clone()]
    );

    domain_event_publisher2.delete(connection).unwrap();
    assert_eq!(
        DomainEventPublisher::find_all(connection).unwrap(),
        vec![domain_event_publisher]
    );
}

#[test]
fn update_last_domain_event_seq() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut domain_event_publisher = project.create_domain_event_publisher().finish();
    domain_event_publisher
        .update_last_domain_event_seq(1, connection)
        .unwrap();
    let mut domain_event_publisher = DomainEventPublisher::find(domain_event_publisher.id, connection).unwrap();
    assert_eq!(domain_event_publisher.last_domain_event_seq, Some(1));

    domain_event_publisher
        .update_last_domain_event_seq(2, connection)
        .unwrap();
    let domain_event_publisher = DomainEventPublisher::find(domain_event_publisher.id, connection).unwrap();
    assert_eq!(domain_event_publisher.last_domain_event_seq, Some(2));
}

#[test]
fn delete() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let domain_event_publisher = project.create_domain_event_publisher().finish();
    assert_eq!(DomainEventPublisher::find_all(connection).unwrap().len(), 1);

    domain_event_publisher.delete(connection).unwrap();
    assert_eq!(DomainEventPublisher::find_all(connection).unwrap().len(), 0);
}

#[test]
fn find_with_unpublished_domain_events() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let webhook = "http://localhost:7644/webhook".to_string();
    let user = project.create_user().finish();
    let organization = project.create_organization().finish();
    let organization2 = project.create_organization().finish();
    let organization_domain_event_publisher = DomainEventPublisher::create(
        Some(organization.id),
        vec![DomainEventTypes::OrderCompleted],
        webhook.clone(),
        true,
    )
    .commit(connection)
    .unwrap();
    let global_domain_event_publisher =
        DomainEventPublisher::create(None, vec![DomainEventTypes::OrderCompleted], webhook.clone(), true)
            .commit(connection)
            .unwrap();

    let event = project
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = project
        .create_event()
        .with_organization(&organization2)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    // Before orders are placed
    let publisher_data = DomainEventPublisher::find_with_unpublished_domain_events(100, connection).unwrap();
    assert_eq!(publisher_data.len(), 0);

    let order = project
        .create_order()
        .for_user(&user)
        .for_event(&event)
        .is_paid()
        .finish();
    let order2 = project
        .create_order()
        .for_user(&user)
        .for_event(&event2)
        .is_paid()
        .finish();
    let publisher_data = DomainEventPublisher::find_with_unpublished_domain_events(100, connection).unwrap();
    assert_eq!(publisher_data.len(), 2);
    let organization_domain_events = publisher_data.get(&organization_domain_event_publisher).unwrap();
    assert_eq!(organization_domain_events.len(), 1);
    assert_eq!(
        organization_domain_events
            .iter()
            .map(|d| d.main_id)
            .collect::<Vec<Option<Uuid>>>(),
        vec![Some(order.id)]
    );
    let order_domain_event = &organization_domain_events[0];

    let global_domain_events = publisher_data.get(&global_domain_event_publisher).unwrap();
    assert_eq!(global_domain_events.len(), 2);
    assert_eq!(
        global_domain_events
            .iter()
            .map(|d| d.main_id)
            .collect::<Vec<Option<Uuid>>>()
            .sort(),
        vec![Some(order.id), Some(order2.id)].sort()
    );

    // Disable importing of legacy events
    let parameters = DomainEventPublisherEditableAttributes {
        webhook_url: None,
        import_historic_events: Some(false),
    };
    let global_domain_event_publisher = global_domain_event_publisher.update(&parameters, connection).unwrap();

    // 2 are still shown as both occurred after the publisher was created
    let publisher_data = DomainEventPublisher::find_with_unpublished_domain_events(100, connection).unwrap();
    assert_eq!(publisher_data.len(), 2);
    let global_domain_events = publisher_data.get(&global_domain_event_publisher).unwrap();
    assert_eq!(global_domain_events.len(), 2);

    // With updated date moving order domain event prior to publisher creation
    let order_created_at = order_domain_event.created_at;
    diesel::update(domain_events::table.filter(domain_events::id.eq(order_domain_event.id)))
        .set(domain_events::created_at.eq(dates::now().add_minutes(-5).finish()))
        .execute(connection)
        .unwrap();
    let publisher_data = DomainEventPublisher::find_with_unpublished_domain_events(100, connection).unwrap();
    assert_eq!(publisher_data.len(), 2);
    let global_domain_events = publisher_data.get(&global_domain_event_publisher).unwrap();
    assert_eq!(global_domain_events.len(), 1);
    assert_eq!(
        global_domain_events
            .iter()
            .map(|d| d.main_id)
            .collect::<Vec<Option<Uuid>>>(),
        vec![Some(order2.id)]
    );

    // Organizational one still lists domain event as it includes records from all time
    let organization_domain_events = publisher_data.get(&organization_domain_event_publisher).unwrap();
    assert_eq!(organization_domain_events.len(), 1);
    assert_eq!(
        organization_domain_events
            .iter()
            .map(|d| d.main_id)
            .collect::<Vec<Option<Uuid>>>(),
        vec![Some(order.id)]
    );

    // Reset date to include order 1 in result set for global_domain_events
    diesel::update(domain_events::table.filter(domain_events::id.eq(order_domain_event.id)))
        .set(domain_events::created_at.eq(order_created_at))
        .execute(connection)
        .unwrap();
    let publisher_data = DomainEventPublisher::find_with_unpublished_domain_events(100, connection).unwrap();
    assert_eq!(publisher_data.len(), 2);
    let global_domain_events = publisher_data.get(&global_domain_event_publisher).unwrap();
    assert_eq!(global_domain_events.len(), 2);
    let organization_domain_events = publisher_data.get(&organization_domain_event_publisher).unwrap();
    assert_eq!(organization_domain_events.len(), 1);

    // Publish event only for global publisher
    global_domain_event_publisher
        .publish(&order_domain_event, &"".to_string(), connection)
        .unwrap();
    let publisher_data = DomainEventPublisher::find_with_unpublished_domain_events(100, connection).unwrap();
    assert_eq!(publisher_data.len(), 2);
    let global_domain_events = publisher_data.get(&global_domain_event_publisher).unwrap();
    assert_eq!(global_domain_events.len(), 1);
    assert_eq!(
        global_domain_events
            .iter()
            .map(|d| d.main_id)
            .collect::<Vec<Option<Uuid>>>(),
        vec![Some(order2.id)]
    );

    // Organizational one still lists domain event as it hasn't been published for that publisher yet
    let organization_domain_events = publisher_data.get(&organization_domain_event_publisher).unwrap();
    assert_eq!(organization_domain_events.len(), 1);
    assert_eq!(
        organization_domain_events
            .iter()
            .map(|d| d.main_id)
            .collect::<Vec<Option<Uuid>>>(),
        vec![Some(order.id)]
    );

    // Publish for other publisher
    organization_domain_event_publisher
        .publish(&order_domain_event, &"".to_string(), connection)
        .unwrap();
    let publisher_data = DomainEventPublisher::find_with_unpublished_domain_events(100, connection).unwrap();
    assert_eq!(publisher_data.len(), 1);
    let global_domain_events = publisher_data.get(&global_domain_event_publisher).unwrap();
    assert_eq!(global_domain_events.len(), 1);
    assert_eq!(
        global_domain_events
            .iter()
            .map(|d| d.main_id)
            .collect::<Vec<Option<Uuid>>>(),
        vec![Some(order2.id)]
    );
}

#[test]
fn publish() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .finish();
    let domain_event_publisher = DomainEventPublisher::create(
        None,
        vec![DomainEventTypes::OrderCompleted],
        "http://localhost:7644/webhook".to_string(),
        true,
    )
    .commit(connection)
    .unwrap();

    let publisher_data = DomainEventPublisher::find_with_unpublished_domain_events(100, connection).unwrap();
    assert_eq!(publisher_data.len(), 1);
    let domain_event = &publisher_data.get(&domain_event_publisher).unwrap()[0];
    domain_event_publisher
        .publish(&domain_event, &"".to_string(), connection)
        .unwrap();
}

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let webhook = "http://localhost:7644/webhook".to_string();
    let organization = project.create_organization().finish();
    let domain_event_publisher = DomainEventPublisher::create(
        Some(organization.id),
        vec![DomainEventTypes::TransferTicketStarted],
        webhook.clone(),
        true,
    )
    .commit(connection)
    .unwrap();

    assert_eq!(domain_event_publisher.organization_id, Some(organization.id));
    assert_eq!(
        domain_event_publisher.event_types,
        vec![DomainEventTypes::TransferTicketStarted]
    );
    assert_eq!(domain_event_publisher.webhook_url, webhook);
    assert_eq!(domain_event_publisher.import_historic_events, true);
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let domain_event_publisher = DomainEventPublisher::create(
        Some(organization.id),
        vec![DomainEventTypes::TransferTicketStarted],
        "http://localhost:7644/webhook".to_string(),
        true,
    )
    .commit(connection)
    .unwrap();
    let found_publisher = DomainEventPublisher::find(domain_event_publisher.id, connection).unwrap();
    assert_eq!(found_publisher, domain_event_publisher);
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let domain_event_publisher = DomainEventPublisher::create(
        Some(organization.id),
        vec![DomainEventTypes::TransferTicketStarted],
        "http://localhost:7644/webhook".to_string(),
        true,
    )
    .commit(connection)
    .unwrap();

    let new_webhook_url = "http://localhost:7699/webhook".to_string();
    let parameters = DomainEventPublisherEditableAttributes {
        webhook_url: Some(new_webhook_url.clone()),
        import_historic_events: Some(false),
    };
    let domain_event_publisher = domain_event_publisher.update(&parameters, connection).unwrap();

    assert_eq!(domain_event_publisher.webhook_url, new_webhook_url);
    assert_eq!(domain_event_publisher.import_historic_events, false);
}

#[test]
fn acquire_lock() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let mut domain_event_publisher = DomainEventPublisher::create(
        Some(organization.id),
        vec![DomainEventTypes::TransferTicketStarted],
        "http://localhost:7644/webhook".to_string(),
        true,
    )
    .commit(connection)
    .unwrap();

    assert!(domain_event_publisher.acquire_lock(60, connection).is_ok());

    // pretend this is from another thread
    let mut domain_event_publisher_alias = DomainEventPublisher::find(domain_event_publisher.id, connection).unwrap();

    assert!(domain_event_publisher_alias.acquire_lock(60, connection).is_err());
}

#[test]
fn release_lock() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let mut domain_event_publisher = DomainEventPublisher::create(
        Some(organization.id),
        vec![DomainEventTypes::TransferTicketStarted],
        "http://localhost:7644/webhook".to_string(),
        true,
    )
    .commit(connection)
    .unwrap();

    // pretend this is from another thread
    let mut domain_event_publisher_alias = DomainEventPublisher::find(domain_event_publisher.id, connection).unwrap();

    assert!(domain_event_publisher.acquire_lock(60, connection).is_ok());

    assert!(domain_event_publisher_alias.acquire_lock(60, connection).is_err());

    assert!(domain_event_publisher.release_lock(connection).is_ok());

    assert!(domain_event_publisher_alias.acquire_lock(60, connection).is_ok());
}

#[test]
fn renew_lock() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let mut domain_event_publisher = DomainEventPublisher::create(
        Some(organization.id),
        vec![DomainEventTypes::TransferTicketStarted],
        "http://localhost:7644/webhook".to_string(),
        true,
    )
    .commit(connection)
    .unwrap();

    assert!(domain_event_publisher.acquire_lock(60, connection).is_ok());

    assert!(domain_event_publisher.renew_lock(60, connection).is_ok());
}
