use actix_web::{http::StatusCode, web::Path, web::Query, HttpResponse};
use auth::user::User;
use bigneon_db::models::*;
use db::Connection;
use errors::*;
use extractors::*;
use models::PathParameters;
use models::WebPayload;

pub fn index(
    connection: Connection,
    query_parameters: Query<PagingParameters>,
) -> Result<WebPayload<Region>, BigNeonError> {
    //TODO refactor query using paging parameters
    let regions = Region::all(connection.get())?;

    Ok(WebPayload::new(
        StatusCode::OK,
        Payload::from_data(regions, query_parameters.page(), query_parameters.limit()),
    ))
}

pub fn show(
    connection: Connection,
    parameters: Path<PathParameters>,
) -> Result<HttpResponse, BigNeonError> {
    let region = Region::find(parameters.id, connection.get())?;
    Ok(HttpResponse::Ok().json(&region))
}

pub fn create(
    connection: Connection,
    new_region: Json<NewRegion>,
    user: User,
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::RegionWrite)?;
    let connection = connection.get();
    let region = new_region.into_inner().commit(connection)?;
    Ok(HttpResponse::Created().json(&region))
}

pub fn update(
    connection: Connection,
    parameters: Path<PathParameters>,
    region_parameters: Json<RegionEditableAttributes>,
    user: User,
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::RegionWrite)?;
    let connection = connection.get();
    let region = Region::find(parameters.id, connection)?;
    let updated_region = region.update(region_parameters.into_inner(), connection)?;
    Ok(HttpResponse::Ok().json(updated_region))
}
