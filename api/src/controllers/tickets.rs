use actix_web::State;
use actix_web::{http::StatusCode, HttpResponse, Json, Path, Query};
use auth::user::User;
use bigneon_db::models::*;
use chrono::prelude::*;
use db::Connection;
use errors::*;
use helpers::application;
use mail::mailers;
use models::{OptionalPathParameters, Paging, PathParameters, Payload, SearchParam, SortingDir};
use server::AppState;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SearchParameters {
    pub start_utc: Option<NaiveDateTime>,
    pub end_utc: Option<NaiveDateTime>,
}
impl SearchParameters {
    pub fn create_paging_struct(&self) -> Paging {
        let mut default_tags = Vec::new();
        if let Some(ref i) = self.start_utc {
            let new_value = SearchParam {
                name: "start_utc".to_owned(),
                values: vec![i.to_string()],
            };
            default_tags.push(new_value);
        }
        if let Some(ref i) = self.end_utc {
            let new_value = SearchParam {
                name: "end_utc".to_owned(),
                values: vec![i.to_string()],
            };
            default_tags.push(new_value);
        }
        Paging {
            page: 0,
            limit: 100,
            sort: "".to_owned(),
            dir: SortingDir::None,
            total: 0,
            tags: default_tags,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct TicketRedeemRequest {
    pub redeem_key: String,
}

pub fn index(
    (connection, path, parameters, auth_user): (
        Connection,
        Path<OptionalPathParameters>,
        Query<SearchParameters>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    //todo convert to use pagingparams

    let connection = connection.get();

    let queryparms = parameters.create_paging_struct();
    let tickets = TicketInstance::find_for_user_for_display(
        auth_user.id(),
        path.id,
        parameters.start_utc,
        parameters.end_utc,
        connection,
    )?;
    let ticket_count = tickets.len();
    let mut payload = Payload {
        data: tickets,
        paging: Paging::clone_with_new_total(&queryparms, ticket_count as u64),
    };
    payload.paging.limit = ticket_count as u64;
    // If specifying event drill into tuple vector to return tickets alone
    if path.id.is_some() && !payload.data.is_empty() {
        let mut payload2 = Payload {
            data: (payload.data[0].1).clone(),
            paging: Paging::clone_with_new_total(&queryparms, ticket_count as u64),
        };
        payload2.paging.limit = payload2.data.len() as u64;
        payload2.paging.total = payload2.data.len() as u64;
        return Ok(HttpResponse::Ok().json(&payload2));
    }

    Ok(HttpResponse::Ok().json(&payload))
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct ShowTicketResponse {
    pub event: DisplayEvent,
    pub user: Option<DisplayUser>,
    pub ticket: DisplayTicket,
}

pub fn show(
    (connection, parameters, auth_user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let (event, user, ticket) = TicketInstance::find_for_display(parameters.id, connection)?;
    let db_event = Event::find(event.id, connection)?;
    let organization = db_event.organization(connection)?;

    if !auth_user.has_scope(Scopes::TicketAdmin, Some(&organization), connection)?
        && (user.is_none() || user.as_ref().unwrap().id != auth_user.id())
    {
        return application::unauthorized();
    }

    let ticket_response = ShowTicketResponse {
        event,
        user,
        ticket,
    };

    Ok(HttpResponse::Ok().json(&ticket_response))
}

pub fn redeem(
    (connection, parameters, redeem_parameters, auth_user, state): (
        Connection,
        Path<PathParameters>,
        Json<TicketRedeemRequest>,
        User,
        State<AppState>,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let (event, ticket) = TicketInstance::find_for_processing(parameters.id, connection)?;
    let db_event = Event::find(event.id, connection)?;
    let organization = db_event.organization(connection)?;

    if !auth_user.has_scope(Scopes::TicketAdmin, Some(&organization), connection)? {
        return application::unauthorized();
    }

    let result =
        TicketInstance::redeem_ticket(ticket.id, redeem_parameters.redeem_key.clone(), connection);

    match result {
        Ok(r) => match r {
                RedeemResults::TicketRedeemSuccess => {
                    //Redeem ticket on chain
                    let asset = Asset::find(ticket.asset_id, connection)?;
                    match asset.blockchain_asset_id {
                        Some(a) => {
                            let wallet = Wallet::find(ticket.wallet_id, connection)?;
                            state.config.tari_client.modify_asset_redeem_token(&wallet.secret_key, &wallet.public_key,
                                                                               &a,
                                                                               vec![ticket.token_id as u64],
                            )?;
                            Ok(HttpResponse::Ok().json(json!({"success": true})))
                        },
                        None => Ok(HttpResponse::Ok().json(json!({"success": false, "message": "Could not complete this checkout because the asset has not been assigned on the blockchain.".to_string()}))),
                    }
                },
                RedeemResults::TicketAlreadyRedeemed => Ok(HttpResponse::Ok().json(json!({"success": false, "message": "Ticket has already been redeemed.".to_string()}))),
                RedeemResults::TicketInvalid => Ok(HttpResponse::Ok().json(json!({"success": false, "message": "Ticket is invalid.".to_string()}))),
            },
        Err(e) => Ok(HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            .into_builder()
            .json(json!({"error": e.cause.unwrap().to_string(),}))),
    }
}

pub fn show_redeemable_ticket(
    (connection, parameters, auth_user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let (event, user, _ticket) = TicketInstance::find_for_display(parameters.id, connection)?;
    let db_event = Event::find(event.id, connection)?;
    let organization = db_event.organization(connection)?;

    if !auth_user.has_scope(Scopes::TicketAdmin, Some(&organization), connection)?
        && (user.is_none() || user.unwrap().id != auth_user.id())
    {
        return application::unauthorized();
    }

    let redeemable_ticket = TicketInstance::show_redeemable_ticket(parameters.id, connection)?;

    Ok(HttpResponse::Ok().json(&redeemable_ticket))
}

pub fn send_via_email(
    (connection, send_tickets_request, auth_user, state): (
        Connection,
        Json<SendTicketsRequest>,
        User,
        State<AppState>,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    if !auth_user.has_scope(Scopes::TicketTransfer, None, connection)? {
        return application::unauthorized();
    }

    let authorization = TicketInstance::authorize_ticket_transfer(
        auth_user.id(),
        send_tickets_request.ticket_ids.clone(),
        send_tickets_request
            .validity_period_in_seconds
            .unwrap_or(604_800) as u32,
        connection,
    )?;

    mailers::tickets::send_tickets(
        &state.config,
        &send_tickets_request.email,
        &authorization.sender_user_id.to_string(),
        authorization.num_tickets,
        &authorization.transfer_key.to_string(),
        &authorization.signature,
        &auth_user.user,
    ).deliver()?;

    Ok(HttpResponse::Ok().finish())
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SendTicketsRequest {
    pub ticket_ids: Vec<Uuid>,
    pub validity_period_in_seconds: Option<i64>,
    pub email: String,
}

pub fn transfer_authorization(
    (connection, transfer_tickets_request, auth_user): (
        Connection,
        Json<TransferTicketRequest>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    if !auth_user.has_scope(Scopes::TicketTransfer, None, connection)? {
        return application::unauthorized();
    }

    let transfer_authorization = TicketInstance::authorize_ticket_transfer(
        auth_user.id(),
        transfer_tickets_request.ticket_ids.clone(),
        transfer_tickets_request.validity_period_in_seconds as u32,
        connection,
    )?;

    Ok(HttpResponse::Ok().json(&transfer_authorization))
}

pub fn receive_transfer(
    (connection, transfer_authorization, auth_user, state): (
        Connection,
        Json<TransferAuthorization>,
        User,
        State<AppState>,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    if !auth_user.has_scope(Scopes::TicketTransfer, None, connection)? {
        return application::unauthorized();
    }

    let sender_wallet =
        Wallet::find_default_for_user(transfer_authorization.sender_user_id, connection)?;
    let receiver_wallet = Wallet::find_default_for_user(auth_user.id(), connection)?;

    let tickets = TicketInstance::receive_ticket_transfer(
        transfer_authorization.into_inner(),
        &sender_wallet,
        &receiver_wallet.id,
        connection,
    )?;

    //Assemble token ids and ticket instance ids for each asset in the order
    let mut tokens_per_asset: HashMap<Uuid, Vec<u64>> = HashMap::new();
    for ticket in &tickets {
        tokens_per_asset
            .entry(ticket.asset_id)
            .or_insert_with(|| Vec::new())
            .push(ticket.token_id as u64);
    }

    //Transfer each ticket on chain in batches per asset
    for (asset_id, token_ids) in &tokens_per_asset {
        let asset = Asset::find(*asset_id, connection)?;
        match asset.blockchain_asset_id {
            Some(a) => {
                state.config.tari_client.transfer_tokens(&sender_wallet.secret_key, &sender_wallet.public_key,
                                                         &a,
                                                         token_ids.clone(),
                                                         receiver_wallet.public_key.clone(),
                )?
            },
            None => return application::internal_server_error(
                "Could not complete ticket transfer because the asset has not been assigned on the blockchain",
            ),
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[derive(Clone, Deserialize, Serialize)]
pub struct TransferTicketRequest {
    pub ticket_ids: Vec<Uuid>,
    pub validity_period_in_seconds: i64,
}
