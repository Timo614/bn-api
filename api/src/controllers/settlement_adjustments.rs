use actix_web::{web::Path, HttpResponse};
use auth::user::User as AuthUser;
use bigneon_db::models::*;
use db::Connection;
use errors::*;
use extractors::*;
use models::PathParameters;

pub fn index(
    connection: Connection,
    path: Path<PathParameters>,
    user: AuthUser,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let settlement = Settlement::find(path.id, connection)?;
    let organization = Organization::find(settlement.organization_id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgFinancialReports, &organization, connection)?;
    Ok(HttpResponse::Ok().json(settlement.adjustments(connection)?))
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct NewSettlementAdjustmentRequest {
    pub amount_in_cents: i64,
    pub note: Option<String>,
    pub settlement_adjustment_type: SettlementAdjustmentTypes,
}

pub fn create(
    connection: Connection,
    path: Path<PathParameters>,
    json: Json<NewSettlementAdjustmentRequest>,
    user: AuthUser,
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrgAdmin)?;
    let connection = connection.get();
    let settlement_adjustment = SettlementAdjustment::create(
        path.id,
        json.settlement_adjustment_type,
        json.note.clone(),
        json.amount_in_cents,
    )
    .commit(connection)?;
    Ok(HttpResponse::Created().json(&settlement_adjustment))
}

pub fn destroy(
    connection: Connection,
    path: Path<PathParameters>,
    user: AuthUser,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    user.requires_scope(Scopes::OrgAdmin)?;
    let settlement_adjustment = SettlementAdjustment::find(path.id, connection)?;
    settlement_adjustment.destroy(connection)?;
    Ok(HttpResponse::Ok().json({}))
}
