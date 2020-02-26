use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::http::{HttpTryFrom, Method};
use actix_web::middleware::{Middleware, Response, Started};
use actix_web::{FromRequest, HttpRequest, HttpResponse, Result};
use extractors::*;
use helpers::*;
use server::AppState;
use std::collections::BTreeMap;

const CACHED_RESPONSE_HEADER: &'static str = "X-Cached-Response";

#[derive(Clone)]
pub struct CacheResource {}

impl CacheResource {
    pub fn new() -> CacheResource {
        CacheResource {}
    }
}

impl Middleware<AppState> for CacheResource {
    fn start(&self, request: &HttpRequest<AppState>) -> Result<Started> {
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
            let path_text = "path".to_string();
            let path = resource_def.unwrap().pattern().to_string();
            let method_text = "method".to_string();
            let method = request.method().to_string();
            query.insert(&path_text, &path);
            query.insert(&method_text, &method);
            let state = request.state().clone();
            let config = state.config.clone();
            let user = OptionalUser::from_request(request, &());
            if user.is_ok() && user.unwrap().0.is_none() {
                let cache_database = state.database.cache_database.clone();
                // if there is a error in the cache, the value does not exist
                let cached_value = cache_database
                    .clone()
                    .inner
                    .clone()
                    .and_then(|conn| caching::get_cached_value(conn, &config, &query));
                if let Some(response) = cached_value {
                    // Insert self into extensions to let response know not to set the value
                    request.extensions_mut().insert(self.clone());
                    return Ok(Started::Response(response));
                }
            }
        }

        Ok(Started::Done)
    }

    fn response(&self, request: &HttpRequest<AppState>, mut response: HttpResponse) -> Result<Response> {
        if request.method() == Method::GET {
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
            let path_text = "path".to_string();
            let path = resource_def.unwrap().pattern().to_string();
            let method_text = "method".to_string();
            let method = request.method().to_string();
            query.insert(&path_text, &path);
            query.insert(&method_text, &method);
            let state = request.state().clone();
            let config = state.config.clone();
            let extensions = request.extensions();
            let cached = extensions.get::<CacheResource>();
            let user = OptionalUser::from_request(request, &());
            if cached.is_none() && user.is_ok() && user.unwrap().0.is_none() {
                let cache_database = state.database.cache_database.clone();
                cache_database
                    .inner
                    .clone()
                    .and_then(|conn| caching::set_cached_value(conn, &config, &response, &query).ok());
            } else if cached.is_some() {
                response.headers_mut().insert(
                    &HeaderName::try_from(CACHED_RESPONSE_HEADER).unwrap(),
                    HeaderValue::from_static("1"),
                );
            }
        }

        Ok(Response::Done(response))
    }
}
