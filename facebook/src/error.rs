#[derive(Error, Debug)]
pub enum FacebookError {
    #[error(no_from, non_std)]
    FacebookError(FacebookErrorResponse),
    HttpError(reqwest::Error),
    DeserializationError(serde_json::Error),
    #[error(msg_embedded, no_from, non_std)]
    ParseError(String),
}

#[derive(Deserialize, Debug)]
pub struct FacebookErrorResponse {
    pub message: String,
    pub code: i32,
    pub fbtrace_id: String,
}
