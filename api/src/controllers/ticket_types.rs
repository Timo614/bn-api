use actix_web::{HttpResponse, Json, Path, Query, State};
use auth::user::User;
use bigneon_db::models::*;
use chrono::prelude::*;
use db::Connection;
use errors::*;
use helpers::application;
use models::{
    AdminDisplayTicketType, EventTicketPathParameters, Paging, PagingParameters, PathParameters,
    Payload,
};
use server::AppState;
use tari_client::MessagePayloadCreateAsset as TariNewAsset;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateTicketPricingRequest {
    pub name: String,
    pub price_in_cents: i64,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
}

#[derive(Deserialize)]
pub struct CreateTicketTypeRequest {
    pub name: String,
    pub capacity: u32,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub ticket_pricing: Vec<CreateTicketPricingRequest>,
    pub increment: Option<i32>,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateTicketPricingRequest {
    pub id: Option<Uuid>,
    pub name: Option<String>,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub price_in_cents: Option<i64>,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateTicketTypeRequest {
    pub name: Option<String>,
    pub capacity: Option<u32>,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub ticket_pricing: Option<Vec<UpdateTicketPricingRequest>>,
    pub increment: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct DisplayCreatedTicket {
    pub id: Uuid,
}

pub fn create(
    (connection, path, data, user, state): (
        Connection,
        Path<PathParameters>,
        Json<CreateTicketTypeRequest>,
        User,
        State<AppState>,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(path.id, connection)?;
    if !user.has_scope(
        Scopes::EventWrite,
        Some(&event.organization(connection)?),
        connection,
    )? {
        return application::unauthorized();
    }
    //Retrieve default wallet
    let org_wallet = Wallet::find_default_for_organization(event.organization_id, connection)?;

    //Add new ticket type
    let ticket_type = event.add_ticket_type(
        data.name.clone(),
        data.capacity,
        data.start_date,
        data.end_date,
        org_wallet.id,
        data.increment,
        connection,
    )?;
    //Add each ticket pricing entry for newly created ticket type
    for current_pricing_entry in &data.ticket_pricing {
        let _pricing_result = ticket_type.add_ticket_pricing(
            current_pricing_entry.name.clone(),
            current_pricing_entry.start_date,
            current_pricing_entry.end_date,
            current_pricing_entry.price_in_cents,
            connection,
        )?;
    }

    ticket_type.validate_record(connection)?;

    // TODO: move this to an async processor...

    let tari_asset_id = state.config.tari_client.create_asset(
        &org_wallet.secret_key,
        &org_wallet.public_key,
        TariNewAsset {
            name: format!("{}.{}", event.id, data.name),
            total_supply: data.capacity as u64,
            authorised_signers: Vec::new(),
            rule_flags: 0,
            rule_metadata: "".to_string(),
            expiry_date: data.end_date.timestamp(),
        },
    )?;
    let asset = Asset::find_by_ticket_type(&ticket_type.id, connection)?;
    let _asset = asset.update_blockchain_id(tari_asset_id, connection)?;
    Ok(HttpResponse::Created().json(DisplayCreatedTicket { id: ticket_type.id }))
}

#[derive(Deserialize, Serialize)]
pub struct TicketTypesResponse {
    pub ticket_types: Vec<AdminDisplayTicketType>,
}

pub fn index(
    (connection, path, query_parameters, user): (
        Connection,
        Path<PathParameters>,
        Query<PagingParameters>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(path.id, connection)?;
    let organization = &event.organization(connection)?;
    if !user.has_scope(Scopes::EventWrite, Some(organization), connection)? {
        return application::unauthorized();
    }

    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection)?;
    //TODO refactor using paging params
    let ticket_types = TicketType::find_by_event_id(path.id, connection)?;
    let query_parameters = Paging::new(&query_parameters.into_inner());
    let mut payload = Payload {
        data: Vec::new(),
        paging: Paging::clone_with_new_total(&query_parameters, 0 as u64),
    };
    for t in ticket_types {
        payload.data.push(AdminDisplayTicketType::from_ticket_type(
            &t,
            &fee_schedule,
            connection,
        )?);
    }
    payload.paging.limit = payload.data.len() as u64;
    payload.paging.total = payload.data.len() as u64;
    Ok(HttpResponse::Ok().json(&payload))
}

pub fn update(
    (connection, path, data, user, state): (
        Connection,
        Path<EventTicketPathParameters>,
        Json<UpdateTicketTypeRequest>,
        User,
        State<AppState>,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(path.event_id, connection)?;
    if !user.has_scope(
        Scopes::EventWrite,
        Some(&event.organization(connection)?),
        connection,
    )? {
        return application::unauthorized();
    }

    let ticket_type = TicketType::find(path.ticket_type_id, connection)?;
    if data.capacity.is_some() {
        let valid_ticket_count = ticket_type.valid_ticket_count(connection)?;
        let requested_capacity = data.capacity.unwrap();
        if valid_ticket_count < requested_capacity {
            let starting_tari_id = ticket_type.ticket_count(connection)?;
            let additional_ticket_count = requested_capacity - valid_ticket_count;
            let asset = Asset::find_by_ticket_type(&ticket_type.id, connection)?;
            let org_wallet =
                Wallet::find_default_for_organization(event.organization_id, connection)?;
            //Issue more tickets locally
            TicketInstance::create_multiple(
                asset.id,
                starting_tari_id,
                additional_ticket_count,
                org_wallet.id,
                connection,
            )?;
            //Issue more tickets on chain
            match asset.blockchain_asset_id {
                Some(a) => {
                    state.config.tari_client.modify_asset_increase_supply(&org_wallet.secret_key,
                                                                          &org_wallet.public_key,
                                                                          &a,
                                                                          requested_capacity as u64,
                    )?
                },
                None => return application::internal_server_error(
                    "Could not complete capacity increase because the asset has not been assigned on the blockchain",
                ),
            }
        } else if valid_ticket_count > requested_capacity {
            let nullify_ticket_count = valid_ticket_count - requested_capacity;
            let asset = Asset::find_by_ticket_type(&ticket_type.id, connection)?;
            let org_wallet =
                Wallet::find_default_for_organization(event.organization_id, connection)?;
            //Nullify tickets locally
            let tickets =
                TicketInstance::nullify_tickets(asset.id, nullify_ticket_count, connection)?;
            //Nullify tickets on chain
            if tickets.len() == nullify_ticket_count as usize {
                let tari_ids: Vec<u64> = (0..tickets.len())
                    .map(|i| tickets[i as usize].token_id as u64)
                    .collect();
                match asset.blockchain_asset_id {
                    Some(a) => {
                        state.config.tari_client.modify_asset_nullify_tokens(&org_wallet.secret_key,
                                                                             &org_wallet.public_key,
                                                                             &a,
                                                                             tari_ids,
                        )?
                    },
                    None => return application::internal_server_error(
                        "Could not complete capacity increase because the asset has not been assigned on the blockchain",
                    ),
                }
            } else {
                return application::internal_server_error(&format!(
                    "Unable to nullify the requested number ({}) of ticket instances",
                    requested_capacity
                ));
            }
        }
    }

    //Update the editable attributes of the ticket type
    let update_parameters = TicketTypeEditableAttributes {
        name: data.name.clone(),
        start_date: data.start_date,
        end_date: data.end_date,
        increment: data.increment,
    };
    let updated_ticket_type = ticket_type.update(update_parameters, connection)?;

    if data.ticket_pricing.is_some() {
        let data_ticket_pricing = data.into_inner().ticket_pricing.unwrap();
        //Retrieve the current list of pricing associated with this ticket_type and remove unwanted pricing
        let ticket_pricing = updated_ticket_type.ticket_pricing(connection)?;
        for current_ticket_pricing in &ticket_pricing {
            let mut found_flag = false;
            for request_ticket_pricing in &data_ticket_pricing {
                if request_ticket_pricing.id.is_some()
                    && current_ticket_pricing.id == request_ticket_pricing.id.unwrap()
                {
                    found_flag = true;
                    break;
                }
            }
            if !found_flag {
                current_ticket_pricing.destroy(connection)?;
            }
        }

        //Update the editable attributes for remaining ticket pricing
        for current_ticket_pricing in &data_ticket_pricing {
            if current_ticket_pricing.id.is_some() {
                //Update the ticket pricing
                let update_parameters = TicketPricingEditableAttributes {
                    name: current_ticket_pricing.name.clone(),
                    price_in_cents: current_ticket_pricing.price_in_cents,
                    start_date: current_ticket_pricing.start_date,
                    end_date: current_ticket_pricing.end_date,
                };
                let current_ticket_pricing_id = current_ticket_pricing.id.unwrap();
                let found_index = ticket_pricing
                    .iter()
                    .position(|ref r| r.id == current_ticket_pricing_id);
                match found_index {
                    Some(index) => ticket_pricing[index].update(update_parameters, connection)?,
                    None => {
                        return application::internal_server_error(&format!(
                            "Unable to find specified ticket pricing with id {}",
                            current_ticket_pricing_id
                        ));
                    }
                };
            } else if current_ticket_pricing.name.is_some()
                && current_ticket_pricing.price_in_cents.is_some()
                && current_ticket_pricing.start_date.is_some()
                && current_ticket_pricing.end_date.is_some()
            {
                //Only create a new pricing entry if all of its required data was provided
                let current_ticket_pricing_name = current_ticket_pricing.name.clone().unwrap();

                //Add new ticket pricing
                let _pricing_result = updated_ticket_type.add_ticket_pricing(
                    current_ticket_pricing_name,
                    current_ticket_pricing.start_date.unwrap(),
                    current_ticket_pricing.end_date.unwrap(),
                    current_ticket_pricing.price_in_cents.unwrap(),
                    connection,
                )?;
            } else {
                //TODO send error when all data was not specified

            }
        }
        updated_ticket_type.validate_record(connection)?;
    }

    Ok(HttpResponse::Ok().finish())
}
