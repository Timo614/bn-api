use actix_web::{http::StatusCode, FromRequest, HttpResponse, Json, Path};
use bigneon_api::controllers::organizations;
use bigneon_api::controllers::organizations::*;
use bigneon_db::models::*;
use chrono::NaiveDateTime;
use serde_json;
use support;
use support::database::TestDatabase;
use support::test_request::TestRequest;
use uuid::Uuid;

pub fn index(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();

    let organization = if role != Roles::User {
        database.create_organization_with_user(&user, role == Roles::OrgOwner)
    } else {
        database.create_organization()
    }.with_name("Organization 1".to_string())
    .finish();

    let organization2 = if role != Roles::User {
        database.create_organization_with_user(&user, role == Roles::OrgOwner)
    } else {
        database.create_organization()
    }.with_name("Organization 2".to_string())
    .finish();

    let expected_organizations = if role != Roles::User {
        vec![organization, organization2]
    } else {
        Vec::new()
    };
    let organization_expected_json = serde_json::to_string(&expected_organizations).unwrap();

    let user = support::create_auth_user_from_user(&user, role, &database);
    let response: HttpResponse = organizations::index((database.connection.into(), user)).into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body, organization_expected_json);
    } else {
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let temp_json = HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}));
        let organization_expected_json = support::unwrap_body_to_string(&temp_json).unwrap();
        assert_eq!(body, organization_expected_json);
    }
}

pub fn show(role: Roles, should_succeed: bool, same_organization: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = if same_organization && role != Roles::User {
        database.create_organization_with_user(&user, role == Roles::OrgOwner)
    } else {
        database.create_organization()
    }.finish();

    let organization_expected_json = serde_json::to_string(&organization).unwrap();

    let auth_user = support::create_auth_user_from_user(&user, role, &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;
    let response: HttpResponse =
        organizations::show((database.connection.into(), path, auth_user.clone())).into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, organization_expected_json);
}

pub fn index_for_all_orgs(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();

    let user = database.create_user().finish();
    let user2 = database.create_user().finish();

    let organization = database
        .create_organization()
        .with_name("Organization 1".to_string())
        .with_owner(&user)
        .finish();
    let organization2 = database
        .create_organization()
        .with_name("Organization 2".to_string())
        .with_owner(&user2)
        .finish();

    let mut expected_organizations = vec![organization, organization2];
    if role == Roles::OrgMember {
        let index = expected_organizations
            .iter()
            .position(|x| x.owner_user_id == user2.id)
            .unwrap();
        expected_organizations.remove(index);
    }
    let organization_expected_json = serde_json::to_string(&expected_organizations).unwrap();

    let auth_user = support::create_auth_user_from_user(&user, role, &database);
    let response: HttpResponse =
        organizations::index_for_all_orgs((database.connection.into(), auth_user)).into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body, organization_expected_json);
    } else {
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let temp_json = HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}));
        let organization_expected_json = support::unwrap_body_to_string(&temp_json).unwrap();
        assert_eq!(body, organization_expected_json);
    }
}

pub fn create(role: Roles, should_test_succeed: bool) {
    let database = TestDatabase::new();
    let name = "Organization Example";
    let user = database.create_user().finish();

    let auth_user = support::create_auth_user(role, &database);

    let fee_schedule = FeeSchedule::create(
        format!("Zero fees",).into(),
        vec![NewFeeScheduleRange {
            min_price: 0,
            fee: 0,
        }],
    ).commit(&*database.connection)
    .unwrap();

    let json = Json(NewOrganizationRequest {
        owner_user_id: user.id,
        name: name.clone().to_string(),
        address: None,
        city: None,
        state: None,
        postal_code: None,
        country: None,
        phone: None,
    });

    let response: HttpResponse =
        organizations::create((database.connection.into(), json, auth_user)).into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
        let org: Organization = serde_json::from_str(&body).unwrap();
        assert_eq!(org.name, name);
    } else {
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let temp_json = HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}));
        let organization_expected_json = support::unwrap_body_to_string(&temp_json).unwrap();
        assert_eq!(body, organization_expected_json);
    }
}

pub fn update(role: Roles, should_succeed: bool, same_organization: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = if same_organization && role != Roles::User {
        database.create_organization_with_user(&user, role == Roles::OrgOwner)
    } else {
        database.create_organization()
    }.finish();

    let new_name = "New Name";
    let auth_user = support::create_auth_user_from_user(&user, role, &database);
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;
    let json = Json(OrganizationEditableAttributes {
        name: Some(new_name.clone().to_string()),
        address: Some("address".to_string()),
        city: Some("city".to_string()),
        state: Some("state".to_string()),
        country: Some("country".to_string()),
        postal_code: Some("postal_code".to_string()),
        phone: Some("phone".to_string()),
        fee_schedule_id: None,
    });

    let response: HttpResponse =
        organizations::update((database.connection.into(), path, json, auth_user.clone())).into();
    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let updated_organization: Organization = serde_json::from_str(&body).unwrap();
    assert_eq!(updated_organization.name, new_name);
}

pub fn remove_user(role: Roles, should_test_succeed: bool, same_organization: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let user2 = database.create_user().finish();
    let user3 = database.create_user().finish();

    let organization = if same_organization && role != Roles::User {
        database.create_organization_with_user(&user, role == Roles::OrgOwner)
    } else {
        database.create_organization()
    }.with_user(&user2)
    .with_user(&user3)
    .finish();

    let auth_user = support::create_auth_user_from_user(&user, role, &database);
    let test_request = TestRequest::create();
    let json = Json(user3.id);
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let response: HttpResponse =
        organizations::remove_user((database.connection.into(), path, json, auth_user.clone()))
            .into();
    let count = 1;
    let body = support::unwrap_body_to_string(&response).unwrap();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::OK);
        let removed_entries: usize = serde_json::from_str(&body).unwrap();
        assert_eq!(removed_entries, count);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn add_user(role: Roles, should_test_succeed: bool, same_organization: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let user2 = database.create_user().finish();
    let organization = if same_organization && role != Roles::User {
        database.create_organization_with_user(&user, role == Roles::OrgOwner)
    } else {
        database.create_organization()
    }.finish();

    let auth_user = support::create_auth_user_from_user(&user, role, &database);
    let test_request = TestRequest::create();
    let json = Json(organizations::AddUserRequest { user_id: user2.id });
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let response: HttpResponse =
        organizations::add_user((database.connection.into(), path, json, auth_user.clone())).into();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn add_venue(role: Roles, should_test_succeed: bool, same_organization: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = if same_organization && role != Roles::User {
        database.create_organization_with_user(&user, role == Roles::OrgOwner)
    } else {
        database.create_organization()
    }.finish();

    let auth_user = support::create_auth_user_from_user(&user, role, &database);
    let test_request = TestRequest::create();
    let name = "Venue";
    let json = Json(NewVenue {
        name: name.clone().to_string(),
        ..Default::default()
    });

    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let response: HttpResponse =
        organizations::add_venue((database.connection.into(), path, json, auth_user.clone()))
            .into();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let venue: Venue = serde_json::from_str(&body).unwrap();
        assert_eq!(venue.name, name);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn add_artist(role: Roles, should_test_succeed: bool, same_organization: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = if same_organization && role != Roles::User {
        database.create_organization_with_user(&user, role == Roles::OrgOwner)
    } else {
        database.create_organization()
    }.finish();

    let auth_user = support::create_auth_user_from_user(&user, role, &database);
    let test_request = TestRequest::create();
    let name = "Artist Example";
    let bio = "Bio";
    let website_url = "http://www.example.com";

    let json = Json(NewArtist {
        name: name.to_string(),
        bio: bio.to_string(),
        website_url: Some(website_url.to_string()),
        ..Default::default()
    });

    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let response: HttpResponse =
        organizations::add_artist((database.connection.into(), path, json, auth_user.clone()))
            .into();
    if should_test_succeed {
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = support::unwrap_body_to_string(&response).unwrap();
        let artist: Artist = serde_json::from_str(&body).unwrap();
        assert_eq!(artist.name, name);
    } else {
        support::expects_unauthorized(&response);
    }
}

pub fn update_owner(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let new_owner = database.create_user().finish();
    let organization = database.create_organization().with_owner(&user).finish();

    let auth_user = support::create_auth_user_from_user(&new_owner, role, &database);
    let test_request = TestRequest::create();
    let update_owner_request = UpdateOwnerRequest {
        owner_user_id: new_owner.id,
    };

    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let json = Json(update_owner_request);

    let response: HttpResponse =
        organizations::update_owner((database.connection.into(), path, json, auth_user)).into();

    if !should_succeed {
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let updated_organization: Organization = serde_json::from_str(&body).unwrap();
    assert_eq!(updated_organization.owner_user_id, new_owner.id);
}

pub fn list_organization_members(role: Roles, should_succeed: bool, same_organization: bool) {
    let database = TestDatabase::new();
    let user1 = database
        .create_user()
        .with_last_name("User1".into())
        .finish();
    let user2 = database
        .create_user()
        .with_last_name("User2".into())
        .finish();

    let organization = if same_organization && role != Roles::User {
        database.create_organization_with_user(&user1, role == Roles::OrgOwner)
    } else {
        database.create_organization()
    }.with_user(&user2)
    .finish();

    let mut organization_members = Vec::new();

    if role != Roles::OrgOwner && same_organization {
        organization_members.push(DisplayUser::from(user1.clone()));
    }
    organization_members.push(DisplayUser::from(user2.clone()));

    let auth_user = support::create_auth_user_from_user(&user1, role, &database);

    #[derive(Serialize)]
    struct OrgOwnerMembers {
        organization_owner: DisplayUser,
        organization_members: Vec<DisplayUser>,
    }

    let expected_data = OrgOwnerMembers {
        organization_owner: if role == Roles::OrgOwner {
            DisplayUser::from(user1)
        } else {
            DisplayUser::from(organization.owner(&database.connection).unwrap())
        },
        organization_members: organization_members,
    };

    let expected_json = serde_json::to_string(&expected_data).unwrap();
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let response: HttpResponse = organizations::list_organization_members((
        database.connection.into(),
        path,
        auth_user.clone(),
    )).into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json.to_string());
}

pub fn show_fee_schedule(role: Roles, should_succeed: bool, same_organization: bool) {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let fee_schedule = database.create_fee_schedule().finish();
    let fee_schedule_ranges = fee_schedule.ranges(&database.connection);
    let organization = if same_organization && role != Roles::User {
        database.create_organization_with_user(&user, role == Roles::OrgOwner)
    } else {
        database.create_organization()
    }.with_fee_schedule(&fee_schedule)
    .finish();
    let auth_user = support::create_auth_user_from_user(&user, role, &database);

    #[derive(Serialize)]
    struct FeeScheduleWithRanges {
        id: Uuid,
        name: String,
        version: i64,
        created_at: NaiveDateTime,
        ranges: Vec<FeeScheduleRange>,
    }

    let expected_data = FeeScheduleWithRanges {
        id: fee_schedule.id,
        name: fee_schedule.name,
        version: 0,
        created_at: fee_schedule.created_at,
        ranges: fee_schedule_ranges.unwrap(),
    };

    let expected_json = serde_json::to_string(&expected_data).unwrap();
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let response: HttpResponse =
        organizations::show_fee_schedule((database.connection.into(), path, auth_user.clone()))
            .into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json.to_string());
}

pub fn add_fee_schedule(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let organization = database.create_organization().finish();

    let auth_user = support::create_auth_user(role, &database);

    let json = Json(NewFeeSchedule {
        name: "Fees".to_string(),
        ranges: vec![
            NewFeeScheduleRange {
                min_price: 20,
                fee: 10,
            },
            NewFeeScheduleRange {
                min_price: 1000,
                fee: 100,
            },
        ],
    });
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let response: HttpResponse =
        organizations::add_fee_schedule((database.connection.into(), path, json, auth_user)).into();

    if !should_succeed {
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        return;
    }
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let result: FeeScheduleWithRanges = serde_json::from_str(&body).unwrap();
    assert_eq!(result.name, "Fees".to_string());
}
