use actix_web::http::header::*;
use actix_web::http::{HttpTryFrom, Method, StatusCode};
use actix_web::middleware::{Middleware, Response, Started};
use actix_web::{Body, FromRequest, HttpRequest, HttpResponse, Result};
use bigneon_http::caching::*;
use extractors::*;
use helpers::*;
use serde_json::Value;
use server::AppState;
use std::collections::BTreeMap;
use uuid::Uuid;

const CACHED_RESPONSE_HEADER: &'static str = "X-Cached-Response";

pub struct CacheResource {}

impl CacheResource {
    pub fn new() -> CacheResource {
        CacheResource {}
    }
}

pub struct CacheConfiguration {
    cache_response: bool,
    served_cache: bool,
    error: bool,
    user_id: Option<Uuid>,
}

impl CacheConfiguration {
    pub fn new() -> CacheConfiguration {
        CacheConfiguration {
            cache_response: false,
            served_cache: false,
            error: false,
            user_id: None,
        }
    }
}

impl Middleware<AppState> for CacheResource {
    fn start(&self, request: &HttpRequest<AppState>) -> Result<Started> {
        let mut cache_configuration = CacheConfiguration::new();
        if request.method() == Method::GET {
            let mut query = BTreeMap::new();
            let resource = request.resource().clone();
            let query_parameters = request.query().clone();
            for (key, value) in query_parameters.iter() {
                query.insert(key, value);
            }
            let resource_def = resource.rdef().clone();
            if resource_def.is_none() {
                return Ok(Started::Done);
            }
            let path = resource_def.unwrap().pattern().to_string();
            let path_text = "path".to_string();
            let method = request.method().to_string();
            let method_text = "method".to_string();
            let user_text = "x-user-id".to_string();
            query.insert(&path_text, &path);
            query.insert(&method_text, &method);
            let state = request.state().clone();
            let config = state.config.clone();
            let mut user_value = "".to_string();
            match OptionalUser::from_request(request, &()) {
                Ok(user) => {
                    cache_configuration.user_id = user.0.map(|u| u.id());
                    if let Some(user_id) = cache_configuration.user_id {
                        user_value = user_id.to_string();
                    }
                    query.insert(&user_text, &user_value);
                }
                Err(error) => {
                    cache_configuration.error = true;
                    error!("CacheResource Middleware start: {:?}", error);
                    request.extensions_mut().insert(cache_configuration);
                    return Ok(Started::Done);
                }
            }

            let cache_database = state.database.cache_database.clone();
            // if there is a error in the cache, the value does not exist
            let cached_value = cache_database
                .clone()
                .inner
                .clone()
                .and_then(|conn| caching::get_cached_value(conn, &config, &query));
            if let Some(response) = cached_value {
                // Insert self into extensions to let response know not to set the value
                cache_configuration.served_cache = true;
                request.extensions_mut().insert(cache_configuration);
                return Ok(Started::Response(response));
            }
        }

        cache_configuration.cache_response = true;
        request.extensions_mut().insert(cache_configuration);
        Ok(Started::Done)
    }

    fn response(&self, request: &HttpRequest<AppState>, mut response: HttpResponse) -> Result<Response> {
        if request.method() == Method::GET {
            let extensions = request.extensions();
            if let Some(cache_configuration) = extensions.get::<CacheConfiguration>() {
                let state = request.state().clone();
                let config = state.config.clone();

                if cache_configuration.cache_response {
                    let cache_database = state.database.cache_database.clone();
                    let mut query = BTreeMap::new();
                    let resource = request.resource().clone();
                    let query_parameters = request.query();
                    for (key, value) in query_parameters.iter() {
                        query.insert(key, value);
                    }
                    let resource_def = resource.rdef().clone();
                    if resource_def.is_none() {
                        return Ok(Response::Done(response));
                    }
                    let path = resource_def.unwrap().pattern().to_string();
                    let path_text = "path".to_string();
                    let method = request.method().to_string();
                    let method_text = "method".to_string();
                    let user_text = "x-user-id".to_string();
                    let user_id = cache_configuration
                        .user_id
                        .map(|u| u.to_string())
                        .unwrap_or("".to_string());
                    query.insert(&path_text, &path);
                    query.insert(&method_text, &method);
                    query.insert(&user_text, &user_id);

                    cache_database
                        .inner
                        .clone()
                        .and_then(|conn| caching::set_cached_value(conn, &config, &response, &query).ok());
                }

                if cache_configuration.served_cache {
                    response.headers_mut().insert(
                        &HeaderName::try_from(CACHED_RESPONSE_HEADER).unwrap(),
                        HeaderValue::from_static("1"),
                    );
                }

                // If an error occurred fetching db data, do not send caching headers
                if !cache_configuration.error {
                    // Cache headers for client
                    if let Ok(cache_control_header_value) = HeaderValue::from_str(&format!(
                        "{}, max-age={}",
                        if cache_configuration.user_id.is_none() {
                            "public"
                        } else {
                            "private"
                        },
                        config.client_cache_period
                    )) {
                        response
                            .headers_mut()
                            .insert(&CACHE_CONTROL, cache_control_header_value);
                    }

                    if let Ok(response_str) = application::unwrap_body_to_string(&response) {
                        if let Ok(payload) = serde_json::from_str::<Value>(&response_str) {
                            let etag_hash = etag_hash(&payload.to_string());
                            if let Ok(new_header_value) = HeaderValue::from_str(&etag_hash) {
                                response.headers_mut().insert(&ETAG, new_header_value);
                                if request.headers().contains_key(IF_NONE_MATCH) {
                                    let etag = ETag(EntityTag::weak(etag_hash.to_string()));
                                    if let Ok(header_value) = request.headers()[IF_NONE_MATCH].to_str() {
                                        let etag_header = ETag(EntityTag::weak(header_value.to_string()));
                                        if etag.weak_eq(&etag_header) {
                                            response.set_body(Body::Empty);
                                            *response.status_mut() = StatusCode::NOT_MODIFIED;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(Response::Done(response))
    }
}
