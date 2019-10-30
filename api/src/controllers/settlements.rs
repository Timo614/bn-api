use actix_web::{http::StatusCode, HttpResponse, Path, Query};
use auth::user::User as AuthUser;
use bigneon_db::models::*;
use chrono::prelude::*;
use db::Connection;
use errors::*;
use extractors::*;
use models::{PathParameters, WebPayload};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct NewSettlementRequest {
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub comment: Option<String>,
}

pub fn index(
    (connection, query, path, user): (
        Connection,
        Query<PagingParameters>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<WebPayload<Settlement>, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(path.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgFinancialReports, &organization, connection)?;

    let payload = Settlement::find_for_organization(
        path.id,
        Some(query.limit()),
        Some(query.page() * query.limit()),
        connection,
    )?;

    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub fn create(
    (connection, new_settlement, path, user): (
        Connection,
        Json<NewSettlementRequest>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(path.id, connection)?;
    user.requires_scope_for_organization(Scopes::SettlementWrite, &organization, connection)?;
    let new_settlement = Settlement::create(
        organization.id,
        new_settlement.start_time,
        new_settlement.end_time,
        SettlementStatus::PendingSettlement,
        new_settlement.comment.clone(),
        organization.settlement_type == SettlementTypes::PostEvent,
    );
    let settlement = new_settlement.commit(Some(user.user), connection)?;
    Ok(HttpResponse::Created().json(&settlement))
}

pub fn show(
    (connection, path, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let settlement = Settlement::find(path.id, connection)?;
    let organization = Organization::find(settlement.organization_id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgFinancialReports, &organization, connection)?;
    let display_settlement: DisplaySettlement = settlement.for_display(connection)?;
    Ok(HttpResponse::Ok().json(&display_settlement))
}

pub fn destroy(
    (connection, path, user): (Connection, Path<PathParameters>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    user.requires_scope(Scopes::OrgAdmin)?;
    let settlement = Settlement::find(path.id, connection)?;
    settlement.destroy(connection)?;
    Ok(HttpResponse::Ok().json({}))
}
