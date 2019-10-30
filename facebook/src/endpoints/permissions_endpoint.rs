use error::FacebookError;
use facebook_client::FacebookClientInner;
use paging::Paging;
use permission::Permission;
use std::rc::Rc;

pub struct PermissionsEndpoint {
    pub client: Rc<FacebookClientInner>,
}

impl PermissionsEndpoint {
    pub fn list(&self, user_id: &str) -> Result<Paging<Permission>, FacebookError> {
        let client = reqwest::Client::new();

        // println!("{}", json!(&request));

        //jlog!(Info, "Sending request to Facebook", { "request": &request });

        let mut resp = client
            .get(&format!(
                "{}/{}/permissions",
                &self.client.base_url, user_id
            ))
            .header(
                "Authorization",
                format!("Bearer {}", &self.client.app_access_token),
            )
            .send()?;
        //        let status = resp.status();
        let value: serde_json::Value = resp.json()?;
        println!("{:?}", value.clone().to_string());

        let results: Paging<Permission> = serde_json::from_value(value)?;
        Ok(results)
    }
}
