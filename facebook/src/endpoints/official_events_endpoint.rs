use error::{FacebookError, FacebookErrorResponse};
use facebook_client::FacebookClientInner;
use facebook_request::FacebookRequest;
use fbid::FBID;
use log::Level::Info;
use nodes::Event;
use reqwest::StatusCode;
use std::rc::Rc;

pub struct OfficialEventsEndpoint {
    pub client: Rc<FacebookClientInner>,
}

impl OfficialEventsEndpoint {
    pub fn create(&self, event: Event) -> Result<FBID, FacebookError> {
        let request = FacebookRequest {
            access_token: &self.client.app_access_token,
            data: event,
        };

        let client = reqwest::Client::new();

        println!("{}", json!(&request));

        jlog!(Info, "Sending request to Facebook", { "request": &request });

        // Example json to use at https://developers.facebook.com/tools/explorer
        /*
        {
            "category": "WORKSHOP",
            "name": "Test",
            "description": "Test",
            "cover": {
                "source": "https://source.unsplash.com/random"
            },
            "place_id":

                "1078236045577061",
            "timezone": "UTC",
            "start_time": "1 Jan 2021"
        }
        */
        let mut resp = client
            .post(&format!("{}/v5.0/official_events", &self.client.base_url))
            .json(&request)
            .send()?;
        //        let status = resp.status();
        //        if status != StatusCode::UNPROCESSABLE_ENTITY && status != StatusCode::OK {
        //            return Err(resp.error_for_status().err().map(|e| e.into()).unwrap_or(
        //                GlobeeError::UnexpectedResponseError(format!(
        //                    "Unexpected status code from Globee: {}",
        //                    status
        //                )),
        //            ));
        //        };

        let value: serde_json::Value = resp.json()?;
        println!("{:?}", value);

        jlog!(Info, "Response from Facebook", { "response": &value });

        if resp.status() != StatusCode::OK {
            if let Some(error) = value.get("error") {
                let error: FacebookErrorResponse = serde_json::from_value(error.clone())?;
                return Err(FacebookError::FacebookError(error));
            }
            resp.error_for_status()?;
        }

        #[derive(Deserialize)]
        struct R {
            id: String,
        }

        Ok(FBID(serde_json::from_value::<R>(value)?.id))
    }
}
