use bigneon_db::models::*;
use config::Config;
use errors::{ApplicationError, BigNeonError};
use log::Level::Debug;
use serde_json::Value;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use utils::webhook_adapters::WebhookAdapter;

pub struct CustomerIoWebhookAdapter {
    site_id: String,
    api_key: String,
    environment: Environment,
}

impl CustomerIoWebhookAdapter {
    pub fn new(global_config: &Config) -> CustomerIoWebhookAdapter {
        CustomerIoWebhookAdapter {
            site_id: "".to_string(),
            api_key: "".to_string(),
            environment: global_config.environment,
        }
    }
}

impl WebhookAdapter for CustomerIoWebhookAdapter {
    fn initialize(&mut self, config: Value) {
        self.site_id = config["site_id"].as_str().unwrap().to_string();
        self.api_key = config["api_key"].as_str().unwrap().to_string();
    }

    fn send(
        &self,
        _webhook_urls: &[String],
        payload: HashMap<String, Value, RandomState>,
    ) -> Result<(), BigNeonError> {
        let client = reqwest::Client::new();
        let mut payload = payload;
        payload.insert("environment".to_string(), json!(self.environment));

        let client = match payload.get("user_id").and_then(|u| u.as_str()) {
            Some(user_id) => {
                if let Some(webhook_event_type) =
                    payload.get("webhook_event_type").and_then(|w| w.as_str())
                {
                    let res = match webhook_event_type {
                        "temporary_user_created" | "user_created" => client
                            .put(&format!(
                                "https://track.customer.io/api/v1/customers/{}",
                                user_id
                            ))
                            .json(&payload),

                        _ => {
                            //                            if !payload.has_element("name") {
                            //                                payload.insert("name".to_string(), webhook_event_type.clone());
                            //                            }
                            client
                                .post(&format!(
                                    "https://track.customer.io/api/v1/customers/{}/events",
                                    user_id
                                ))
                                .json(&json!({"name": webhook_event_type, "data": payload}))
                        }
                    };

                    res
                } else {
                    return Err(ApplicationError::new(
                        "Cannot determine event to send to Customer.io".to_string(),
                    )
                    .into());
                }
            }
            None => client
                .post("https://track.customer.io/api/v1/events")
                .json(&payload),
        };

        jlog!(
            Debug,
            "bigneon::domain_actions",
            "Sending event/customer to customer.io",
            { "payload": &payload }
        );

        let mut resp = client
            .basic_auth(&self.site_id, Some(&self.api_key))
            .send()
            .map_err(|_err| ApplicationError::new("Error making webhook request".to_string()))?;

        let text = resp
            .text()
            .map_err(|_err| ApplicationError::new("Error making webhook request".to_string()))?;

        jlog!(Debug, "bigneon::domain_actions", "Response from customer.io", {"text": text, "status": resp.status().to_string()});
        resp.error_for_status()?;

        Ok(())
    }
}
