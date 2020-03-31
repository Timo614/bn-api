use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, web::Path, FromRequest, HttpResponse};
use api::controllers::collections::*;
use api::extractors::*;
use api::models::PathParameters;
use db::models::{Collection, Roles, UpdateCollectionAttributes};
use serde_json;

pub async fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let auth_user = support::create_auth_user_from_user(&user, role, Some(&organization), &database);

    let name = "Collection1".to_owned();
    let json = Json(CreateCollectionRequest { name: name.to_owned() });

    let response: HttpResponse = api::controllers::collections::create((database.connection.into(), json, auth_user))
        .await
        .into();

    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let collection: Collection = serde_json::from_str(&body).unwrap();
        assert_eq!(collection.name, name);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn index(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let auth_user = support::create_auth_user(role, None, &database);
    let name = "Collection1".to_owned();
    let json = Json(CreateCollectionRequest { name: name.to_owned() });

    api::controllers::collections::create((database.connection.clone().into(), json, auth_user.clone()))
        .await
        .unwrap();

    let response = api::controllers::collections::index((database.connection.clone().into(), auth_user.clone()))
        .await
        .into();

    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let collection: Vec<Collection> = serde_json::from_str(&body).unwrap();
        assert_eq!(collection[0].name, name);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn delete(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let auth_user = support::create_auth_user(role, None, &database);
    let name = "Collection1".to_owned();
    let json = Json(CreateCollectionRequest { name: name.to_owned() });

    let created = api::controllers::collections::create((database.connection.clone().into(), json, auth_user.clone()))
        .await
        .unwrap();

    let collection: Collection = serde_json::from_str(support::unwrap_body_to_string(&created).unwrap()).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    api::controllers::collections::delete((database.connection.clone().into(), path, auth_user.clone()))
        .await
        .unwrap();

    let response = api::controllers::collections::index((database.connection.clone().into(), auth_user.clone()))
        .await
        .into();

    if should_test_succeed {
        let body = support::unwrap_body_to_string(&response).unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let collection: Vec<Collection> = serde_json::from_str(&body).unwrap();
        assert_eq!(collection.len(), 0);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub async fn update(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let conn = &database.connection;
    let auth_user = support::create_auth_user(role, None, &database);
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

    let response = api::controllers::collections::create((conn.clone(), json, auth_user.clone()))
        .await
        .unwrap();

    if !should_test_succeed {
        support::expects_unauthorized(&response);
    }

    let collection: Collection = serde_json::from_str(support::unwrap_body_to_string(&response).unwrap()).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection.id;

    let json = Json(UpdateCollectionAttributes {
        featured_collectible_id: Some(Some(collectible_id1)),
    });

    api::controllers::collections::update((conn.clone(), path, json, auth_user.clone()))
        .await
        .unwrap();

    let response: HttpResponse = api::controllers::collections::index((conn.clone(), auth_user.clone()))
        .await
        .into();

    let collection: Vec<Collection> =
        serde_json::from_str(&support::unwrap_body_to_string(&response).unwrap()).unwrap();
    assert_eq!(collection[0].featured_collectible_id.unwrap(), collectible_id1);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = collection[0].id;

    let json = Json(UpdateCollectionAttributes {
        featured_collectible_id: Some(None),
    });

    api::controllers::collections::update((conn.clone(), path, json, auth_user.clone()))
        .await
        .unwrap();

    let response: HttpResponse = api::controllers::collections::index((conn.clone(), auth_user.clone()))
        .await
        .into();

    let collection: Vec<Collection> =
        serde_json::from_str(&support::unwrap_body_to_string(&response).unwrap()).unwrap();
    assert_eq!(collection[0].featured_collectible_id, None);
}
