use actix_service::{Service, Transform};
use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use actix_web::FromRequest;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use extractors::OptionalUser;
use futures::future::{ok, FutureResult};
use futures::{Future, Poll};
use log::Level;

pub struct BigNeonLogger {
    logger: Logger,
}

impl BigNeonLogger {
    pub fn new(format: &str) -> BigNeonLogger {
        BigNeonLogger {
            logger: Logger::new(format),
        }
    }
}
impl<S, B> Transform<S> for BigNeonLogger
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = BigNeonLoggerMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(BigNeonLoggerMiddleware { service })
    }
}

pub struct BigNeonLoggerMiddleware<S> {
    service: S,
}

impl<S, B> Service for BigNeonLoggerMiddleware<S>
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
        self.logger.start(request)?;
        let user = OptionalUser::from_request(request, &());
        let ip_address = request.connection_info().remote().map(|i| i.to_string());
        let uri = request.uri().to_string();
        let method = request.method().to_string();
        if uri != "/status" {
            jlog!(
                Level::Info,
                "bigneon_api::big_neon_logger",
                format!("{} {} starting", method, uri).as_str(),
                {
                    "user_id": user.ok().map(|u| u.0.map(|v| v.id())),
                    "ip_address": ip_address,
                    "uri": uri,
                    "method": method,
                    "api_version": env!("CARGO_PKG_VERSION")
            });
        }

        Box::new(self.service.call(request).and_then(|response| {
            match response.error() {
                Some(error) => {
                    let user = OptionalUser::from_request(request, &());
                    let ip_address = request.connection_info().remote().map(|i| i.to_string());
                    let uri = request.uri().to_string();
                    let method = request.method().to_string();
                    let level = if response.status() == StatusCode::UNAUTHORIZED {
                        Level::Info
                    } else if response.status().is_client_error() {
                        Level::Warn
                    } else {
                        Level::Error
                    };

                    jlog!(
                        level,
                        "bigneon_api::big_neon_logger",
                        &error.to_string(),
                        {
                            "user_id": user.ok().map(|u| u.0.map(|v| v.id())),
                            "ip_address": ip_address,
                            "uri": uri,
                            "method": method,
                            "api_version": env!("CARGO_PKG_VERSION")
                    });
                }
                None => {
                    if request.uri().to_string() == "/status" {
                        Ok(response)
                    } else {
                        Ok(self.logger.finish(request))
                    }
                }
            }

            Ok(response)
        }))
    }
}
