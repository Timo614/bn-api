use super::tari_messages::*;
use tari_client::TariClient;
use tari_error::TariError;
use uuid::Uuid;

#[derive(Clone)]
pub struct TariTestClient {
    tari_url: String,
}

impl TariTestClient {
    pub fn new(tari_url: String) -> TariTestClient {
        TariTestClient { tari_url }
    }
}

impl TariClient for TariTestClient {
    fn create_asset(&self, _asset: NewAsset) -> Result<String, TariError> {
        Ok(Uuid::new_v4().to_string())
    }

    fn transfer_tokens(
        &self,
        _asset_id: &String,
        _token_ids: Vec<u64>,
        _new_owner: String,
    ) -> Result<(), TariError> {
        Ok(())
    }

    fn get_asset_info(&self, _asset_id: &String) -> Result<AssetInfoResult, TariError> {
        Ok(AssetInfoResult {
            id: Uuid::new_v4().to_string(),
            name: "Awesome Asset".to_string(),
            symbol: "A".to_string(),
            decimals: 8,
            total_supply: 100,
            authorised_signers: vec!["".to_string()],
            issuer: Uuid::new_v4().to_string(),
            rule_flags: 0,
            rule_metadata: "metadata!".to_string(),
            expired: false,
        })
    }

    fn box_clone(&self) -> Box<TariClient + Send + Sync> {
        Box::new((*self).clone())
    }
}
