use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path, Query};
use bigneon_api::controllers::venues;
use bigneon_api::models::{Paging, PagingParameters, PathParameters, Payload, SortingDir};
use bigneon_db::models::Roles;
use functional::base;
use serde_json;
use support;
use support::database::TestDatabase;
use support::test_request::TestRequest;

#[test]
fn index_with_org_linked_and_private_venues() {
    let database = TestDatabase::new();
    let venue = database
        .create_venue()
        .with_name("Venue1".to_string())
        .finish();
    let venue2 = database
        .create_venue()
        .with_name("Venue2".to_string())
        .finish();

    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user(Roles::User, Some(&organization), &database);
    let venue3 = database
        .create_venue()
        .with_name("Venue3".to_string())
        .finish();
    let venue3 = venue3
        .add_to_organization(&organization.id, &database.connection)
        .unwrap();

    let venue4 = database
        .create_venue()
        .make_private()
        .with_name("Venue4".to_string())
        .finish();
    let venue4 = venue4
        .add_to_organization(&organization.id, &database.connection)
        .unwrap();
    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters =
        Query::<PagingParameters>::from_request(&test_request.request, &()).unwrap();
    //first try with no user
    let response: HttpResponse =
        venues::index((database.connection.clone().into(), query_parameters, None)).into();

    let mut expected_venues = vec![venue, venue2, venue3];
    let wrapped_expected_venues = Payload {
        data: expected_venues.clone(),
        paging: Paging {
            page: 0,
            limit: 3,
            sort: "".to_string(),
            dir: SortingDir::None,
            total: 3,
            tags: Vec::new(),
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_venues).unwrap();

    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);
    let query_parameters =
        Query::<PagingParameters>::from_request(&test_request.request, &()).unwrap();
    //now try with user that does not belong to org
    let response: HttpResponse = venues::index((
        database.connection.clone().into(),
        query_parameters,
        Some(auth_user.clone()),
    )).into();

    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);

    //now with user that DOES belong to org
    let _ = organization.add_user(auth_user.id(), &database.connection.clone());
    expected_venues.push(venue4);

    let query_parameters =
        Query::<PagingParameters>::from_request(&test_request.request, &()).unwrap();
    let response: HttpResponse = venues::index((
        database.connection.into(),
        query_parameters,
        Some(auth_user),
    )).into();
    let wrapped_expected_venues = Payload {
        data: expected_venues,
        paging: Paging {
            page: 0,
            limit: 4,
            sort: "".to_string(),
            dir: SortingDir::None,
            total: 4,
            tags: Vec::new(),
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_venues).unwrap();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);
}

#[test]
pub fn show() {
    let database = TestDatabase::new();
    let venue = database.create_venue().finish();
    let venue_expected_json = serde_json::to_string(&venue).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = venue.id;

    let response: HttpResponse = venues::show((database.connection.into(), path)).into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, venue_expected_json);
}

#[cfg(test)]
mod index_tests {
    use super::*;
    #[test]
    fn index_org_member() {
        base::venues::index(Roles::OrgMember, true);
    }
    #[test]
    fn index_admin() {
        base::venues::index(Roles::Admin, true);
    }
    #[test]
    fn index_user() {
        base::venues::index(Roles::User, true);
    }
    #[test]
    fn index_org_owner() {
        base::venues::index(Roles::OrgOwner, true);
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        base::venues::create(Roles::OrgMember, false);
    }
    #[test]
    fn create_admin() {
        base::venues::create(Roles::Admin, true);
    }
    #[test]
    fn create_user() {
        base::venues::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        base::venues::create(Roles::OrgOwner, false);
    }
    #[test]
    fn create_with_organization_org_member() {
        base::venues::create_with_organization(Roles::OrgMember, true);
    }
    #[test]
    fn create_with_organization_admin() {
        base::venues::create_with_organization(Roles::Admin, true);
    }
    #[test]
    fn create_with_organization_user() {
        base::venues::create_with_organization(Roles::User, false);
    }
    #[test]
    fn create_with_organization_org_owner() {
        base::venues::create_with_organization(Roles::OrgOwner, true);
    }
}

#[cfg(test)]
mod toggle_privacy_tests {
    use super::*;
    #[test]
    fn toggle_privacy_org_member() {
        base::venues::toggle_privacy(Roles::OrgMember, false);
    }
    #[test]
    fn toggle_privacy_admin() {
        base::venues::toggle_privacy(Roles::Admin, true);
    }
    #[test]
    fn toggle_privacy_user() {
        base::venues::toggle_privacy(Roles::User, false);
    }
    #[test]
    fn toggle_privacy_org_owner() {
        base::venues::toggle_privacy(Roles::OrgOwner, false);
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[test]
    fn update_org_member() {
        base::venues::update(Roles::OrgMember, false);
    }
    #[test]
    fn update_admin() {
        base::venues::update(Roles::Admin, true);
    }
    #[test]
    fn update_user() {
        base::venues::update(Roles::User, false);
    }
    #[test]
    fn update_org_owner() {
        base::venues::update(Roles::OrgOwner, false);
    }
    #[test]
    fn update_with_organization_org_member() {
        base::venues::update_with_organization(Roles::OrgMember, true, true);
    }
    #[test]
    fn update_with_organization_admin() {
        base::venues::update_with_organization(Roles::Admin, true, true);
    }
    #[test]
    fn update_with_organization_user() {
        base::venues::update_with_organization(Roles::User, false, true);
    }
    #[test]
    fn update_with_organization_org_owner() {
        base::venues::update_with_organization(Roles::OrgOwner, true, true);
    }
    #[test]
    fn update_public_venue_with_organization_org_member() {
        base::venues::update_with_organization(Roles::OrgMember, false, false);
    }
    #[test]
    fn update_public_venue_with_organization_admin() {
        base::venues::update_with_organization(Roles::Admin, true, false);
    }
    #[test]
    fn update_public_venue_with_organization_user() {
        base::venues::update_with_organization(Roles::User, false, false);
    }
    #[test]
    fn update_public_venue_with_organization_org_owner() {
        base::venues::update_with_organization(Roles::OrgOwner, false, false);
    }
}

#[cfg(test)]
mod show_from_organizations_tests {
    use super::*;
    #[test]
    fn show_from_organizations_org_member() {
        base::venues::show_from_organizations(Some(Roles::OrgMember), true);
    }
    #[test]
    fn show_from_organizations_admin() {
        base::venues::show_from_organizations(Some(Roles::Admin), true);
    }
    #[test]
    fn show_from_organizations_user() {
        base::venues::show_from_organizations(Some(Roles::User), true);
    }
    #[test]
    fn show_from_organizations_org_owner() {
        base::venues::show_from_organizations(Some(Roles::OrgOwner), true);
    }
    #[test]
    fn show_from_organizations_no_user() {
        base::venues::show_from_organizations(None, true);
    }
}

#[test]
pub fn show_from_organizations_private_venue_same_org() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().with_user(&user).finish();
    let venue = database
        .create_venue()
        .with_name("Venue 1".to_string())
        .finish();
    let venue2 = database
        .create_venue()
        .with_name("Venue 2".to_string())
        .make_private()
        .finish();
    let venue = venue
        .add_to_organization(&organization.id, &database.connection)
        .unwrap();
    let venue2 = venue2
        .add_to_organization(&organization.id, &database.connection)
        .unwrap();

    let user2 = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(
        &user2,
        Roles::OrgOwner,
        Some(&organization),
        &database,
    );

    let all_venues = vec![venue, venue2];
    let wrapped_expected_venues = Payload {
        data: all_venues,
        paging: Paging {
            page: 0,
            limit: 2,
            sort: "".to_string(),
            dir: SortingDir::None,
            total: 2,
            tags: Vec::new(),
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_venues).unwrap();

    let test_request = TestRequest::create();

    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters =
        Query::<PagingParameters>::from_request(&test_request.request, &()).unwrap();
    let response: HttpResponse = venues::show_from_organizations((
        database.connection.into(),
        path,
        query_parameters,
        Some(auth_user),
    )).into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(expected_json, body);
}

#[cfg(test)]
mod add_to_organization_tests {
    use super::*;
    #[test]
    fn add_to_organization_org_member() {
        base::venues::add_to_organization(Roles::OrgMember, false);
    }
    #[test]
    fn add_to_organization_admin() {
        base::venues::add_to_organization(Roles::Admin, true);
    }
    #[test]
    fn add_to_organization_user() {
        base::venues::add_to_organization(Roles::User, false);
    }
    #[test]
    fn add_to_organization_org_owner() {
        base::venues::add_to_organization(Roles::OrgOwner, false);
    }
}

#[cfg(test)]
mod add_to_organization_where_link_already_exists_tests {
    use super::*;
    #[test]
    fn add_to_organization_where_link_already_exists_org_member() {
        base::venues::add_to_organization_where_link_already_exists(Roles::OrgMember, false);
    }
    #[test]
    fn add_to_organization_where_link_already_exists_admin() {
        base::venues::add_to_organization_where_link_already_exists(Roles::Admin, true);
    }
    #[test]
    fn add_to_organization_where_link_already_exists_user() {
        base::venues::add_to_organization_where_link_already_exists(Roles::User, false);
    }
    #[test]
    fn add_to_organization_where_link_already_exists_org_owner() {
        base::venues::add_to_organization_where_link_already_exists(Roles::OrgOwner, false);
    }
}
