use actix_web::{HttpResponse, Path, Query, State};
use bigneon_db::prelude::*;
use controllers::events::{self, *};
use db::ReadonlyConnection;
use errors::*;
use extractors::*;
use helpers::application;
use models::*;
use server::AppState;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct CityData {
    pub city: String,
    pub state: String,
    pub country: String,
    pub google_place_id: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: String,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum SlugResponse {
    Organization {
        organization: DisplayOrganization,
        events: Vec<EventVenueEntry>,
    },
    City {
        city: CityData,
        events: Vec<EventVenueEntry>,
    },
    Venue {
        venue: DisplayVenue,
        events: Vec<EventVenueEntry>,
    },
}

pub fn show(
    (state, conn, mut parameters, query, auth_user, request): (
        State<AppState>,
        ReadonlyConnection,
        Path<StringPathParameters>,
        Query<EventParameters>,
        OptionalUser,
        RequestInfo,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let user = auth_user
        .clone()
        .into_inner()
        .and_then(|auth_user| Some(auth_user.user));
    let connection = conn.clone();
    let connection = connection.get();
    let slugs = Slug::find_by_slug(&parameters.id, connection)?;
    if slugs.is_empty() {
        return application::not_found();
    }

    let slug = &slugs[0];
    match slug.slug_type {
        SlugTypes::Organization | SlugTypes::Venue | SlugTypes::Event => {
            let primary_slug = Slug::primary_slug(slug.main_table_id, slug.main_table, connection)?;
            if primary_slug.slug != slug.slug {
                return redirection_json(&primary_slug, state);
            }
        }
        _ => (),
    }

    let response = match slug.slug_type {
        SlugTypes::Event => {
            parameters.id = slug.main_table_id.to_string();
            return events::show((state, conn, parameters, query, auth_user, request));
        }
        SlugTypes::Organization => {
            let organization = Organization::find(slug.main_table_id, connection)?;

            let (events, _) = Event::search(
                None,
                None,
                Some(organization.id),
                None,
                None,
                None,
                None,
                None,
                EventSearchSortField::EventStart,
                SortingDir::Asc,
                user.clone(),
                PastOrUpcoming::Upcoming,
                None,
                &Paging::new(0, std::u32::MAX),
                state.service_locator.country_lookup_service(),
                connection,
            )?;

            let events =
                EventVenueEntry::event_venues_from_events(events, user, &state, connection)?;
            SlugResponse::Organization {
                organization: organization.for_display(connection)?,
                events,
            }
        }
        SlugTypes::Venue => {
            let venue = Venue::find(slug.main_table_id, connection)?;

            let (events, _) = Event::search(
                None,
                None,
                None,
                Some(vec![venue.id]),
                None,
                None,
                None,
                None,
                EventSearchSortField::EventStart,
                SortingDir::Asc,
                user.clone(),
                PastOrUpcoming::Upcoming,
                None,
                &Paging::new(0, std::u32::MAX),
                state.service_locator.country_lookup_service(),
                connection,
            )?;

            let events =
                EventVenueEntry::event_venues_from_events(events, user, &state, connection)?;
            SlugResponse::Venue {
                venue: venue.for_display(connection)?,
                events,
            }
        }
        SlugTypes::City => {
            let venue = Venue::find(slug.main_table_id, connection)?;
            let city = CityData {
                city: venue.city,
                state: venue.state,
                country: venue.country,
                google_place_id: venue.google_place_id,
                latitude: venue.latitude,
                longitude: venue.longitude,
                timezone: venue.timezone,
            };

            let (events, _) = Event::search(
                None,
                None,
                None,
                Some(slugs.iter().map(|s| s.main_table_id).collect()),
                None,
                None,
                None,
                None,
                EventSearchSortField::EventStart,
                SortingDir::Asc,
                user.clone(),
                PastOrUpcoming::Upcoming,
                None,
                &Paging::new(0, std::u32::MAX),
                state.service_locator.country_lookup_service(),
                connection,
            )?;

            let events =
                EventVenueEntry::event_venues_from_events(events, user, &state, connection)?;
            SlugResponse::City { city, events }
        }
    };

    Ok(HttpResponse::Ok().json(&response))
}

fn redirection_json(slug: &Slug, state: State<AppState>) -> Result<HttpResponse, BigNeonError> {
    let path = match slug.slug_type {
        SlugTypes::Event => "tickets",
        SlugTypes::Venue => "venues",
        SlugTypes::Organization => "organizations",
        _ => return application::bad_request("Slug type is not valid for redirection"),
    };

    Ok(HttpResponse::Ok().json(json!({
        "redirect": format!("{}/{}/{}", &state.config.front_end_url, path, &slug.slug)
    })))
}
