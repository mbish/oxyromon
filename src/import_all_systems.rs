#[cfg(feature = "chd")]
use super::config::*;
use super::database::*;
#[cfg(feature = "cso")]
use super::SimpleResult;
use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, ArgMatches, Command};
use indicatif::ProgressBar;
use sqlx::sqlite::SqliteConnection;
use std::path::PathBuf;
#[cfg(feature = "ird")]
use super::import_roms::import_system;
use strum::VariantNames;

pub fn subcommand() -> Command {
    Command::new("import-all-systems")
        .about("Validate and import ROM files for all systems")
        .arg(
            Arg::new("ROMS")
                .help("Set the ROM files or directories to import")
                .required(true)
                .num_args(1..)
                .index(1)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("HASH")
                .short('a')
                .long("hash")
                .help("Set the hash algorithm")
                .required(false)
                .num_args(1)
                .value_parser(PossibleValuesParser::new(HashAlgorithm::VARIANTS)),
        )
        .arg(
            Arg::new("NOTRASH")
                .short('n')
                .long("no-trash")
                .help("Do not trash unmatched files")
                .required(false)
                .action(ArgAction::SetTrue),
        )
}

pub async fn main(
    connection: &mut SqliteConnection,
    matches: &ArgMatches,
    progress_bar: &ProgressBar,
) -> SimpleResult<()> {
    let romfile_paths: Vec<&PathBuf> = matches.get_many::<PathBuf>("ROMS").unwrap().collect();
    let no_trash = matches.get_flag("NOTRASH");
    let hash_algorithm = match matches.get_one::<String>("HASH").map(String::as_str) {
        Some("CRC") => HashAlgorithm::Crc,
        Some("MD5") => HashAlgorithm::Md5,
        Some(&_) | None => {
            match find_setting_by_key(connection, "HASH_ALGORITHM")
                .await
                .unwrap()
                .value
                .as_deref()
            {
                Some("CRC") => HashAlgorithm::Crc,
                Some("MD5") => HashAlgorithm::Md5,
                Some(&_) | None => bail!("Not possible"),
            }
        }
    };
    for system in find_systems(connection).await {
        let header = find_header_by_system_id(connection, system.id).await;
        progress_bar.println(&format!("Import System \"{:?}\"", &system.name));
        import_system(
            connection,
            progress_bar,
            &system,
            &header,
            &romfile_paths,
            &hash_algorithm,
            no_trash,
        ).await?;
    }
    Ok(())
}
