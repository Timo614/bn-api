use db::dev::TestProject;
use db::prelude::*;

#[test]
fn commit() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let user1 = project.create_user().finish();
    let name1 = "Collection1";
    let collection1 = Collection::create(name1, user1.id).commit(conn).unwrap();

    assert_eq!(collection1.name, name1);
    assert_eq!(collection1.user_id, user1.id);
}

#[test]
fn new_collection_name_collision() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let user1 = project.create_user().finish();
    let user2 = project.create_user().finish();
    let name1 = "Collection1";
    let collection1_user1 = Collection::create(name1, user1.id).commit(conn);
    let collection1_user2 = Collection::create(name1, user2.id).commit(conn);
    let collection2_user1_dup = Collection::create(name1, user1.id).commit(conn);

    assert!(collection1_user1.is_ok());
    assert!(collection1_user2.is_ok());
    assert!(collection2_user1_dup.is_err());

    let er = collection2_user1_dup.unwrap_err();

    assert_eq!(er.code, errors::get_error_message(&ErrorCode::DuplicateKeyError).0);
}

#[test]
fn find() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let user1 = project.create_user().finish();
    let name1 = "Collection1";
    let collection1_user1 = Collection::create(name1, user1.id).commit(conn).unwrap();

    let found_collection = Collection::find(collection1_user1.id, conn).unwrap();

    assert_eq!(collection1_user1, found_collection);
}

#[test]
fn find_for_user() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let user1 = project.create_user().finish();
    let user2 = project.create_user().finish();
    let name1 = "Collection1";
    let name2 = "Collection2";
    let collection1_user1 = Collection::create(name1, user1.id).commit(conn).unwrap();
    let collection2_user1 = Collection::create(name2, user1.id).commit(conn).unwrap();
    Collection::create(name1, user2.id).commit(conn).unwrap();

    let found_collections = Collection::find_for_user(user1.id, conn).unwrap();

    assert_eq!(found_collections.len(), 2);
    assert_eq!(found_collections[0].id, collection1_user1.id);
    assert_eq!(found_collections[1].id, collection2_user1.id);
}

#[test]
fn update() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let user1 = project.create_user().finish();
    let name1 = "Collection1";
    let collection1 = Collection::create(name1, user1.id).commit(conn).unwrap();
    let event1 = project.create_event().with_ticket_pricing().finish();
    project
        .create_order()
        .for_event(&event1)
        .for_user(&user1)
        .quantity(2)
        .is_paid()
        .finish();
    let collectible_id1 = event1.ticket_types(false, None, conn).unwrap().first().unwrap().id;

    let update1 = UpdateCollectionAttributes {
        featured_collectible_id: Some(Some(collectible_id1)),
    };

    let updated_collection1 = Collection::update(collection1, update1, conn).unwrap();

    let found_collection1 = Collection::find(updated_collection1.id, conn).unwrap();

    assert_eq!(found_collection1.featured_collectible_id.unwrap(), collectible_id1);

    let update2 = UpdateCollectionAttributes {
        featured_collectible_id: Some(None),
    };

    let updated_collection2 = Collection::update(found_collection1, update2, conn).unwrap();

    let found_collection2 = Collection::find(updated_collection2.id, conn).unwrap();

    assert!(found_collection2.featured_collectible_id.is_none());
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let user1 = project.create_user().finish();
    let name1 = "Collection1";
    let name2 = "Collection2";
    let collection1_user1 = Collection::create(name1, user1.id).commit(conn).unwrap();
    let collection2_user1 = Collection::create(name2, user1.id).commit(conn).unwrap();

    let result = Collection::destroy(collection1_user1, conn);
    assert!(result.is_ok());
    let found_collections = Collection::find_for_user(user1.id, conn).unwrap();

    assert_eq!(found_collections.len(), 1);
    assert_eq!(found_collections[0].id, collection2_user1.id);
}
