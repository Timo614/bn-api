use db::dev::TestProject;
use db::prelude::*;

#[test]
fn commit() {
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

    let collection_item1 = CollectionItem::create(collection1.id, collectible_id1)
        .commit(conn)
        .unwrap();

    assert_eq!(collection_item1.collectible_id, collectible_id1);
    assert_eq!(collection_item1.collection_id, collection1.id);
}

#[test]
fn new_collection_item_collision() {
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
    let event2 = project.create_event().with_ticket_pricing().finish();
    project
        .create_order()
        .for_event(&event2)
        .for_user(&user1)
        .quantity(1)
        .is_paid()
        .finish();
    let collectible_id2 = event2.ticket_types(false, None, conn).unwrap().first().unwrap().id;

    let collection_item1_collectible1 = CollectionItem::create(collection1.id, collectible_id1).commit(conn);
    let collection_item1_collectible2 = CollectionItem::create(collection1.id, collectible_id2).commit(conn);
    let collectible1_dup = CollectionItem::create(collection1.id, collectible_id1).commit(conn);

    assert!(collection_item1_collectible1.is_ok());
    assert!(collection_item1_collectible2.is_ok());
    assert!(collectible1_dup.is_err());

    let er = collectible1_dup.unwrap_err();

    assert_eq!(er.code, errors::get_error_message(&ErrorCode::DuplicateKeyError).0);
}

#[test]
fn find() {
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

    let collection_item1 = CollectionItem::create(collection1.id, collectible_id1)
        .commit(conn)
        .unwrap();

    let found_item = CollectionItem::find(collection_item1.id, conn).unwrap();

    assert_eq!(collection_item1, found_item);
}

#[test]
fn find_for_collection_with_num_owned() {
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
    let event2 = project.create_event().with_ticket_pricing().finish();
    project
        .create_order()
        .for_event(&event2)
        .for_user(&user1)
        .quantity(1)
        .is_paid()
        .finish();
    let collectible_id2 = event2.ticket_types(false, None, conn).unwrap().first().unwrap().id;

    CollectionItem::create(collection1.id, collectible_id1)
        .commit(conn)
        .unwrap();
    CollectionItem::create(collection1.id, collectible_id2)
        .commit(conn)
        .unwrap();

    let found_items = CollectionItem::find_for_collection_with_num_owned(collection1.id, user1.id, conn).unwrap();

    assert_eq!(
        found_items
            .iter()
            .find(|&i| i.collectible_id == collectible_id1)
            .unwrap()
            .number_owned,
        2
    );
    assert_eq!(
        found_items
            .iter()
            .find(|&i| i.collectible_id == collectible_id2)
            .unwrap()
            .number_owned,
        1
    );
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
    let event2 = project.create_event().with_ticket_pricing().finish();
    project
        .create_order()
        .for_event(&event2)
        .for_user(&user1)
        .quantity(1)
        .is_paid()
        .finish();
    let collectible_id2 = event2.ticket_types(false, None, conn).unwrap().first().unwrap().id;

    let collection_item1_collectible1 = CollectionItem::create(collection1.id, collectible_id1)
        .commit(conn)
        .unwrap();
    let collection_item1_collectible1_clone1 = collection_item1_collectible1.clone();
    let collection_item1_collectible1_clone2 = collection_item1_collectible1.clone();
    let collection_item1_collectible2 = CollectionItem::create(collection1.id, collectible_id2)
        .commit(conn)
        .unwrap();

    let update1 = UpdateCollectionItemAttributes {
        next_collection_item_id: Some(Some(collection_item1_collectible2.id)),
    };

    CollectionItem::update(collection_item1_collectible1, update1, conn).unwrap();

    let found_items1 = CollectionItem::find_for_collection_with_num_owned(collection1.id, user1.id, conn).unwrap();

    assert_eq!(
        found_items1
            .iter()
            .find(|&i| i.collectible_id == collectible_id1)
            .unwrap()
            .next_collection_item_id
            .unwrap(),
        collection_item1_collectible2.id
    );

    let update2 = UpdateCollectionItemAttributes {
        next_collection_item_id: None,
    };

    CollectionItem::update(collection_item1_collectible1_clone1, update2, conn).unwrap();

    let found_items2 = CollectionItem::find_for_collection_with_num_owned(collection1.id, user1.id, conn).unwrap();

    assert_eq!(
        found_items2
            .iter()
            .find(|&i| i.collectible_id == collectible_id1)
            .unwrap()
            .next_collection_item_id
            .unwrap(),
        collection_item1_collectible2.id
    );

    let update3 = UpdateCollectionItemAttributes {
        next_collection_item_id: Some(None),
    };

    CollectionItem::update(collection_item1_collectible1_clone2, update3, conn).unwrap();

    let found_items3 = CollectionItem::find_for_collection_with_num_owned(collection1.id, user1.id, conn).unwrap();

    assert_eq!(
        found_items3
            .iter()
            .find(|&i| i.collectible_id == collectible_id1)
            .unwrap()
            .next_collection_item_id,
        None
    );
}

#[test]
fn destroy() {
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
    let event2 = project.create_event().with_ticket_pricing().finish();
    project
        .create_order()
        .for_event(&event2)
        .for_user(&user1)
        .quantity(1)
        .is_paid()
        .finish();
    let collectible_id2 = event2.ticket_types(false, None, conn).unwrap().first().unwrap().id;

    let collection_item1_collectible1 = CollectionItem::create(collection1.id, collectible_id1)
        .commit(conn)
        .unwrap();
    CollectionItem::create(collection1.id, collectible_id2)
        .commit(conn)
        .unwrap();

    CollectionItem::destroy(collection_item1_collectible1, conn).unwrap();

    let found_items = CollectionItem::find_for_collection_with_num_owned(collection1.id, user1.id, conn).unwrap();

    assert_eq!(found_items.len(), 1);
}
