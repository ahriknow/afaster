use std::ops::Deref;

use afastorm::{ModelInfo, ModelRegistry};
use serde::Deserialize;

#[cfg(feature = "orm-sqlite")]
#[derive(Deserialize)]
struct SqliteOrmSettings {
    path: String,
    token: Option<String>,
    url: Option<String>,
}

#[cfg(feature = "orm-postgres")]
#[derive(Deserialize)]
struct PostgresOrmSettings {
    host: String,
    port: u16,
    user: String,
    pass: String,
    name: String,
    token: Option<String>,
    url: Option<String>,
}

#[derive(Clone)]
pub struct Orm {
    inner: afastorm::Orm,
    #[cfg(feature = "orm-admin")]
    admin_enabled: bool,
    #[cfg(feature = "orm-admin")]
    admin_url: Option<String>,
    #[cfg(feature = "orm-admin")]
    admin_token: Option<String>,
}

impl Orm {
    pub async fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let (database_url, token, admin_url) = Self::database_settings(table)?;
        let mut inner = afastorm::Orm::connect(database_url)
            .await
            .map_err(|error| {
                crate::Error::custom(50901, format!("ORM initialization failed: {error}"))
            })?;
        if let Some(token) = token {
            inner = inner.with_admin_token(token);
        }
        if let Some(url) = admin_url {
            inner = inner.with_admin_url(url);
        }

        Ok(Self {
            inner,
            #[cfg(feature = "orm-admin")]
            admin_enabled: false,
            #[cfg(feature = "orm-admin")]
            admin_url: None,
            #[cfg(feature = "orm-admin")]
            admin_token: None,
        })
    }

    fn database_settings(
        table: &toml::Table,
    ) -> crate::Result<(String, Option<String>, Option<String>)> {
        let mut settings = None;
        #[cfg(feature = "orm-sqlite")]
        if table.contains_key("orm-sqlite") {
            let config: SqliteOrmSettings = crate::state::extract(table, "orm-sqlite")?;
            let url = if config.path == ":memory:" {
                "sqlite::memory:".to_owned()
            } else if config.path.starts_with("sqlite:") {
                config.path
            } else {
                format!("sqlite://{}?mode=rwc", config.path)
            };
            if settings.replace((url, config.token, config.url)).is_some() {
                return Err(crate::Error::custom(
                    50001,
                    "configure exactly one enabled ORM driver",
                ));
            }
        }

        #[cfg(feature = "orm-postgres")]
        if table.contains_key("orm-postgres") {
            let config: PostgresOrmSettings = crate::state::extract(table, "orm-postgres")?;
            let url = format!(
                "postgres://{}:{}@{}:{}/{}",
                Self::encode_component(&config.user),
                Self::encode_component(&config.pass),
                config.host,
                config.port,
                Self::encode_component(&config.name)
            );
            if settings.replace((url, config.token, config.url)).is_some() {
                return Err(crate::Error::custom(
                    50001,
                    "configure exactly one enabled ORM driver",
                ));
            }
        }

        settings.ok_or_else(|| {
            crate::Error::custom(
                50001,
                "configure an enabled [orm-sqlite] or [orm-postgres] section",
            )
        })
    }

    #[cfg(feature = "orm-postgres")]
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

    pub fn model_metadata(&self) -> Vec<&'static ModelInfo> {
        self.inner.model_metadata()
    }

    pub fn inner(&self) -> &afastorm::Orm {
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
            .or_else(|| self.inner.admin_token())
    }

    #[cfg(feature = "orm-admin")]
    pub fn admin_url(&self) -> Option<&str> {
        self.admin_url.as_deref().or_else(|| self.inner.admin_url())
    }

    #[cfg(feature = "orm-admin")]
    pub(crate) fn admin_service(&self, url: &str) -> crate::Result<afast::Service> {
        if !self
            .model_metadata()
            .iter()
            .any(|model| model.admin_enabled)
        {
            return Err(crate::Error::custom(
                50001,
                "ORM admin service requires at least one registered admin model",
            ));
        }
        super::admin::service(url)
    }
}

impl Deref for Orm {
    type Target = afastorm::Orm;

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
                let token = self.state.orm.admin_token().ok_or_else(|| {
                    crate::Error::custom(
                        50001,
                        "ORM admin is enabled but no `token` is configured or overridden",
                    )
                })?;
                let auth = super::admin::AdminAuth::new(token)?;
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
