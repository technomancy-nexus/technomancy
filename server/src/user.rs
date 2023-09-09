use axum_login::{secrecy::SecretVec, AuthUser};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub(crate) name: String,
}

impl AuthUser<String> for User {
    fn get_id(&self) -> String {
        self.name.clone()
    }

    fn get_password_hash(&self) -> axum_login::secrecy::SecretVec<u8> {
        SecretVec::new(self.name.clone().into_bytes())
    }
}
