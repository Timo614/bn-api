use actix_service::{Service, Transform};
use actix_web::http::HttpTryFrom;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use futures::future::{ok, FutureResult};
use futures::{Future, Poll};

const SEMVER_HEADER_NAME: &'static str = "X-App-Version";
const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub struct AppVersionHeader {
    header_name: HeaderName,
    app_version: HeaderValue,
}

impl AppVersionHeader {
    pub fn new() -> AppVersionHeader {
        AppVersionHeader {
            header_name: HeaderName::try_from(SEMVER_HEADER_NAME).unwrap(),
            app_version: HeaderValue::from_static(APP_VERSION),
        }
    }
}

impl<S, B> Transform<S> for AppVersionHeader
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AppVersionHeaderMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AppVersionHeaderMiddleware { service })
    }
}

pub struct AppVersionHeaderMiddleware<S> {
    service: S,
}

impl<S, B> Service for AppVersionHeaderMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, request: ServiceRequest) -> Self::Future {
        Box::new(self.service.call(request).and_then(|response| {
            let header = AppVersionHeader::new();
            response
                .headers_mut()
                .insert(&header.header_name, header.app_version.clone());

            Ok(response)
        }))
    }
}
