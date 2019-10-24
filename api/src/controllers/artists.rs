use actix_web::{http::StatusCode, web::Path, web::Query, HttpResponse};
use auth::user::User;
use bigneon_db::models::*;
use db::Connection;
use domain_events::executors::UpdateGenresPayload;
use errors::*;
use extractors::*;
use helpers::application;
use models::{CreateArtistRequest, PathParameters, UpdateArtistRequest, WebPayload};
use utils::spotify;

pub fn search(
    connection: Connection,
    query_parameters: Query<PagingParameters>,
    user: OptionalUser,
) -> Result<WebPayload<CreateArtistRequest>, BigNeonError> {
    let connection = connection.get();
    let db_user = user.into_inner().map(|u| u.user);
    let artists = Artist::search(&db_user, query_parameters.get_tag("q"), connection)?;

    let try_spotify = query_parameters
        .get_tag("spotify")
        .map(|spotify| spotify != "0")
        .unwrap_or(false);
    if try_spotify && artists.is_empty() && query_parameters.get_tag("q").is_some() {
        //Try spotify
        let spotify_client = &spotify::SINGLETON;
        let spotify_artists =
            spotify_client.search(query_parameters.get_tag("q").unwrap_or("".to_string()))?;

        let payload = Payload::new(spotify_artists, query_parameters.into_inner().into());
        Ok(WebPayload::new(StatusCode::OK, payload))
    } else {
        let wrapper = artists
            .into_iter()
            .map(|a| CreateArtistRequest::from(a))
            .collect();
        let payload = Payload::new(wrapper, query_parameters.into_inner().into());
        Ok(WebPayload::new(StatusCode::OK, payload))
    }
}

pub fn index(
    connection: Connection,
    query_parameters: Query<PagingParameters>,
    user: OptionalUser,
) -> Result<HttpResponse, BigNeonError> {
    let db_user = user.into_inner().map(|u| u.user);
    let artists = Artist::search(&db_user, query_parameters.get_tag("q"), connection.get())?;
    let payload = Payload::from_data(artists, query_parameters.page(), query_parameters.limit());
    Ok(HttpResponse::Ok().json(&payload))
}

pub fn show(
    connection: Connection,
    parameters: Path<PathParameters>,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let artist = Artist::find(&parameters.id, connection)?.for_display(connection)?;
    Ok(HttpResponse::Ok().json(&artist))
}

pub fn create(
    connection: Connection,
    json_create_artist: Json<CreateArtistRequest>,
    user: User,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    if let Some(organization_id) = json_create_artist.organization_id {
        let organization = Organization::find(organization_id, connection)?;
        user.requires_scope_for_organization(Scopes::ArtistWrite, &organization, connection)?;
    } else {
        user.requires_scope(Scopes::ArtistWrite)?;
    }

    let create_artist = json_create_artist.into_inner();
    let mut genres = create_artist.genres.clone().unwrap_or(Vec::new());
    let mut artist = match &create_artist.spotify_id {
        Some(spotify_id) => {
            let spotify_client = &spotify::SINGLETON;

            let spotify_artist_result = spotify_client.read_artist(&spotify_id)?;
            match spotify_artist_result {
                Some(artist) => {
                    let mut new_artist: NewArtist = artist.clone().into();
                    let client_data: NewArtist = create_artist.clone().into();
                    if let Some(mut spotify_genres) = artist.genres {
                        genres.append(&mut spotify_genres);
                    }
                    new_artist.merge(client_data);
                    new_artist.commit(connection)?
                }
                None => return application::not_found(),
            }
        }
        None => {
            let new_artist: NewArtist = create_artist.clone().into();
            new_artist.commit(connection)?
        }
    };

    artist.set_genres(&genres, Some(user.id()), connection)?;

    // Trigger update for associated genres (events and users with tickets)
    let action = DomainAction::create(
        None,
        DomainActionTypes::UpdateGenres,
        None,
        json!(UpdateGenresPayload { user_id: user.id() }),
        Some(Tables::Artists),
        Some(artist.id),
    );
    action.commit(connection)?;

    // New artists belonging to an organization start private
    if artist.organization_id.is_some() {
        artist = artist.set_privacy(true, connection)?;
    }
    Ok(HttpResponse::Created().json(&artist.for_display(connection)?))
}

pub fn show_from_organizations(
    connection: Connection,
    organization_id: Path<PathParameters>,
    query_parameters: Query<PagingParameters>,
    user: OptionalUser,
) -> Result<HttpResponse, BigNeonError> {
    //TODO implement proper paging on db
    let artists = match user.into_inner() {
        Some(u) => {
            Artist::find_for_organization(Some(&u.user), organization_id.id, connection.get())?
        }
        None => Artist::find_for_organization(None, organization_id.id, connection.get())?,
    };
    let payload = Payload::from_data(artists, query_parameters.page(), query_parameters.limit());
    Ok(HttpResponse::Ok().json(&payload))
}

pub fn update(
    connection: Connection,
    parameters: Path<PathParameters>,
    artist_parameters: Json<UpdateArtistRequest>,
    user: User,
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let artist_parameters = artist_parameters.into_inner();
    let artist = Artist::find(&parameters.id, connection)?;
    if !artist.is_private || artist.organization_id.is_none() {
        user.requires_scope(Scopes::ArtistWrite)?;
    } else {
        let organization = artist.organization(connection)?.unwrap();
        user.requires_scope_for_organization(Scopes::ArtistWrite, &organization, connection)?;
    }

    let genres = artist_parameters.genres.clone();
    let main_genre = artist_parameters.main_genre.clone();
    let mut attr: ArtistEditableAttributes = artist_parameters.into();
    attr.main_genre_id = match main_genre {
        Some(g) => match g {
            Some(g) => Some(Genre::find_or_create(&vec![g], connection)?.pop()),
            None => Some(None),
        },
        None => None,
    };

    let updated_artist = artist.update(&attr, connection)?;

    if let Some(genres) = genres {
        artist.set_genres(&genres, Some(user.id()), connection)?;

        let action = DomainAction::create(
            None,
            DomainActionTypes::UpdateGenres,
            None,
            json!(UpdateGenresPayload { user_id: user.id() }),
            Some(Tables::Artists),
            Some(artist.id),
        );
        action.commit(connection)?;
    }

    Ok(HttpResponse::Ok().json(&updated_artist.for_display(connection)?))
}

pub fn toggle_privacy(
    connection: Connection,
    parameters: Path<PathParameters>,
    user: User,
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::ArtistWrite)?;

    let connection = connection.get();
    let artist = Artist::find(&parameters.id, connection)?;
    let updated_artist = artist.set_privacy(!artist.is_private, connection)?;
    Ok(HttpResponse::Ok().json(&updated_artist.for_display(connection)?))
}
