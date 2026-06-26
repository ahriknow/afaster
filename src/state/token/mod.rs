mod err;

use err::*;

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize)]
pub struct Token {
    pub expire: i64,
    pub secret: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Data1 {
    pub id: i64,
    pub username: String,
    pub nickname: String,
    pub avatar: String,
    pub is_super: bool,
}

impl Token {
    /// 创建 JWT 令牌，支持嵌入泛型数据
    pub fn create_token<T: Serialize>(&self, user_id: i64, data: T) -> crate::Result<String> {
        let expiration = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(self.expire))
            .ok_or_else(|| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50003, msg = "Invalid timestamp" });
                crate::error::server()
            })?
            .timestamp() as usize;

        let claims = Claims {
            sub: user_id.to_string(),
            exp: expiration,
            uid: user_id,
            data: Some(data),
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50101, msg = "JWT encode failed" },  "{}", _e);
            encode_failed()
        })?;

        Ok(token)
    }

    /// 获取令牌中的用户 ID
    pub fn get_id(&self, token: &str) -> crate::Result<i64> {
        let claims = self.verify_token::<serde::de::IgnoredAny>(token)?;
        Ok(claims.uid)
    }

    pub fn get_super_id(&self, token: &str) -> crate::Result<i64> {
        let claims = self.verify_token::<Data1>(token)?;
        let uid = claims.uid;
        let data = claims.data.ok_or_else(|| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50103, msg = "Token data not found" });
            crate::Error::custom(50103, "Token data not found")
        })?;
        if !data.is_super {
            return Err(crate::Error::custom(403, "权限不足"));
        }
        Ok(uid)
    }

    /// 验证令牌并返回完整声明（含泛型数据）
    pub fn verify_token<T: DeserializeOwned>(&self, token: &str) -> crate::Result<Claims<T>> {
        let validation = Validation::new(Algorithm::HS256);

        let token_data = decode::<Claims<T>>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::InvalidToken => {
                #[cfg(feature = "log")]
                tracing::warn!({ code = 40101, msg = "Invalid token" });
                invalid_token()
            }
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                #[cfg(feature = "log")]
                tracing::warn!({ code = 40102, msg = "Token expired" });
                token_expired()
            }
            _ => {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50102, msg = "JWT verify failed" },  "{}", e);
                verify_failed()
            }
        })?;

        Ok(token_data.claims)
    }

    /// 验证令牌并获取嵌入的泛型数据
    pub fn get_data<T: DeserializeOwned>(&self, token: &str) -> crate::Result<T> {
        let claims = self.verify_token::<T>(token)?;
        claims.data.ok_or_else(|| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50103, msg = "Token data not found" });
            crate::Error::custom(50103, "Token data not found")
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(bound(deserialize = "T: for<'de2> Deserialize<'de2>"))]
pub struct Claims<T = ()> {
    pub sub: String,
    pub exp: usize,
    pub uid: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub data: Option<T>,
}

impl Token {
    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        crate::state::extract(table, "token")
    }
}
