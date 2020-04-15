use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, HttpResponse};
use api::auth::TokenResponse;
use api::controllers::users;
use api::extractors::*;
use api::models::{RegisterRequest, RequestInfo, UserProfileAttributes};
use db::prelude::*;
use serde_json;
use std::collections::HashMap;

use api::errors::ApiError;

#[cfg(test)]
mod history_tests {
    use super::*;
    #[actix_rt::test]
    async fn history_org_member() {
        base::users::history(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn history_admin() {
        base::users::history(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn history_user() {
        base::users::history(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn history_org_owner() {
        base::users::history(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn history_door_person() {
        base::users::history(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn history_promoter() {
        base::users::history(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn history_promoter_read_only() {
        base::users::history(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn history_org_admin() {
        base::users::history(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn history_box_office() {
        base::users::history(Roles::OrgBoxOffice, true).await;
    }
}

#[cfg(test)]
mod activity_tests {
    use super::*;
    #[actix_rt::test]
    async fn activity_org_member() {
        base::users::activity(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn activity_admin() {
        base::users::activity(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn activity_user() {
        base::users::activity(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn activity_org_owner() {
        base::users::activity(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn activity_door_person() {
        base::users::activity(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn activity_promoter() {
        base::users::activity(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn activity_promoter_read_only() {
        base::users::activity(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn activity_org_admin() {
        base::users::activity(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn activity_box_office() {
        base::users::activity(Roles::OrgBoxOffice, true).await;
    }
}

#[cfg(test)]
mod profile_tests {
    use super::*;
    #[actix_rt::test]
    async fn profile_org_member() {
        base::users::profile(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn profile_admin() {
        base::users::profile(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn profile_user() {
        base::users::profile(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn profile_org_owner() {
        base::users::profile(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn profile_door_person() {
        base::users::profile(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn profile_promoter() {
        base::users::profile(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn profile_promoter_read_only() {
        base::users::profile(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn profile_org_admin() {
        base::users::profile(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn profile_box_office() {
        base::users::profile(Roles::OrgBoxOffice, true).await;
    }
}

#[cfg(test)]
mod user_list_organizations_tests {
    use super::*;
    #[actix_rt::test]
    async fn list_organizations_org_member() {
        base::users::list_organizations(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn list_organizations_admin() {
        base::users::list_organizations(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn list_organizations_user() {
        base::users::list_organizations(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn list_organizations_org_owner() {
        base::users::list_organizations(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn list_organizations_door_person() {
        base::users::list_organizations(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn list_organizations_promoter() {
        base::users::list_organizations(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn list_organizations_promoter_read_only() {
        base::users::list_organizations(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn list_organizations_org_admin() {
        base::users::list_organizations(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn list_organizations_box_office() {
        base::users::list_organizations(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod show_push_notification_tokens_for_user_id_tests {
    use super::*;
    #[actix_rt::test]
    async fn show_push_notification_tokens_for_user_id_org_member() {
        base::users::show_push_notification_tokens_for_user_id(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_for_user_id_admin() {
        base::users::show_push_notification_tokens_for_user_id(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_for_user_id_user() {
        base::users::show_push_notification_tokens_for_user_id(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_for_user_id_org_owner() {
        base::users::show_push_notification_tokens_for_user_id(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_for_user_id_door_person() {
        base::users::show_push_notification_tokens_for_user_id(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_for_user_id_promoter() {
        base::users::show_push_notification_tokens_for_user_id(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_for_user_id_promoter_read_only() {
        base::users::show_push_notification_tokens_for_user_id(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_for_user_id_org_admin() {
        base::users::show_push_notification_tokens_for_user_id(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_for_user_id_box_office() {
        base::users::show_push_notification_tokens_for_user_id(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod show_push_notification_tokens_tests {
    use super::*;
    #[actix_rt::test]
    async fn show_push_notification_tokens_org_member() {
        base::users::show_push_notification_tokens(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_admin() {
        base::users::show_push_notification_tokens(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_user() {
        base::users::show_push_notification_tokens(Roles::User, true).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_org_owner() {
        base::users::show_push_notification_tokens(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_door_person() {
        base::users::show_push_notification_tokens(Roles::DoorPerson, true).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_promoter() {
        base::users::show_push_notification_tokens(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_promoter_read_only() {
        base::users::show_push_notification_tokens(Roles::PromoterReadOnly, true).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_org_admin() {
        base::users::show_push_notification_tokens(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn show_push_notification_tokens_box_office() {
        base::users::show_push_notification_tokens(Roles::OrgBoxOffice, true).await;
    }
}

#[cfg(test)]
mod add_push_notification_token_tests {
    use super::*;
    #[actix_rt::test]
    async fn add_push_notification_token_org_member() {
        base::users::add_push_notification_token(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn add_push_notification_token_admin() {
        base::users::add_push_notification_token(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn add_push_notification_token_user() {
        base::users::add_push_notification_token(Roles::User, true).await;
    }
    #[actix_rt::test]
    async fn add_push_notification_token_org_owner() {
        base::users::add_push_notification_token(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn add_push_notification_token_door_person() {
        base::users::add_push_notification_token(Roles::DoorPerson, true).await;
    }
    #[actix_rt::test]
    async fn add_push_notification_token_promoter() {
        base::users::add_push_notification_token(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn add_push_notification_token_promoter_read_only() {
        base::users::add_push_notification_token(Roles::PromoterReadOnly, true).await;
    }
    #[actix_rt::test]
    async fn add_push_notification_token_org_admin() {
        base::users::add_push_notification_token(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn add_push_notification_token_box_office() {
        base::users::add_push_notification_token(Roles::OrgBoxOffice, true).await;
    }
}

#[cfg(test)]
mod remove_push_notification_token_tests {
    use super::*;
    #[actix_rt::test]
    async fn remove_push_notification_token_org_member() {
        base::users::remove_push_notification_token(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn remove_push_notification_token_admin() {
        base::users::remove_push_notification_token(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn remove_push_notification_token_user() {
        base::users::remove_push_notification_token(Roles::User, true).await;
    }
    #[actix_rt::test]
    async fn remove_push_notification_token_owner() {
        base::users::remove_push_notification_token(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn remove_push_notification_token_door_person() {
        base::users::remove_push_notification_token(Roles::DoorPerson, true).await;
    }
    #[actix_rt::test]
    async fn remove_push_notification_token_promoter() {
        base::users::remove_push_notification_token(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn remove_push_notification_token_promoter_read_only() {
        base::users::remove_push_notification_token(Roles::PromoterReadOnly, true).await;
    }
    #[actix_rt::test]
    async fn remove_push_notification_token_org_admin() {
        base::users::remove_push_notification_token(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn remove_push_notification_token_box_office() {
        base::users::remove_push_notification_token(Roles::OrgBoxOffice, true).await;
    }
}

#[cfg(test)]
mod users_show_tests {
    use super::*;
    #[actix_rt::test]
    async fn show_org_member() {
        base::users::show(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn show_admin() {
        base::users::show(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn show_user() {
        base::users::show(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn show_org_owner() {
        base::users::show(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn show_door_person() {
        base::users::show(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn show_promoter() {
        base::users::show(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn show_promoter_read_only() {
        base::users::show(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn show_org_admin() {
        base::users::show(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn show_box_office() {
        base::users::show(Roles::OrgBoxOffice, false).await;
    }
}

#[actix_rt::test]
async fn register_address_exists() {
    let database = TestDatabase::new();
    let request = TestRequest::create();
    let existing_user = database.create_user().finish();

    let json = Json(RegisterRequest::new(
        &"First",
        &"Last",
        &existing_user.email.unwrap(),
        &"555",
        &"not_important",
        None,
    ));

    let response: HttpResponse = users::register((request.request, database.connection.into(), json))
        .await
        .into();

    if response.status() == StatusCode::OK {
        panic!("Duplicate email was allowed when it should not be")
    }
}

#[actix_rt::test]
async fn register_succeeds_without_name() {
    let database = TestDatabase::new();
    let request = TestRequest::create();
    let json = Json(RegisterRequest {
        email: "noname@localhost".to_string(),
        password: "password".to_string(),
        first_name: None,
        last_name: None,
        phone: None,
        captcha_response: None,
    });

    let response: HttpResponse = users::register((request.request, database.connection.into(), json))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[actix_rt::test]
async fn register_succeeds() {
    let database = TestDatabase::new();
    let request = TestRequest::create();
    let json = Json(RegisterRequest::new(
        &"First",
        &"Last",
        &"fake@localhost",
        &"555",
        &"not_important",
        None,
    ));

    let response: HttpResponse = users::register((request.request, database.connection.into(), json))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[actix_rt::test]
async fn register_succeeds_with_login() {
    let database = TestDatabase::new();
    let request = TestRequest::create();
    let json = Json(RegisterRequest::new(
        &"First",
        &"Last",
        &"fake@localhost",
        &"555",
        &"not_important",
        None,
    ));

    let response: HttpResponse = users::register_and_login((
        request.request,
        database.connection.into(),
        json,
        RequestInfo { user_agent: None },
    ))
    .await
    .into();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let token_response: TokenResponse = serde_json::from_str(&body).unwrap();
    assert_eq!(token_response.access_token.is_empty(), false);
    assert_eq!(token_response.refresh_token.is_empty(), false);
}

#[actix_rt::test]
async fn register_with_validation_errors() {
    let database = TestDatabase::new();
    let request = TestRequest::create();
    let json = Json(RegisterRequest::new(
        &"First",
        &"Last",
        &"bad-email",
        &"555",
        &"not_important",
        None,
    ));

    let response: HttpResponse = users::register((request.request, database.connection.into(), json))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());

    let validation_response = support::validation_response_from_response(&response).unwrap();
    let email = validation_response.fields.get("email").unwrap();
    assert_eq!(email[0].code, "email");
    assert_eq!(&email[0].message.clone().unwrap().into_owned(), "Email is invalid");
}

#[actix_rt::test]
async fn current_user() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::User, None, &database);

    let response = users::current_user((database.connection.into(), auth_user))
        .await
        .unwrap();
    let user = response.user;
    assert_eq!(user.id, user.id);
    assert_eq!(
        vec![
            Scopes::CollectionRead,
            Scopes::CollectionWrite,
            Scopes::EventInterest,
            Scopes::ListingWrite,
            Scopes::OrderReadOwn,
            Scopes::TransferCancelOwn,
            Scopes::TransferReadOwn,
            Scopes::TicketWriteOwn,
            Scopes::TicketTransfer
        ],
        response.scopes
    );
    assert!(response.organization_scopes.is_empty());
}

#[actix_rt::test]
async fn current_user_organization_owner() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);

    let current_user = users::current_user((database.connection.clone().into(), auth_user))
        .await
        .unwrap();
    let user = current_user.user;
    assert_eq!(user.id, user.id);
    let mut expected_results = HashMap::new();
    expected_results.insert(
        organization.id,
        vec![
            "announcement-engagement:write",
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "collection:read",
            "collection:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:broadcast",
            "event:cancel",
            "event:clone",
            "event:data-read",
            "event:delete",
            "event:financial-reports",
            "event:interest",
            "event-report-subscriber:delete",
            "event-report-subscriber:read",
            "event-report-subscriber:write",
            "event:reports",
            "event:scan",
            "event:view-guests",
            "event:write",
            "hold:read",
            "hold:write",
            "listing:write",
            "loot-box:write",
            "note:delete",
            "note:read",
            "note:write",
            "order:make-external-payment",
            "order:read",
            "order:read-own",
            "order:refund",
            "order:resend-confirmation",
            "org:admin-users",
            "org:fans",
            "org:read",
            "org:read-events",
            "org:reports",
            "org:users",
            "org:write",
            "rarity:write",
            "redeem:ticket",
            "sales-summary-report:read",
            "scan-report:read",
            "settlement:read",
            "transfer:cancel",
            "transfer:cancel-own",
            "transfer:read",
            "transfer:read-own",
            "ticket:admin",
            "ticket:read",
            "ticket:write",
            "ticket:write-own",
            "ticket:transfer",
            "ticket-type:read",
            "ticket-type:write",
            "user:read",
            "venue:write",
            "websocket:initiate",
        ]
        .into_iter()
        .map(|scope| scope.parse::<Scopes>().unwrap())
        .collect(),
    );
    assert_eq!(expected_results, current_user.organization_scopes);

    let mut expected_roles = HashMap::new();
    expected_roles.insert(organization.id, vec![Roles::OrgOwner]);
    assert_eq!(expected_roles, current_user.organization_roles);
}

#[actix_rt::test]
async fn current_user_organization_member() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgMember, Some(&organization), &database);

    let current_user = users::current_user((database.connection.clone().into(), auth_user))
        .await
        .unwrap();
    let user = current_user.user;
    assert_eq!(user.id, user.id);
    assert_eq!(
        vec![
            Scopes::CollectionRead,
            Scopes::CollectionWrite,
            Scopes::EventInterest,
            Scopes::ListingWrite,
            Scopes::OrderReadOwn,
            Scopes::TransferCancelOwn,
            Scopes::TransferReadOwn,
            Scopes::TicketWriteOwn,
            Scopes::TicketTransfer
        ],
        current_user.scopes
    );
    let mut expected_scopes = HashMap::new();
    expected_scopes.insert(
        organization.id,
        vec![
            "announcement-engagement:write",
            "artist:write",
            "box-office-ticket:read",
            "box-office-ticket:write",
            "code:read",
            "code:write",
            "collection:read",
            "collection:write",
            "comp:read",
            "comp:write",
            "dashboard:read",
            "event:broadcast",
            "event:cancel",
            "event:clone",
            "event:delete",
            "event:interest",
            "event-report-subscriber:delete",
            "event-report-subscriber:read",
            "event-report-subscriber:write",
            "event:scan",
            "event:view-guests",
            "event:write",
            "hold:read",
            "hold:write",
            "listing:write",
            "loot-box:write",
            "note:read",
            "note:write",
            "order:read",
            "order:read-own",
            "order:refund",
            "order:resend-confirmation",
            "org:fans",
            "org:read",
            "org:read-events",
            "rarity:write",
            "redeem:ticket",
            "sales-summary-report:read",
            "scan-report:read",
            "transfer:cancel",
            "transfer:cancel-own",
            "transfer:read",
            "transfer:read-own",
            "ticket:admin",
            "ticket:read",
            "ticket:write-own",
            "ticket:transfer",
            "ticket-type:read",
            "ticket-type:write",
            "venue:write",
            "websocket:initiate",
        ]
        .into_iter()
        .map(|scope| scope.parse::<Scopes>().unwrap())
        .collect(),
    );
    assert_eq!(expected_scopes, current_user.organization_scopes);

    let mut expected_roles = HashMap::new();
    expected_roles.insert(organization.id, vec![Roles::OrgMember]);
    assert_eq!(expected_roles, current_user.organization_roles);
}

#[actix_rt::test]
pub async fn update_current_user() {
    let database = TestDatabase::new();
    let user = support::create_auth_user(Roles::User, None, &database);
    let email = "new-email@tari.com";
    let mut attributes: UserProfileAttributes = Default::default();
    attributes.email = Some(email.to_string());
    let json = Json(attributes);

    let updated_user = users::update_current_user((database.connection.into(), json, user))
        .await
        .unwrap();
    assert_eq!(updated_user.user.email, Some(email.into()));
}

#[actix_rt::test]
pub async fn update_current_user_with_validation_errors() {
    let database = TestDatabase::new();
    let user = support::create_auth_user(Roles::User, None, &database);
    let mut attributes: UserProfileAttributes = Default::default();
    attributes.email = Some("bad-email".into());
    let json = Json(attributes);

    let result: Result<HttpResponse, ApiError> =
        Err(users::update_current_user((database.connection.into(), json, user))
            .await
            .err()
            .unwrap());

    let response: HttpResponse = result.into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let validation_response = support::validation_response_from_response(&response).unwrap();
    let email = validation_response.fields.get("email").unwrap();
    assert_eq!(email[0].code, "email");
    assert_eq!(&email[0].message.clone().unwrap().into_owned(), "Email is invalid");
}

#[actix_rt::test]
async fn update_current_user_address_exists() {
    let database = TestDatabase::new();
    let existing_user = database.create_user().finish();

    let user = support::create_auth_user(Roles::User, None, &database);
    let mut attributes: UserProfileAttributes = Default::default();
    attributes.email = existing_user.email;
    let json = Json(attributes);

    let result: Result<HttpResponse, ApiError> =
        Err(users::update_current_user((database.connection.into(), json, user))
            .await
            .err()
            .unwrap());
    let response: HttpResponse = result.into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let validation_response = support::validation_response_from_response(&response).unwrap();
    let email = validation_response.fields.get("email").unwrap();
    assert_eq!(email[0].code, "uniqueness");
    assert_eq!(
        &email[0].message.clone().unwrap().into_owned(),
        "Email is already in use"
    );
}
