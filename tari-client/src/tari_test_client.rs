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
    fn create_asset(
        &self,
        _secret_key: &String,
        _public_key: &String,
        _asset: MessagePayloadCreateAsset,
    ) -> Result<String, TariError> {
        Ok(Uuid::new_v4().to_string())
    }

    fn modify_asset_increase_supply(
        &self,
        _secret_key: &String,
        _public_key: &String,
        _asset_id: &String,
        _new_supply: u64,
    ) -> Result<(), TariError> {
        Ok(())
    }

    fn modify_asset_nullify_tokens(
        &self,
        _secret_key: &String,
        _public_key: &String,
        _asset_id: &String,
        _token_ids: Vec<u64>,
    ) -> Result<(), TariError> {
        Ok(())
    }

    fn modify_asset_redeem_token(
        &self,
        _secret_key: &String,
        _public_key: &String,
        _asset_id: &String,
        _token_ids: Vec<u64>,
    ) -> Result<(), TariError> {
        Ok(())
    }

    fn transfer_tokens(
        &self,
        _secret_key: &String,
        _public_key: &String,
        _asset_id: &String,
        _token_ids: Vec<u64>,
        _new_owner: String,
    ) -> Result<(), TariError> {
        Ok(())
    }

    fn get_asset_info(
        &self,
        _secret_key: &String,
        _public_key: &String,
        _asset_id: &String,
    ) -> Result<ResponsePayloadReadAsset, TariError> {
        Ok(ResponsePayloadReadAsset {
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

    fn box_clone(&self) -> Box<dyn TariClient + Send + Sync> {
        Box::new((*self).clone())
    }
}
