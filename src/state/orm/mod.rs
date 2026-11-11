mod config;

#[cfg(feature = "orm-admin")]
pub(crate) mod admin;

#[cfg(feature = "orm-admin")]
pub use admin::{AFasterAdmin, AdminFieldInfo, AdminModelInfo, AdminModelRegistry};
#[cfg(feature = "orm-admin")]
pub use afaster_derive::AFasterAdmin;

#[cfg(feature = "orm-migrate")]
mod command;

pub use config::{AFasterOrmExt, Orm};

#[cfg(feature = "orm-migrate")]
pub(crate) use command::OrmMigrateCommands;
