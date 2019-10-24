use actix_web::{http::StatusCode, web::Data, web::Path, web::Query, HttpResponse};
use auth::user::User;
use bigneon_db::models::{User as DbUser, *};
use chrono::prelude::*;
use communications::{mailers, smsers};
use db::Connection;
use diesel::PgConnection;
use errors::*;
use helpers::application;
use models::{OptionalPathParameters, PathParameters, WebPayload};
use server::AppState;

#[derive(Deserialize, Clone)]
pub struct TransferFilters {
    source_or_destination: Option<String>,
    start_utc: Option<NaiveDateTime>,
    end_utc: Option<NaiveDateTime>,
}

pub fn show_by_transfer_key(
    connection: Connection,
    path: Path<PathParameters>,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let transfer = Transfer::find_by_transfer_key(path.id, connection)?;
    // if you have the transfer key, you can view the transfer
    Ok(HttpResponse::Ok().json(&transfer.for_display(connection)?))
}

pub fn index(
    connection: Connection,
    paging_query: Query<PagingParameters>,
    filter_query: Query<TransferFilters>,
    path: Path<OptionalPathParameters>,
    auth_user: User,
) -> Result<WebPayload<DisplayTransfer>, BigNeonError> {
    let connection = connection.get();
    let mut lookup_user_id = auth_user.id();

    match path.id {
        Some(order_id) => {
            let order = Order::find(order_id, connection)?;
            lookup_user_id = order.on_behalf_of_user_id.unwrap_or(order.user_id);
            if order.on_behalf_of_user_id != Some(auth_user.id())
                && !(order.on_behalf_of_user_id.is_none() && order.user_id == auth_user.id())
            {
                auth_user.requires_scope_for_order(Scopes::TransferRead, &order, connection)?;
            } else {
                auth_user.requires_scope(Scopes::TransferReadOwn)?;
            }
        }
        None => {
            auth_user.requires_scope(Scopes::TransferReadOwn)?;
        }
    }

    let source_or_destination = match filter_query
        .source_or_destination
        .clone()
        .unwrap_or("source".to_string())
        .as_str()
    {
        "source" => SourceOrDestination::Source,
        _ => SourceOrDestination::Destination,
    };

    let mut payload = Transfer::find_for_user_for_display(
        lookup_user_id,
        path.id,
        source_or_destination,
        filter_query.start_utc,
        filter_query.end_utc,
        Some(paging_query.limit()),
        Some(paging_query.page()),
        connection,
    )?;
    payload
        .paging
        .tags
        .insert("start_utc".to_string(), json!(filter_query.start_utc));
    payload
        .paging
        .tags
        .insert("end_utc".to_string(), json!(filter_query.end_utc));
    payload.paging.tags.insert(
        "source_or_destination".to_string(),
        json!(filter_query.source_or_destination),
    );
    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub fn cancel(
    connection: Connection,
    path: Path<PathParameters>,
    auth_user: User,
    state: Data<AppState>,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let transfer = Transfer::find(path.id, connection)?;
    check_transfer_cancel_access(&transfer, &auth_user, connection)?;

    let transfer = transfer.cancel(auth_user.id(), None, connection)?;
    let source_user = DbUser::find(transfer.source_user_id, connection)?;

    if let Some(transfer_message_type) = transfer.transfer_message_type {
        if let Some(transfer_address) = &transfer.transfer_address {
            match transfer_message_type {
                TransferMessageType::Phone => {
                    smsers::tickets::transfer_cancelled(
                        &state.config,
                        transfer_address.clone(),
                        &source_user,
                        connection,
                    )?;
                }
                TransferMessageType::Email => {
                    mailers::tickets::transfer_cancelled(
                        &state.config,
                        transfer_address.clone(),
                        &source_user,
                        &transfer,
                        connection,
                    )?;
                }
            }
        }
    }

    if let Some(source_email) = source_user.email.clone() {
        mailers::tickets::transfer_cancelled_receipt(
            &state.config,
            source_email,
            &source_user,
            &transfer,
            connection,
        )?;
    }

    Ok(HttpResponse::Ok().json(&transfer.for_display(connection)?))
}

fn check_transfer_cancel_access(
    transfer: &Transfer,
    user: &User,
    connection: &PgConnection,
) -> Result<(), BigNeonError> {
    if transfer.source_user_id != user.id() {
        let mut valid = true;
        let events = transfer.events(connection)?;
        for event in events {
            let org = event.organization(connection)?;
            valid = valid
                && user.has_scope_for_organization_event(
                    Scopes::TransferCancel,
                    &org,
                    event.id,
                    connection,
                )?;
        }

        if !valid {
            application::forbidden::<HttpResponse>("You do not have access to this transfer")?;
        }
    } else {
        user.requires_scope(Scopes::TransferCancelOwn)?;
    }
    Ok(())
}
