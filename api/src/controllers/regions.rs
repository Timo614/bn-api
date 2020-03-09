use crate::auth::user::User;
use crate::db::{CacheDatabase, Connection};
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::*;
use crate::models::PathParameters;
use crate::models::WebPayload;
use actix_web::{http::StatusCode, HttpResponse, Path, Query};
use bigneon_db::models::*;

pub fn index(
    (connection, query_parameters): (Connection, Query<PagingParameters>),
) -> Result<WebPayload<Region>, BigNeonError> {
    //TODO refactor query using paging parameters
    let regions = Region::all(connection.get())?;

    Ok(WebPayload::new(
        StatusCode::OK,
        Payload::from_data(regions, query_parameters.page(), query_parameters.limit(), None),
    ))
}

pub fn show((connection, parameters): (Connection, Path<PathParameters>)) -> Result<HttpResponse, BigNeonError> {
    let region = Region::find(parameters.id, connection.get())?;
    Ok(HttpResponse::Ok().json(&region))
}

pub fn create(
    (connection, new_region, user): (Connection, Json<NewRegion>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::RegionWrite)?;
    let connection = connection.get();
    let region = new_region.into_inner().commit(connection)?;
    Ok(HttpResponse::Created().json(&region))
}

pub fn update(
    (connection, parameters, region_parameters, user, cache_database): (
        Connection,
        Path<PathParameters>,
        Json<RegionEditableAttributes>,
        User,
        CacheDatabase,
    ),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::RegionWrite)?;
    let connection = connection.get();
    let region = Region::find(parameters.id, connection)?;
    let updated_region = region.update(region_parameters.into_inner(), connection)?;

    cache_database
        .inner
        .clone()
        .and_then(|conn| caching::delete_by_key_fragment(conn, region.id).ok());
    Ok(HttpResponse::Ok().json(updated_region))
}
