use bigneon_db::dev::TestProject;
use bigneon_db::models::{TicketPricing, TicketPricingEditableAttributes, TicketType};
use bigneon_db::utils::errors::{self, *};
use chrono::NaiveDate;
use diesel::result::Error;
use diesel::Connection;

#[test]
fn create() {
    let project = TestProject::new();
    let event = project.create_event().with_tickets().finish();
    let ticket_type = &event.ticket_types(project.get_connection()).unwrap()[0];
    let sd1 = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed1 = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let sd2 = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ed2 = NaiveDate::from_ymd(2016, 7, 10).and_hms(4, 10, 11);

    let ticket_pricing =
        TicketPricing::create(ticket_type.id, "Early Bird".to_string(), sd1, ed1, 100)
            .commit(project.get_connection())
            .unwrap();

    let pricing2 =
        TicketPricing::create(ticket_type.id, "Wormless Bird".to_string(), sd2, ed2, 500)
            .commit(project.get_connection())
            .unwrap();

    let pricing = ticket_type
        .ticket_pricing(project.get_connection())
        .unwrap();
    assert_eq!(pricing, vec![ticket_pricing, pricing2]);
}

#[test]
fn ticket_pricing_no_overlapping_periods() {
    let project = TestProject::new();
    let event = project.create_event().with_tickets().finish();
    let ticket_type = &event.ticket_types(project.get_connection()).unwrap()[0];
    let start_date1 = NaiveDate::from_ymd(2016, 7, 6).and_hms(4, 10, 11);
    let end_date1 = NaiveDate::from_ymd(2016, 7, 10).and_hms(4, 10, 11);
    let start_date2 = NaiveDate::from_ymd(2016, 7, 7).and_hms(4, 10, 11);
    let end_date2 = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let start_date3 = NaiveDate::from_ymd(2016, 8, 7).and_hms(4, 10, 11);
    let end_date3 = NaiveDate::from_ymd(2016, 8, 9).and_hms(4, 10, 11);
    let ticket_pricing1 = TicketPricing::create(
        ticket_type.id,
        "Early Bird".to_string(),
        start_date1,
        end_date1,
        100,
    ).commit(project.get_connection())
    .unwrap();

    let ticket_pricing2 = TicketPricing::create(
        ticket_type.id,
        "Early Bird".to_string(),
        start_date2,
        end_date2,
        100,
    ).commit(project.get_connection())
    .unwrap();

    let ticket_pricing3 = TicketPricing::create(
        ticket_type.id,
        "Early Bird".to_string(),
        start_date3,
        end_date3,
        100,
    ).commit(project.get_connection())
    .unwrap();

    // ticket_pricing1 and ticket_pricing2 overlap
    assert!(
        TicketPricing::ticket_pricing_no_overlapping_periods(
            ticket_pricing1.id,
            ticket_type.id,
            start_date1,
            end_date1,
            project.get_connection()
        ).unwrap()
        .is_err()
    );
    assert!(
        TicketPricing::ticket_pricing_no_overlapping_periods(
            ticket_pricing2.id,
            ticket_type.id,
            start_date2,
            end_date2,
            project.get_connection()
        ).unwrap()
        .is_err()
    );

    // ticket_pricing3 does not overlap
    assert!(
        TicketPricing::ticket_pricing_no_overlapping_periods(
            ticket_pricing3.id,
            ticket_type.id,
            start_date3,
            end_date3,
            project.get_connection()
        ).unwrap()
        .is_ok()
    );
}

#[test]
fn validate_new() {
    let project = TestProject::new();
    let event = project.create_event().with_tickets().finish();
    let ticket_type = &event.ticket_types(project.get_connection()).unwrap()[0];
    let start_date1 = NaiveDate::from_ymd(2016, 7, 6).and_hms(4, 10, 11);
    let end_date1 = NaiveDate::from_ymd(2016, 7, 10).and_hms(4, 10, 11);
    let start_date2 = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let end_date2 = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    TicketPricing::create(
        ticket_type.id,
        "Early Bird".to_string(),
        start_date1,
        end_date1,
        100,
    ).commit(project.get_connection())
    .unwrap();

    let mut ticket_pricing = TicketPricing::create(
        ticket_type.id,
        "Early Bird".to_string(),
        start_date2,
        end_date2,
        100,
    );

    // Invalid start date validation
    project
        .get_connection()
        .transaction::<(), Error, _>(|| {
            let result = ticket_pricing.clone().commit(project.get_connection());
            assert!(result.is_err());
            let errors = result.unwrap_err();
            assert_eq!(
                errors.code,
                errors::get_error_message(&ErrorCode::InsertError).0
            );
            assert_eq!(
                errors.cause,
                Some("Could not create ticket pricing, new row for relation \"ticket_pricing\" violates check constraint \"ticket_pricing_start_date_prior_to_end_date\"".into())
            );
            Err(Error::RollbackTransaction)
        }).unwrap_err();

    // Period without start date validation
    ticket_pricing.start_date = end_date1;
    ticket_pricing.end_date = NaiveDate::from_ymd(2016, 7, 15).and_hms(4, 10, 11);
    let result = ticket_pricing.clone().commit(project.get_connection());
    assert!(result.is_ok());
}

#[test]
fn validate_existing() {
    let project = TestProject::new();
    let event = project.create_event().with_tickets().finish();
    let ticket_type = &event.ticket_types(project.get_connection()).unwrap()[0];
    let start_date1 = NaiveDate::from_ymd(2016, 7, 6).and_hms(4, 10, 11);
    let end_date1 = NaiveDate::from_ymd(2016, 7, 10).and_hms(4, 10, 11);
    let start_date2 = NaiveDate::from_ymd(2016, 7, 10).and_hms(4, 10, 11);
    let end_date2 = NaiveDate::from_ymd(2016, 7, 11).and_hms(4, 10, 11);
    TicketPricing::create(
        ticket_type.id,
        "Early Bird".to_string(),
        start_date1,
        end_date1,
        100,
    ).commit(project.get_connection())
    .unwrap();
    let ticket_pricing = TicketPricing::create(
        ticket_type.id,
        "Regular".to_string(),
        start_date2,
        end_date2,
        100,
    ).commit(project.get_connection())
    .unwrap();
    let mut ticket_pricing_parameters: TicketPricingEditableAttributes = Default::default();

    // Invalid start date validation
    project
        .get_connection()
        .transaction::<(), Error, _>(|| {
            ticket_pricing_parameters.start_date = Some(NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11));
            ticket_pricing_parameters.end_date = Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11));
            let result = ticket_pricing.update(ticket_pricing_parameters.clone(), project.get_connection());
            assert!(result.is_err());
            let errors = result.unwrap_err();
            assert_eq!(
                errors.code,
                errors::get_error_message(&ErrorCode::UpdateError).0
            );
            assert_eq!(
                errors.cause,
                Some("Could not update ticket_pricing, new row for relation \"ticket_pricing\" violates check constraint \"ticket_pricing_start_date_prior_to_end_date\"".into())
            );
            Err(Error::RollbackTransaction)
        }).unwrap_err();

    // Period without start date validation
    project
        .get_connection()
        .transaction::<(), Error, _>(|| {
            ticket_pricing_parameters.start_date = Some(end_date1);
            ticket_pricing_parameters.end_date =
                Some(NaiveDate::from_ymd(2016, 7, 15).and_hms(4, 10, 11));
            let result =
                ticket_pricing.update(ticket_pricing_parameters.clone(), project.get_connection());
            assert!(result.is_ok());
            Err(Error::RollbackTransaction)
        }).unwrap_err();
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_tickets().finish();
    let ticket_type = &event.ticket_types(connection).unwrap()[0];
    let start_date = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let end_date = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_pricing = TicketPricing::create(
        ticket_type.id,
        "Early Bird".to_string(),
        start_date,
        end_date,
        100,
    ).commit(connection)
    .unwrap();
    //Change editable parameter and submit ticket pricing update request
    let update_name = String::from("updated_event_name");
    let update_price_in_cents: i64 = 200;
    let update_start_date = NaiveDate::from_ymd(2018, 4, 23).and_hms(5, 14, 18);
    let update_end_date = NaiveDate::from_ymd(2018, 6, 1).and_hms(8, 5, 34);
    let update_parameters = TicketPricingEditableAttributes {
        name: Some(update_name.clone()),
        price_in_cents: Some(update_price_in_cents),
        start_date: Some(update_start_date),
        end_date: Some(update_end_date),
    };
    let updated_ticket_pricing = ticket_pricing
        .update(update_parameters, connection)
        .unwrap();
    assert_eq!(updated_ticket_pricing.id, ticket_pricing.id);
    assert_eq!(updated_ticket_pricing.name, update_name);
    assert_eq!(updated_ticket_pricing.price_in_cents, update_price_in_cents);
    assert_eq!(updated_ticket_pricing.start_date, update_start_date);
    assert_eq!(updated_ticket_pricing.end_date, update_end_date);
}

#[test]
fn remove() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_tickets().finish();
    let ticket_type = &event.ticket_types(connection).unwrap()[0];
    let start_date = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let end_date = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_pricing1 = TicketPricing::create(
        ticket_type.id,
        "Early Bird".to_string(),
        start_date,
        end_date,
        100,
    ).commit(connection)
    .unwrap();

    let start_date = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let end_date = NaiveDate::from_ymd(2016, 7, 10).and_hms(4, 10, 11);
    let ticket_pricing2 = TicketPricing::create(
        ticket_type.id,
        "Standard".to_string(),
        start_date,
        end_date,
        200,
    ).commit(connection)
    .unwrap();
    //Remove ticket pricing and check if it is still available
    ticket_pricing1.destroy(connection).unwrap();
    let ticket_pricings = ticket_type.ticket_pricing(connection).unwrap();
    let found_index1 = ticket_pricings
        .iter()
        .position(|ref r| r.id == ticket_pricing1.id);
    let found_index2 = ticket_pricings
        .iter()
        .position(|ref r| r.id == ticket_pricing2.id);
    assert!(found_index1.is_none());
    assert!(found_index2.is_some());
}

#[test]
fn find() {
    let project = TestProject::new();
    let event = project.create_event().with_tickets().finish();
    let ticket_type = &event.ticket_types(project.get_connection()).unwrap()[0];
    let sd1 = NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11);
    let ed1 = NaiveDate::from_ymd(2016, 7, 9).and_hms(4, 10, 11);
    let ticket_pricing =
        TicketPricing::create(ticket_type.id, "Early Bird".to_string(), sd1, ed1, 100)
            .commit(project.get_connection())
            .unwrap();
    let found_ticket_pricing =
        TicketPricing::find(ticket_pricing.id, project.get_connection()).unwrap();

    assert_eq!(found_ticket_pricing, ticket_pricing);
}

#[test]
fn get_current_ticket_pricing() {
    let db = TestProject::new();

    let organization = db
        .create_organization()
        .with_fee_schedule(&db.create_fee_schedule().finish())
        .finish();
    let event = db
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();

    let ticket_types = TicketType::find_by_event_id(event.id, db.get_connection()).unwrap();

    let ticket_pricing =
        TicketPricing::get_current_ticket_pricing(ticket_types[0].id, db.get_connection()).unwrap();

    assert_eq!(ticket_pricing.name, "Standard".to_string())
}

#[test]
fn get_current_ticket_capacity() {
    let db = TestProject::new();

    let organization = db
        .create_organization()
        .with_fee_schedule(&db.create_fee_schedule().finish())
        .finish();
    let event = db
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let ticket_types = TicketType::find_by_event_id(event.id, db.get_connection()).unwrap();
    assert_eq!(ticket_types.len(), 1);

    let ticket_capacity = ticket_types[0]
        .ticket_capacity(db.get_connection())
        .unwrap();
    assert_eq!(ticket_capacity, 100);
}
