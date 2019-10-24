use actix_service::{Service, Transform};
use actix_web::dev::Body::Bytes;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::http::HttpTryFrom;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use futures::future::{ok, FutureResult};
use futures::{Future, Poll};
use regex::{Captures, Regex};
use serde_json;
use serde_json::Value;
use std::collections::HashMap;
use std::str;

const CONTENT_TYPE: &'static str = "Content-Type";
const TEXT_HTML: &'static str = "text/html";

const HTML_RESPONSE: &'static str = r#"
<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8">

    <title>%title%</title>
    <meta name="description" content="%description%">
    <meta name="author" content="%creator%">

    <meta property="og:type" content="website"/>
    <meta property="og:title" content="%title%"/>
    <meta property="og:url" content="%url%"/>
    <meta property="og:image" content="%promo_image_url%"/>
    <meta property="og:site_name" content="%site_name%"/>
    <meta property="og:description" content="%description%"/>

    <meta name="twitter:site" content="%url%"/>
    <meta name="twitter:creator" content="%creator%"/>
    <meta name="twitter:title" content="%title%"/>
    <meta name="twitter:image" content="%promo_image_url%"/>
    <meta name="description" content="%description%"/>
</head>

<body>
</body>
</html>"#;

pub struct Metatags {
    trigger_header: String,
    trigger_value: String,
    front_end_url: String,
    app_name: String,
}

impl Metatags {
    pub fn new(
        trigger_header: String,
        trigger_value: String,
        front_end_url: String,
        app_name: String,
    ) -> Metatags {
        Metatags {
            trigger_header,
            trigger_value,
            front_end_url,
            app_name,
        }
    }
}

impl<S, B> Transform<S> for Metatags
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = MetatagsMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(MetatagsMiddleware { service })
    }
}

pub struct MetatagsMiddleware<S> {
    service: S,
}

impl<S, B> Service for MetatagsMiddleware<S>
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
            if response.status() != 200 {
                return Ok(response);
            }
            let response = match request.headers().get(&self.trigger_header) {
                Some(header) => {
                    if header.to_str().unwrap_or("").to_string() == self.trigger_value {
                        let mut values: HashMap<&str, String> = HashMap::new();
                        let path = request.uri().path();

                        let mut data_to_use: Option<Value> = None;
                        //Check we have hit an event view endpoint
                        //TODO move this into a customizable format
                        let event_re = Regex::new(r"/events/[A-Za-z0-9\-]{36}$").unwrap();
                        if event_re.is_match(path) {
                            data_to_use = match response.body() {
                                Bytes(bytes) => {
                                    let json = serde_json::from_str(
                                        str::from_utf8(bytes.as_ref()).unwrap(),
                                    )
                                    .unwrap_or(None);
                                    if json.is_some() {
                                        Some(json.unwrap())
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            }
                        }

                        values.insert("title", format!("{}", self.app_name));
                        values.insert("site_name", format!("{}", self.app_name));
                        values.insert("url", format!("{}{}", self.front_end_url, path));
                        values.insert("creator", format!("{}", self.app_name));
                        //TODO add the slogan to the .env
                        values.insert("description", format!("{}", "The Future Of Ticketing"));
                        //TODO Add the logo address to the .env
                        values.insert(
                            "promo_image_url",
                            format!("{}{}", self.front_end_url, "/images/bn-logo-text-web.svg"),
                        );
                        if let Some(data) = data_to_use {
                            let name = data["name"].as_str().unwrap_or("");
                            let description = data["additional_info"].as_str().unwrap_or("");
                            let promo_image_url = data["promo_image_url"].as_str().unwrap_or("");
                            let creator = data["venue"]["name"].as_str().unwrap_or("");

                            values.entry("title").and_modify(|e| {
                                *e = format!("{} - {}", self.app_name, name);
                            });
                            values.entry("description").and_modify(|e| {
                                *e = format!("{}", description);
                            });
                            values.entry("creator").and_modify(|e| {
                                *e = format!("{}", creator);
                            });
                            values.entry("promo_image_url").and_modify(|e| {
                                *e = format!("{}", promo_image_url);
                            });
                        }
                        response.headers_mut().insert(
                            HeaderName::try_from(CONTENT_TYPE).unwrap(),
                            HeaderValue::from_static(TEXT_HTML),
                        );

                        let keys = vec![
                            "creator",
                            "description",
                            "promo_image_url",
                            "site_name",
                            "title",
                            "url",
                        ];

                        let mut result = HTML_RESPONSE.to_string();

                        for key in keys.into_iter() {
                            let regex_expression = format!("%{}%", key);
                            let re = Regex::new(regex_expression.as_str()).unwrap();
                            let value = values
                                .get(key)
                                .map(|v| v.to_string())
                                .unwrap_or("".to_string());
                            result = re
                                .replace_all(result.as_str(), |_caps: &Captures| {
                                    format!("{}", value)
                                })
                                .to_string();
                        }

                        response.set_body(result);
                    }
                    response
                }
                None => response,
            };
            return Ok(response);
        }))
    }
}
