use std::ops::Deref;
#[cfg(feature = "orm-admin")]
use std::sync::Arc;

#[cfg(feature = "orm-admin")]
use super::admin::{AdminModelRegistry, AdminTokenGenerator, AdminTokenVerifier};
use afastorm::{ModelInfo, ModelRegistry};
use serde::Deserialize;

#[cfg(feature = "orm-sqlite")]
pub(crate) type Database = afastorm::sqlx::Sqlite;
#[cfg(feature = "orm-postgres")]
pub(crate) type Database = afastorm::sqlx::Postgres;
#[cfg(feature = "orm-mysql")]
pub(crate) type Database = afastorm::sqlx::MySql;

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

#[cfg(feature = "orm-mysql")]
#[derive(Deserialize)]
struct MysqlOrmSettings {
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
        let (database_url, token, admin_url) = Self::database_settings(table)?;
        #[cfg(not(feature = "orm-admin"))]
        let _ = (&token, &admin_url);
        let inner = afastorm::Orm::<Database>::connect(database_url)
            .await
            .map_err(|error| {
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

    fn database_settings(
        table: &toml::Table,
    ) -> crate::Result<(String, Option<String>, Option<String>)> {
        #[cfg(feature = "orm-sqlite")]
        {
            if table.contains_key("orm-postgres") {
                return Err(crate::Error::custom(
                    50001,
                    "this application was compiled with the SQLite ORM; remove [orm-postgres]",
                ));
            }
            if table.contains_key("orm-mysql") {
                return Err(crate::Error::custom(
                    50001,
                    "this application was compiled with the SQLite ORM; remove [orm-mysql]",
                ));
            }
            if !table.contains_key("orm-sqlite") {
                return Err(crate::Error::custom(
                    50001,
                    "configure the compile-time SQLite database in [orm-sqlite]",
                ));
            }
            let config: SqliteOrmSettings = crate::state::extract(table, "orm-sqlite")?;
            let url = if config.path == ":memory:" {
                "sqlite::memory:".to_owned()
            } else if config.path.starts_with("sqlite:") {
                config.path
            } else {
                format!("sqlite://{}?mode=rwc", config.path)
            };
            return Ok((url, config.token, config.url));
        }

        #[cfg(feature = "orm-postgres")]
        {
            if table.contains_key("orm-sqlite") {
                return Err(crate::Error::custom(
                    50001,
                    "this application was compiled with the PostgreSQL ORM; remove [orm-sqlite]",
                ));
            }
            if table.contains_key("orm-mysql") {
                return Err(crate::Error::custom(
                    50001,
                    "this application was compiled with the PostgreSQL ORM; remove [orm-mysql]",
                ));
            }
            if !table.contains_key("orm-postgres") {
                return Err(crate::Error::custom(
                    50001,
                    "configure the compile-time PostgreSQL database in [orm-postgres]",
                ));
            }
            let config: PostgresOrmSettings = crate::state::extract(table, "orm-postgres")?;
            let url = format!(
                "postgres://{}:{}@{}:{}/{}",
                Self::encode_component(&config.user),
                Self::encode_component(&config.pass),
                config.host,
                config.port,
                Self::encode_component(&config.name)
            );
            return Ok((url, config.token, config.url));
        }

        #[cfg(feature = "orm-mysql")]
        {
            if table.contains_key("orm-sqlite") {
                return Err(crate::Error::custom(
                    50001,
                    "this application was compiled with the MySQL ORM; remove [orm-sqlite]",
                ));
            }
            if table.contains_key("orm-postgres") {
                return Err(crate::Error::custom(
                    50001,
                    "this application was compiled with the MySQL ORM; remove [orm-postgres]",
                ));
            }
            if !table.contains_key("orm-mysql") {
                return Err(crate::Error::custom(
                    50001,
                    "configure the compile-time MySQL database in [orm-mysql]",
                ));
            }
            let config: MysqlOrmSettings = crate::state::extract(table, "orm-mysql")?;
            let url = format!(
                "mysql://{}:{}@{}:{}/{}",
                Self::encode_component(&config.user),
                Self::encode_component(&config.pass),
                config.host,
                config.port,
                Self::encode_component(&config.name)
            );
            return Ok((url, config.token, config.url));
        }
        #[allow(unreachable_code)]
        Err(crate::Error::custom(
            50001,
            "no ORM database feature is enabled",
        ))
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
