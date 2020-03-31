use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, web::Path, FromRequest, HttpResponse};
use api::controllers::collection_items::*;
use api::controllers::collections::*;
use api::extractors::*;
use api::models::PathParameters;
// use db::models::*;
use db::models::{Collection, CollectionItem, Roles, UpdateCollectionItemAttributes};
use serde_json;
// use db::prelude::*;

#[actix_rt::test]
pub async fn create() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let conn = &database.connection;
    let auth_user = support::create_auth_user(Roles::User, None, &database);
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let event1 = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let collectible_id1 = event1.ticket_types(true, None, connection).unwrap()[0].id;
    database.create_purchased_tickets(&auth_user.user, collectible_id1, 1);

    let name = "Collection1".to_owned();
    let json = Json(CreateCollectionRequest { name: name.to_owned() });

    let collection = api::controllers::collections::create((conn.clone(), json, auth_user.clone()))
        .await
        .unwrap();

    let collection: Collection = serde_json::from_str(support::unwrap_body_to_string(&collection).unwrap()).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    let json = Json(CreateCollectionItemRequest {
        collectible_id: collectible_id1,
    });

    let response: HttpResponse = api::controllers::collection_items::create((conn.clone(), path, json, auth_user))
        .await
        .into();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[actix_rt::test]
pub async fn index() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let conn = &database.connection;
    let auth_user = support::create_auth_user(Roles::User, None, &database);
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let event1 = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let collectible_id1 = event1.ticket_types(true, None, connection).unwrap()[0].id;
    database.create_purchased_tickets(&auth_user.user, collectible_id1, 1);

    let name = "Collection1".to_owned();
    let json = Json(CreateCollectionRequest { name: name.to_owned() });

    let collection = api::controllers::collections::create((conn.clone(), json, auth_user.clone()))
        .await
        .unwrap();

    let collection: Collection = serde_json::from_str(support::unwrap_body_to_string(&collection).unwrap()).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    let json = Json(CreateCollectionItemRequest {
        collectible_id: collectible_id1,
    });
    api::controllers::collection_items::create((conn.clone(), path, json, auth_user.clone()))
        .await
        .unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    let response: HttpResponse = api::controllers::collection_items::index((conn.clone(), path, auth_user))
        .await
        .into();

    let collection_items: Vec<CollectionItem> =
        serde_json::from_str(&support::unwrap_body_to_string(&response).unwrap()).unwrap();
    assert_eq!(collection_items.len(), 1);
}

#[actix_rt::test]
pub async fn delete() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let conn = &database.connection;
    let auth_user = support::create_auth_user(Roles::User, None, &database);
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let event1 = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let collectible_id1 = event1.ticket_types(true, None, connection).unwrap()[0].id;
    database.create_purchased_tickets(&auth_user.user, collectible_id1, 1);

    let event2 = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let collectible_id2 = event2.ticket_types(true, None, connection).unwrap()[0].id;
    database.create_purchased_tickets(&auth_user.user, collectible_id2, 1);

    let name = "Collection1".to_owned();
    let json = Json(CreateCollectionRequest { name: name.to_owned() });

    let collection = api::controllers::collections::create((conn.clone(), json, auth_user.clone()))
        .await
        .unwrap();

    let collection: Collection = serde_json::from_str(support::unwrap_body_to_string(&collection).unwrap()).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    let json = Json(CreateCollectionItemRequest {
        collectible_id: collectible_id1,
    });

    api::controllers::collection_items::create((conn.clone(), path, json, auth_user.clone()))
        .await
        .unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    let json = Json(CreateCollectionItemRequest {
        collectible_id: collectible_id2,
    });
    api::controllers::collection_items::create((conn.clone(), path, json, auth_user.clone()))
        .await
        .unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    let response: HttpResponse = api::controllers::collection_items::index((conn.clone(), path, auth_user.clone()))
        .await
        .into();

    let collection_items: Vec<CollectionItem> =
        serde_json::from_str(&support::unwrap_body_to_string(&response).unwrap()).unwrap();
    assert_eq!(collection_items.len(), 2);

    let item_to_delete = &collection_items[0];

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = item_to_delete.id;

    api::controllers::collection_items::delete((conn.clone(), path, auth_user.clone()))
        .await
        .unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    let response: HttpResponse = api::controllers::collection_items::index((conn.clone(), path, auth_user.clone()))
        .await
        .into();

    let collection_items: Vec<CollectionItem> =
        serde_json::from_str(&support::unwrap_body_to_string(&response).unwrap()).unwrap();
    assert_eq!(collection_items.len(), 1);
}

#[actix_rt::test]
pub async fn update() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let conn = &database.connection;
    let auth_user = support::create_auth_user(Roles::User, None, &database);
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let event1 = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let collectible_id1 = event1.ticket_types(true, None, connection).unwrap()[0].id;
    database.create_purchased_tickets(&auth_user.user, collectible_id1, 1);

    let event2 = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let collectible_id2 = event2.ticket_types(true, None, connection).unwrap()[0].id;
    database.create_purchased_tickets(&auth_user.user, collectible_id2, 1);

    let name = "Collection1".to_owned();
    let json = Json(CreateCollectionRequest { name: name.to_owned() });

    let collection = api::controllers::collections::create((conn.clone(), json, auth_user.clone()))
        .await
        .unwrap();

    let collection: Collection = serde_json::from_str(support::unwrap_body_to_string(&collection).unwrap()).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    let json = Json(CreateCollectionItemRequest {
        collectible_id: collectible_id1,
    });

    api::controllers::collection_items::create((conn.clone(), path, json, auth_user.clone()))
        .await
        .unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    let json = Json(CreateCollectionItemRequest {
        collectible_id: collectible_id2,
    });

    api::controllers::collection_items::create((conn.clone(), path, json, auth_user.clone()))
        .await
        .unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    let response: HttpResponse = api::controllers::collection_items::index((conn.clone(), path, auth_user.clone()))
        .await
        .into();

    let collection_items: Vec<CollectionItem> =
        serde_json::from_str(&support::unwrap_body_to_string(&response).unwrap()).unwrap();
    assert_eq!(collection_items.len(), 2);

    let item_to_update = &collection_items
        .iter()
        .find(|&i| i.collectible_id == collectible_id1)
        .unwrap();
    let item_to_be_pointed_to = &collection_items
        .iter()
        .find(|&i| i.collectible_id == collectible_id2)
        .unwrap();

    // set next item to only other in the collection
    let json = Json(UpdateCollectionItemAttributes {
        next_collection_item_id: Some(Some(item_to_be_pointed_to.id)),
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = item_to_update.id;

    api::controllers::collection_items::update((conn.clone(), path, json, auth_user.clone()))
        .await
        .unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    let response: HttpResponse = api::controllers::collection_items::index((conn.clone(), path, auth_user.clone()))
        .await
        .into();

    let collection_items: Vec<CollectionItem> =
        serde_json::from_str(&support::unwrap_body_to_string(&response).unwrap()).unwrap();

    assert_eq!(
        collection_items
            .iter()
            .find(|&i| i.collectible_id == collectible_id1)
            .unwrap()
            .next_collection_item_id
            .unwrap(),
        item_to_be_pointed_to.id
    );

    // set next item to null
    let json = Json(UpdateCollectionItemAttributes {
        next_collection_item_id: Some(None),
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = item_to_update.id;

    api::controllers::collection_items::update((conn.clone(), path, json, auth_user.clone()))
        .await
        .unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    let response: HttpResponse = api::controllers::collection_items::index((conn.clone(), path, auth_user.clone()))
        .await
        .into();

    let collection_items: Vec<CollectionItem> =
        serde_json::from_str(&support::unwrap_body_to_string(&response).unwrap()).unwrap();
    assert!(collection_items
        .iter()
        .find(|&i| i.collectible_id == collectible_id1)
        .unwrap()
        .next_collection_item_id
        .is_none());
}
