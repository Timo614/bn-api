use access_token::AccessToken;
use endpoints::*;
use error::FacebookError;
use std::rc::Rc;

use url::form_urlencoded;

pub struct FacebookClient {
    pub official_events: OfficialEventsEndpoint,
    pub me: MeEndpoint,
    pub inner_client: Rc<FacebookClientInner>,
    pub permissions: PermissionsEndpoint,
}

const BASE_URL: &str = "https://graph.facebook.com";

impl FacebookClient {
    pub fn from_access_token(access_token: String) -> FacebookClient {
        let inner = FacebookClientInner {
            base_url: BASE_URL,
            app_access_token: access_token,
        };

        let inner = Rc::new(inner);

        FacebookClient {
            inner_client: inner.clone(),
            official_events: OfficialEventsEndpoint {
                client: inner.clone(),
            },
            me: MeEndpoint::new(inner.clone()),
            permissions: PermissionsEndpoint {
                client: inner.clone(),
            },
        }
    }

    pub fn get_login_url(
        app_id: &str,
        redirect_uri: Option<&str>,
        state: &str,
        scopes: &[&str],
    ) -> String {
        let scope = scopes.iter().fold("".to_string(), |s, t| {
            if s.len() == 0 {
                t.to_string()
            } else {
                s + "," + t
            }
        });

        let result = form_urlencoded::Serializer::new(String::new())
            .append_pair("client_id", app_id)
            .append_pair(
                "redirect_uri",
                redirect_uri.unwrap_or("https://www.facebook.com/connect/login_success.html"),
            )
            .append_pair("state", state)
            .append_pair("scope", &scope)
            .finish();
        format!("https://www.facebook.com/v3.2/dialog/oauth?{}", result)
    }

    pub fn get_access_token(
        app_id: &str,
        app_secret: &str,
        redirect_uri: Option<&str>,
        code: &str,
    ) -> Result<AccessToken, FacebookError> {
        let client = reqwest::Client::new();
        let mut resp = client
            .get(&format!(
                "{}/v3.2/oauth/access_token?client_id={}&redirect_uri={}&client_secret={}&code={}",
                BASE_URL,
                app_id,
                redirect_uri.unwrap_or("https://www.facebook.com/connect/login_success.html"),
                app_secret,
                code
            ))
            .send()?;
        //        let status = resp.status();
        let value: serde_json::Value = resp.json()?;
        println!("{:?}", value);
        let result: AccessToken = serde_json::from_value(value)?;
        Ok(result)
    }
}

pub struct FacebookClientInner {
    pub base_url: &'static str,
    pub app_access_token: String,
}
