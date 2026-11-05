pub(crate) struct OrmMigrateCommands;

impl crate::CommandModule for OrmMigrateCommands {
    fn name(&self) -> &'static str {
        "orm"
    }

    fn command(&self) -> clap::Command {
        use clap::{Arg, Command};

        Command::new("orm")
            .about("ORM schema and migration commands")
            .subcommand_required(true)
            .arg_required_else_help(true)
            .subcommand(
                Command::new("makemigrations")
                    .about("Generate a migration from registered model metadata")
                    .arg(Arg::new("name").required(true))
                    .arg(
                        Arg::new("dir")
                            .long("dir")
                            .default_value("migrations")
                            .help("Migration directory"),
                    ),
            )
            .subcommand(
                Command::new("migrate")
                    .about("Apply pending migrations")
                    .arg(
                        Arg::new("dir")
                            .long("dir")
                            .default_value("migrations")
                            .help("Migration directory"),
                    ),
            )
    }

    fn run<'a>(
        &'a self,
        app: &'a crate::AFaster,
        matches: &'a clap::ArgMatches,
    ) -> crate::CommandFuture<'a> {
        Box::pin(async move {
            let Some((command, matches)) = matches.subcommand() else {
                return Err(crate::Error::custom(40001, "missing ORM command"));
            };
            let directory = matches
                .get_one::<String>("dir")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("migrations"));

            match command {
                "makemigrations" => {
                    let name = matches
                        .get_one::<String>("name")
                        .ok_or_else(|| crate::Error::custom(40001, "missing migration name"))?;
                    if app.state.orm.model_metadata().is_empty() {
                        return Err(crate::Error::custom(
                            40001,
                            "orm makemigrations requires registered models",
                        ));
                    }
                    let snapshot = app.state.orm.schema_snapshot().map_err(command_error)?;
                    let created =
                        afastorm::migration::create_schema_migration(directory, name, &snapshot)
                            .map_err(command_error)?;
                    if !created {
                        println!("no schema changes found");
                    }
                }
                "migrate" => {
                    let applied =
                        afastorm::migration::apply_migrations(app.state.orm.inner(), directory)
                            .await
                            .map_err(command_error)?;
                    if applied.is_empty() {
                        println!("no pending migrations");
                    } else {
                        for (version, name) in applied {
                            println!("applied {version} {name}");
                        }
                    }
                }
                _ => {
                    return Err(crate::Error::custom(
                        40001,
                        format!("unknown ORM command `{command}`"),
                    ));
                }
            }
            Ok(())
        })
    }
}

fn command_error(error: impl std::fmt::Display) -> crate::Error {
    crate::Error::custom(50001, format!("ORM migration command failed: {error}"))
}

#[cfg(all(test, feature = "orm-sqlite"))]
mod tests {
    use super::*;
    use crate::CommandModule;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn migrate_command_applies_files_without_starting_the_server() {
        let directory = std::env::temp_dir().join(format!(
            "afaster-orm-command-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).expect("create command test directory");
        let config_path = directory.join("config.toml");
        std::fs::write(
            &config_path,
            r#"
[backend]
host = "127.0.0.1"
port = 6080

[orm-sqlite]
path = ":memory:"
"#,
        )
        .expect("write config");
        let migrations = directory.join("migrations");
        std::fs::create_dir_all(&migrations).expect("create migrations directory");
        std::fs::write(
            migrations.join("0001_create_probe.up.sql"),
            "CREATE TABLE command_probe (id INTEGER PRIMARY KEY);",
        )
        .expect("write migration");

        let app = crate::AFaster::new(config_path.to_string_lossy().into_owned())
            .await
            .expect("initialize app");
        let command = OrmMigrateCommands;
        let matches = clap::Command::new("afaster")
            .subcommand(command.command())
            .try_get_matches_from([
                "afaster",
                "orm",
                "migrate",
                "--dir",
                migrations.to_str().expect("migration path is UTF-8"),
            ])
            .expect("parse migrate command");
        let (_, matches) = matches.subcommand().expect("orm subcommand");

        command
            .run(&app, matches)
            .await
            .expect("run migration command");
        let count: (i64,) = app
            .state()
            .orm
            .query_as("SELECT COUNT(*) FROM command_probe")
            .fetch_one()
            .await
            .expect("verify migration table");
        assert_eq!(count.0, 0);

        drop(app);
        std::fs::remove_dir_all(directory).expect("remove command test files");
    }
}
