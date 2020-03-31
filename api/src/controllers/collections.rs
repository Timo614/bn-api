use crate::auth::user::User;
use crate::database::Connection;
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::application;
use crate::models::*;
use actix_web::{web::Path, HttpResponse};
use db::models::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCollectionRequest {
    pub name: String,
}

pub async fn create(
    (connection, create_collection_request, user): (Connection, Json<CreateCollectionRequest>, User),
) -> Result<HttpResponse, ApiError> {
    let conn = connection.get();
    user.requires_scope(Scopes::CollectionWrite)?;
    let new_collection = Collection::create(&create_collection_request.name, user.id());
    let created_collection = new_collection.commit(conn)?;
    Ok(HttpResponse::Created().json(&created_collection))
}

pub async fn index((connection, user): (Connection, User)) -> Result<HttpResponse, ApiError> {
    let conn = connection.get();
    user.requires_scope(Scopes::CollectionRead)?;
    let user_collections = Collection::find_for_user(user.id(), conn)?;

    Ok(HttpResponse::Ok().json(user_collections))
}

pub async fn update(
    (connection, path, collection_update_attr, user): (
        Connection,
        Path<PathParameters>,
        Json<UpdateCollectionAttributes>,
        User,
    ),
) -> Result<HttpResponse, ApiError> {
    let conn = connection.get();
    user.requires_scope(Scopes::CollectionWrite)?;
    let collection = Collection::find(path.id, conn)?;
    if collection.user_id != user.id() {
        return application::forbidden("User does not have access to this collection");
    }
    let updated_collection = Collection::update(collection, collection_update_attr.into_inner(), conn)?;
    Ok(HttpResponse::Ok().json(&updated_collection))
}

pub async fn delete(
    (connection, path, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, ApiError> {
    let conn = connection.get();
    user.requires_scope(Scopes::CollectionWrite)?;
    let collection = Collection::find(path.id, conn)?;
    if collection.user_id != user.id() {
        return application::forbidden("User does not have access to this collection");
    }
    Collection::destroy(collection, conn)?;
    Ok(HttpResponse::Ok().finish())
}
