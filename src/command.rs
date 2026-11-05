use std::collections::HashSet;
use std::{future::Future, pin::Pin};

use clap::{ArgMatches, Command};

use crate::{AFaster, Result};

pub type CommandFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

/// A feature module that contributes one top-level application subcommand.
pub trait CommandModule: Send + Sync {
    /// Must equal the name returned by `command().get_name()`.
    fn name(&self) -> &'static str;

    fn command(&self) -> Command;

    fn run<'a>(&'a self, app: &'a AFaster, matches: &'a ArgMatches) -> CommandFuture<'a>;
}

pub(crate) fn builtin_modules() -> Vec<Box<dyn CommandModule>> {
    vec![
        #[cfg(feature = "orm-migrate")]
        Box::new(crate::state::orm::OrmMigrateCommands),
    ]
}

impl AFaster {
    /// Registers an application-specific command module.
    pub fn command_module(mut self, module: impl CommandModule + 'static) -> Self {
        self.commands.push(Box::new(module));
        self
    }

    /// Executes a matching command before application startup.
    ///
    /// Call this after registering module state and models, but before startup
    /// work such as seeding data. Returns `false` when no command was requested.
    pub async fn run_command_if_requested(&self) -> Result<bool> {
        let mut command = Command::new("afaster")
            .about("AFaster application commands")
            .subcommand_required(false)
            .arg_required_else_help(false);
        let mut names = HashSet::new();
        for module in &self.commands {
            let subcommand = module.command();
            if subcommand.get_name() != module.name() {
                return Err(crate::Error::custom(
                    50001,
                    format!(
                        "command module name `{}` does not match subcommand `{}`",
                        module.name(),
                        subcommand.get_name()
                    ),
                ));
            }
            if !names.insert(module.name()) {
                return Err(crate::Error::custom(
                    50001,
                    format!("duplicate command module `{}`", module.name()),
                ));
            }
            command = command.subcommand(subcommand);
        }

        let matches = match command.try_get_matches_from(std::env::args_os()) {
            Ok(matches) => matches,
            Err(error)
                if matches!(
                    error.kind(),
                    clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
                ) =>
            {
                error.print().map_err(|error| {
                    crate::Error::custom(50001, format!("failed to print command help: {error}"))
                })?;
                return Ok(true);
            }
            Err(error) => {
                return Err(crate::Error::custom(40001, error.to_string()));
            }
        };

        let Some((name, matches)) = matches.subcommand() else {
            return Ok(false);
        };
        let Some(module) = self.commands.iter().find(|module| module.name() == name) else {
            return Err(crate::Error::custom(
                50001,
                format!("command module `{name}` was parsed but is not registered"),
            ));
        };
        module.run(self, matches).await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "orm-migrate")]
    fn orm_migration_commands_are_registered_and_parse_arguments() {
        let modules = builtin_modules();
        let module = modules
            .iter()
            .find(|module| module.name() == "orm")
            .expect("ORM migration module is registered");
        let matches = Command::new("afaster")
            .subcommand(module.command())
            .try_get_matches_from([
                "afaster",
                "orm",
                "makemigrations",
                "add_profile",
                "--dir",
                "db/migrations",
            ])
            .expect("parse ORM command");
        let (_, orm_matches) = matches.subcommand().expect("ORM subcommand");
        let (_, command_matches) = orm_matches.subcommand().expect("makemigrations subcommand");
        assert_eq!(
            command_matches
                .get_one::<String>("name")
                .map(String::as_str),
            Some("add_profile")
        );
        assert_eq!(
            command_matches.get_one::<String>("dir").map(String::as_str),
            Some("db/migrations")
        );
    }

    #[test]
    #[cfg(feature = "orm-migrate")]
    fn orm_migration_help_is_available_from_the_root_command() {
        let module = builtin_modules()
            .into_iter()
            .find(|module| module.name() == "orm")
            .expect("ORM migration module is registered");
        let error = Command::new("afaster")
            .subcommand(module.command())
            .try_get_matches_from(["afaster", "orm", "--help"])
            .expect_err("help should be returned by clap");
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
    }
}
