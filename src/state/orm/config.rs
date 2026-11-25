use std::ops::Deref;
#[cfg(feature = "orm-admin")]
use std::sync::Arc;

#[cfg(feature = "orm-admin")]
use super::admin::{AdminModelRegistry, AdminTokenGenerator, AdminTokenVerifier};
use afastorm::{ModelInfo, ModelRegistry};

#[cfg(feature = "orm-sqlite")]
pub(crate) type Database = afastorm::sqlx::Sqlite;
#[cfg(feature = "orm-postgres")]
pub(crate) type Database = afastorm::sqlx::Postgres;
#[cfg(feature = "orm-mysql")]
pub(crate) type Database = afastorm::sqlx::MySql;

#[derive(Clone)]
pub struct Orm {
    inner: afastorm::Orm<Database>,
    #[cfg(feature = "orm-admin")]
    admin_enabled: bool,
    #[cfg(feature = "orm-admin")]
    admin_url: Option<String>,
    #[cfg(feature = "orm-admin")]
    admin_token: Option<String>,
    #[cfg(feature = "orm-admin")]
    admin_logo_letter: String,
    #[cfg(feature = "orm-admin")]
    admin_brand_text: String,
    #[cfg(feature = "orm-admin")]
    admin_i18ns: Vec<(String, std::collections::HashMap<String, String>)>,
    #[cfg(feature = "orm-admin")]
    admin_models: AdminModelRegistry,
    #[cfg(feature = "orm-admin")]
    admin_token_generator: Option<AdminTokenGenerator>,
    #[cfg(feature = "orm-admin")]
    admin_token_verifier: Option<AdminTokenVerifier>,
}

impl Orm {
    pub async fn from_table(table: &toml::Table) -> crate::Result<Self> {
        #[cfg(feature = "orm-sqlite")]
        let (database_url, database_token, database_admin_url) = {
            if table.contains_key("orm-postgres") || table.contains_key("orm-mysql") {
                return Err(crate::Error::custom(
                    50001,
                    "this application was compiled with the SQLite ORM; remove other ORM sections",
                ));
            }
            let map = table
                .get("orm-sqlite")
                .and_then(toml::Value::as_table)
                .cloned()
                .unwrap_or_default();
            let path = match (map.get("path"), std::env::var("ORM_SQLITE_PATH")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[orm-sqlite].path must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => String::new(),
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_SQLITE_PATH is not valid UTF-8",
                    ));
                }
            };
            let token = match (map.get("token"), std::env::var("ORM_SQLITE_TOKEN")) {
                (_, Ok(value)) => Some(value),
                (Some(value), Err(std::env::VarError::NotPresent)) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[orm-sqlite].token must be a string")
                        })?
                        .to_owned(),
                ),
                (None, Err(std::env::VarError::NotPresent)) => None,
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_SQLITE_TOKEN is not valid UTF-8",
                    ));
                }
            };
            let admin_url = match (map.get("url"), std::env::var("ORM_SQLITE_URL")) {
                (_, Ok(value)) => Some(value),
                (Some(value), Err(std::env::VarError::NotPresent)) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[orm-sqlite].url must be a string")
                        })?
                        .to_owned(),
                ),
                (None, Err(std::env::VarError::NotPresent)) => None,
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_SQLITE_URL is not valid UTF-8",
                    ));
                }
            };
            let url = if path.is_empty() || path == ":memory:" {
                "sqlite::memory:".to_owned()
            } else if path.starts_with("sqlite:") {
                path
            } else {
                format!("sqlite://{}?mode=rwc", path)
            };
            (url, token, admin_url)
        };
        #[cfg(feature = "orm-postgres")]
        let (database_url, database_token, database_admin_url) = {
            if table.contains_key("orm-sqlite") || table.contains_key("orm-mysql") {
                return Err(crate::Error::custom(
                    50001,
                    "this application was compiled with the PostgreSQL ORM; remove other ORM sections",
                ));
            }
            let map = table
                .get("orm-postgres")
                .and_then(toml::Value::as_table)
                .cloned()
                .unwrap_or_default();
            let host = match (map.get("host"), std::env::var("ORM_POSTGRES_HOST")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[orm-postgres].host must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        "missing required configuration [orm-postgres].host or ORM_POSTGRES_HOST",
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_POSTGRES_HOST is not valid UTF-8",
                    ));
                }
            };
            let port = match (map.get("port"), std::env::var("ORM_POSTGRES_PORT")) {
                (_, Ok(value)) => value.parse().map_err(|error| {
                    crate::Error::custom(50001, format!("Invalid ORM_POSTGRES_PORT: {error}"))
                })?,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_integer()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[orm-postgres].port must be an integer")
                    })?
                    .try_into()
                    .map_err(|_| {
                        crate::Error::custom(50001, "[orm-postgres].port is out of range")
                    })?,
                (None, Err(std::env::VarError::NotPresent)) => 5432,
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_POSTGRES_PORT is not valid UTF-8",
                    ));
                }
            };
            let user = match (map.get("user"), std::env::var("ORM_POSTGRES_USER")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[orm-postgres].user must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        "missing required configuration [orm-postgres].user or ORM_POSTGRES_USER",
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_POSTGRES_USER is not valid UTF-8",
                    ));
                }
            };
            let pass = match (map.get("pass"), std::env::var("ORM_POSTGRES_PASS")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[orm-postgres].pass must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => String::new(),
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_POSTGRES_PASS is not valid UTF-8",
                    ));
                }
            };
            let name = match (map.get("name"), std::env::var("ORM_POSTGRES_NAME")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[orm-postgres].name must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        "missing required configuration [orm-postgres].name or ORM_POSTGRES_NAME",
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_POSTGRES_NAME is not valid UTF-8",
                    ));
                }
            };
            let token = match (map.get("token"), std::env::var("ORM_POSTGRES_TOKEN")) {
                (_, Ok(value)) => Some(value),
                (Some(value), Err(std::env::VarError::NotPresent)) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[orm-postgres].token must be a string")
                        })?
                        .to_owned(),
                ),
                (None, Err(std::env::VarError::NotPresent)) => None,
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_POSTGRES_TOKEN is not valid UTF-8",
                    ));
                }
            };
            let admin_url = match (map.get("url"), std::env::var("ORM_POSTGRES_URL")) {
                (_, Ok(value)) => Some(value),
                (Some(value), Err(std::env::VarError::NotPresent)) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[orm-postgres].url must be a string")
                        })?
                        .to_owned(),
                ),
                (None, Err(std::env::VarError::NotPresent)) => None,
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_POSTGRES_URL is not valid UTF-8",
                    ));
                }
            };
            (
                format!(
                    "postgres://{}:{}@{}:{}/{}",
                    Self::encode_component(&user),
                    Self::encode_component(&pass),
                    host,
                    port,
                    Self::encode_component(&name)
                ),
                token,
                admin_url,
            )
        };
        #[cfg(feature = "orm-mysql")]
        let (database_url, database_token, database_admin_url) = {
            if table.contains_key("orm-sqlite") || table.contains_key("orm-postgres") {
                return Err(crate::Error::custom(
                    50001,
                    "this application was compiled with the MySQL ORM; remove other ORM sections",
                ));
            }
            let map = table
                .get("orm-mysql")
                .and_then(toml::Value::as_table)
                .cloned()
                .unwrap_or_default();
            let host = match (map.get("host"), std::env::var("ORM_MYSQL_HOST")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[orm-mysql].host must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        "missing required configuration [orm-mysql].host or ORM_MYSQL_HOST",
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_MYSQL_HOST is not valid UTF-8",
                    ));
                }
            };
            let port = match (map.get("port"), std::env::var("ORM_MYSQL_PORT")) {
                (_, Ok(value)) => value.parse().map_err(|error| {
                    crate::Error::custom(50001, format!("Invalid ORM_MYSQL_PORT: {error}"))
                })?,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_integer()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[orm-mysql].port must be an integer")
                    })?
                    .try_into()
                    .map_err(|_| crate::Error::custom(50001, "[orm-mysql].port is out of range"))?,
                (None, Err(std::env::VarError::NotPresent)) => 3306,
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_MYSQL_PORT is not valid UTF-8",
                    ));
                }
            };
            let user = match (map.get("user"), std::env::var("ORM_MYSQL_USER")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[orm-mysql].user must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        "missing required configuration [orm-mysql].user or ORM_MYSQL_USER",
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_MYSQL_USER is not valid UTF-8",
                    ));
                }
            };
            let pass = match (map.get("pass"), std::env::var("ORM_MYSQL_PASS")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[orm-mysql].pass must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => String::new(),
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_MYSQL_PASS is not valid UTF-8",
                    ));
                }
            };
            let name = match (map.get("name"), std::env::var("ORM_MYSQL_NAME")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[orm-mysql].name must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        "missing required configuration [orm-mysql].name or ORM_MYSQL_NAME",
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_MYSQL_NAME is not valid UTF-8",
                    ));
                }
            };
            let token = match (map.get("token"), std::env::var("ORM_MYSQL_TOKEN")) {
                (_, Ok(value)) => Some(value),
                (Some(value), Err(std::env::VarError::NotPresent)) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[orm-mysql].token must be a string")
                        })?
                        .to_owned(),
                ),
                (None, Err(std::env::VarError::NotPresent)) => None,
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_MYSQL_TOKEN is not valid UTF-8",
                    ));
                }
            };
            let admin_url = match (map.get("url"), std::env::var("ORM_MYSQL_URL")) {
                (_, Ok(value)) => Some(value),
                (Some(value), Err(std::env::VarError::NotPresent)) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[orm-mysql].url must be a string")
                        })?
                        .to_owned(),
                ),
                (None, Err(std::env::VarError::NotPresent)) => None,
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "ORM_MYSQL_URL is not valid UTF-8",
                    ));
                }
            };
            (
                format!(
                    "mysql://{}:{}@{}:{}/{}",
                    Self::encode_component(&user),
                    Self::encode_component(&pass),
                    host,
                    port,
                    Self::encode_component(&name)
                ),
                token,
                admin_url,
            )
        };
        #[cfg(feature = "orm-admin")]
        let token = match (database_token, std::env::var("ORM_ADMIN_TOKEN")) {
            (_, Ok(value)) => (!value.is_empty()).then_some(value),
            (Some(value), Err(std::env::VarError::NotPresent)) => Some(value),
            (None, Err(std::env::VarError::NotPresent)) => None,
            (_, Err(std::env::VarError::NotUnicode(_))) => {
                return Err(crate::Error::custom(
                    50001,
                    "ORM_ADMIN_TOKEN is not valid UTF-8",
                ));
            }
        };
        #[cfg(feature = "orm-admin")]
        let admin_url = match (database_admin_url, std::env::var("ORM_ADMIN_URL")) {
            (_, Ok(value)) => (!value.is_empty()).then_some(value),
            (Some(value), Err(std::env::VarError::NotPresent)) => Some(value),
            (None, Err(std::env::VarError::NotPresent)) => None,
            (_, Err(std::env::VarError::NotUnicode(_))) => {
                return Err(crate::Error::custom(
                    50001,
                    "ORM_ADMIN_URL is not valid UTF-8",
                ));
            }
        };
        #[cfg(not(feature = "orm-admin"))]
        let _ = (&database_token, &database_admin_url);
        let inner = afastorm::Orm::<Database>::connect_lazy(database_url).map_err(|error| {
            crate::Error::custom(50901, format!("ORM initialization failed: {error}"))
        })?;
        Ok(Self {
            inner,
            #[cfg(feature = "orm-admin")]
            admin_enabled: false,
            #[cfg(feature = "orm-admin")]
            admin_url,
            #[cfg(feature = "orm-admin")]
            admin_token: token,
            #[cfg(feature = "orm-admin")]
            admin_logo_letter: "A".to_owned(),
            #[cfg(feature = "orm-admin")]
            admin_brand_text: "Afast ORM".to_owned(),
            #[cfg(feature = "orm-admin")]
            admin_i18ns: Vec::new(),
            #[cfg(feature = "orm-admin")]
            admin_models: AdminModelRegistry::new(),
            #[cfg(feature = "orm-admin")]
            admin_token_generator: None,
            #[cfg(feature = "orm-admin")]
            admin_token_verifier: None,
        })
    }

    #[cfg(any(feature = "orm-postgres", feature = "orm-mysql"))]
    fn encode_component(value: &str) -> String {
        let mut output = String::with_capacity(value.len());
        for byte in value.bytes() {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                output.push(byte as char);
            } else {
                output.push_str(&format!("%{byte:02X}"));
            }
        }
        output
    }

    pub fn with_models(mut self, registry: ModelRegistry) -> Self {
        self.inner = self.inner.with_models(registry);
        self
    }

    #[cfg(feature = "orm-admin")]
    pub fn with_admin(mut self, enabled: bool) -> Self {
        self.admin_enabled = enabled;
        self
    }

    #[cfg(feature = "orm-admin")]
    pub fn with_admin_url(mut self, url: impl Into<String>) -> Self {
        self.admin_url = Some(url.into());
        self
    }

    #[cfg(feature = "orm-admin")]
    pub fn with_admin_token(mut self, token: impl Into<String>) -> Self {
        self.admin_token = Some(token.into());
        self
    }

    #[cfg(feature = "orm-admin")]
    pub fn with_admin_logo_letter(mut self, letter: impl Into<String>) -> Self {
        self.admin_logo_letter = letter.into();
        self
    }

    #[cfg(feature = "orm-admin")]
    pub fn with_admin_brand_text(mut self, text: impl Into<String>) -> Self {
        self.admin_brand_text = text.into();
        self
    }

    #[cfg(feature = "orm-admin")]
    pub fn with_i18ns(
        mut self,
        i18ns: Vec<(String, std::collections::HashMap<String, String>)>,
    ) -> Self {
        self.admin_i18ns = i18ns;
        self
    }

    #[cfg(feature = "orm-admin")]
    pub fn with_admin_models(mut self, models: AdminModelRegistry) -> Self {
        self.admin_models = models;
        self
    }

    #[cfg(feature = "orm-admin")]
    pub fn with_admin_token_functions(
        mut self,
        generate: impl Fn(&str) -> Option<String> + Send + Sync + 'static,
        verify: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.admin_token_generator = Some(Arc::new(generate));
        self.admin_token_verifier = Some(Arc::new(verify));
        self
    }

    pub fn model_metadata(&self) -> Vec<&'static ModelInfo> {
        self.inner.model_metadata()
    }

    pub fn inner(&self) -> &afastorm::Orm<Database> {
        &self.inner
    }

    #[cfg(feature = "orm-admin")]
    pub(crate) fn admin_enabled(&self) -> bool {
        self.admin_enabled
    }

    #[cfg(feature = "orm-admin")]
    pub(crate) fn admin_token(&self) -> Option<&str> {
        self.admin_token
            .as_deref()
            .filter(|token| !token.trim().is_empty())
    }

    #[cfg(feature = "orm-admin")]
    pub fn admin_url(&self) -> Option<&str> {
        self.admin_url.as_deref()
    }

    #[cfg(feature = "orm-admin")]
    pub(crate) fn admin_service(&self, url: &str) -> crate::Result<afast::Service> {
        if !self.admin_models.has_enabled() {
            return Err(crate::Error::custom(
                50001,
                "ORM admin service requires at least one registered admin model",
            ));
        }
        super::admin::service(url)
    }
}

impl Deref for Orm {
    type Target = afastorm::Orm<Database>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub trait AFasterOrmExt {
    fn with_orm(self, f: impl FnOnce(Orm) -> Orm) -> crate::Result<Self>
    where
        Self: Sized;
}

impl AFasterOrmExt for crate::AFaster {
    fn with_orm(mut self, f: impl FnOnce(Orm) -> Orm) -> crate::Result<Self> {
        self.state.orm = f(self.state.orm.clone());

        #[cfg(feature = "orm-admin")]
        {
            self.services
                .retain(|service| service.name != "afastorm_admin");

            if self.state.orm.admin_enabled() {
                let auth = super::admin::AdminAuth::new(
                    self.state.orm.admin_token().map(str::to_owned),
                    self.state.orm.admin_token_generator.clone(),
                    self.state.orm.admin_token_verifier.clone(),
                    self.state.orm.admin_logo_letter.clone(),
                    self.state.orm.admin_brand_text.clone(),
                    self.state.orm.admin_i18ns.clone(),
                    &self.state.orm.admin_models,
                    self.state.orm.inner.registry(),
                )?;
                let url = self.state.orm.admin_url().unwrap_or("/admin");
                let service = self.state.orm.admin_service(url)?;
                self.orm_admin_auth = Some(auth);
                self.services.push(service);
            } else {
                self.orm_admin_auth = None;
            }
        }

        Ok(self)
    }
}
