use afast::{AFastDeserialize, Tag};

#[derive(AFastDeserialize, Tag)]
#[afast(tag("Authentication data"))]
pub struct AuthData {
    pub token: String,
    #[cfg(feature = "auth-platform")]
    pub platform: String,
}
