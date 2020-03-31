use crate::auth::user::User;
use crate::database::Connection;
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::application;
use crate::models::*;
use actix_web::{web::Path, HttpResponse};
use db::models::*;
use uuid::Uuid;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct CreateCollectionItemRequest {
    pub collectible_id: Uuid,
}

pub async fn create(
    (connection, path, create_collection_item_request, user): (
        Connection,
        Path<PathParameters>,
        Json<CreateCollectionItemRequest>,
        User,
    ),
) -> Result<HttpResponse, ApiError> {
    let conn = connection.get();
    // todo: verify user owns at least one collectible of this type
    // todo: decide what a reasonable limit for number of collectibles in a collection is
    let collection = Collection::find(path.id, conn)?;
    if collection.user_id != user.id() {
        return application::forbidden("User does not have access to this collection");
    }

    let collection_item = CollectionItem::create(path.id, create_collection_item_request.collectible_id);
    let created_item = collection_item.commit(conn)?;
    Ok(HttpResponse::Created().json(&created_item))
}

pub async fn index(
    (connection, path, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, ApiError> {
    let conn = connection.get();
    let collection = Collection::find(path.id, conn)?;
    if collection.user_id != user.id() {
        return application::forbidden("User does not have access to this collection");
    }
    let display_collection_items = CollectionItem::find_for_collection_with_num_owned(path.id, user.id(), conn)?;
    Ok(HttpResponse::Ok().json(&display_collection_items))
}

pub async fn update(
    (connection, path, collection_item_update_attr, user): (
        Connection,
        Path<PathParameters>,
        Json<UpdateCollectionItemAttributes>,
        User,
    ),
) -> Result<HttpResponse, ApiError> {
    let conn = connection.get();
    let item = CollectionItem::find(path.id, conn)?;
    let collection = Collection::find(item.collection_id, conn)?;
    if collection.user_id != user.id() {
        return application::forbidden("User does not have access to this collection");
    }
    let updated_item = CollectionItem::update(item, collection_item_update_attr.into_inner(), conn)?;
    Ok(HttpResponse::Ok().json(&updated_item))
}

pub async fn delete(
    (connection, path, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, ApiError> {
    let conn = connection.get();
    let item = CollectionItem::find(path.id, conn)?;
    let collection = Collection::find(item.collection_id, conn)?;
    if collection.user_id != user.id() {
        return application::forbidden("User does not have access to this collection");
    }
    CollectionItem::destroy(item, conn)?;
    Ok(HttpResponse::Ok().finish())
}
