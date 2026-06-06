use std::{collections::BTreeMap, env::var, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use log::info;
use reqwest::{get, Client};
use semver::Version;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{from_slice, to_string_pretty};
use tokio::fs::{create_dir_all, read_to_string, write};
use tracing_subscriber::fmt::init;

#[derive(Parser)]
#[clap(about, version)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Download the latest data and build an emoji table
    BuildEmojiTable,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Emoji {
    pub name: String,
    pub unified: String,
    pub short_name: String,
    pub short_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Tag {
    #[serde(rename = "ref", deserialize_with = "deserialize_tag_ref")]
    version: Version,
}

fn deserialize_tag_ref<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?.replace("refs/tags/v", "");
    Ok(Version::parse(&s).unwrap_or_else(|_| Version::new(0, 0, 0)))
}

#[tokio::main]
async fn main() -> Result<()> {
    init();
    let args = Args::parse();
    match args.command {
        Command::BuildEmojiTable => {
            let local_version_file = PathBuf::from(var("CARGO_WORKSPACE_DIR")?)
                .join("assets")
                .join("emoji-data-version");
            info!("Checking local version file: {local_version_file:?}");

            let current: Version = match read_to_string(&local_version_file).await {
                Ok(s) => s.trim().parse()?,
                Err(_) => Version::new(0, 0, 0),
            };
            info!("Current version: {current}");

            let latest = get_latest_version().await;
            info!("Latest version: {latest}");

            let output_dir = PathBuf::from(var("CARGO_WORKSPACE_DIR")?).join("assets");
            let table_file = output_dir.join("emoji.json");
            let reverse_table_file = output_dir.join("emoji_reverse.json");

            if current == latest && table_file.exists() && reverse_table_file.exists() {
                info!("The data is up-to-date. Finished.");
                return Ok(());
            }
            info!("The latest version is available: {current} → {latest}. Downloading.");

            let blob = get("https://raw.githubusercontent.com/iamcal/emoji-data/master/emoji.json")
                .await?
                .bytes()
                .await?;
            info!(
                "Downloaded the emoji data: {:.2} MB",
                blob.len() as f64 / 1024.0 / 1024.0
            );

            let mut emojis = BTreeMap::new();
            let mut reverse = BTreeMap::new();
            for emoji in from_slice::<Vec<Emoji>>(&blob)? {
                let unicode = to_emoji(&emoji.unified);
                // Only single-codepoint emoji (optionally followed by FE0F) are reversible;
                // keycaps, flags, and ZWJ sequences are truncated by to_emoji and would map
                // plain text like '#' or lone regional indicators to shortcodes. The
                // canonical short_name wins on first-codepoint collisions.
                if is_reversible(&emoji.unified) {
                    reverse
                        .entry(unicode.clone())
                        .or_insert(format!(":{}:", emoji.short_name));
                }
                for name in emoji.short_names {
                    emojis.insert(format!(":{name}:"), unicode.clone());
                }
            }

            create_dir_all(&output_dir).await?;
            info!("Created the output directory: {output_dir:?}");

            write(&table_file, to_string_pretty(&emojis)?).await?;
            info!("Finished writing the emoji table: {table_file:?}");

            write(&reverse_table_file, to_string_pretty(&reverse)?).await?;
            info!("Finished writing the reverse emoji table: {reverse_table_file:?}");

            write(&local_version_file, &latest.to_string()).await?;
            info!("Updated the local version file: {local_version_file:?}");

            info!("Finished building the emoji table.");
        }
    }

    Ok(())
}

/// A unified sequence is reversible when it is a single codepoint, optionally followed by
/// a variation selector (FE0F). Anything longer is truncated by `to_emoji`.
fn is_reversible(unified: &str) -> bool {
    let mut parts = unified.split('-');
    parts.next();
    parts.all(|c| c == "FE0F")
}

#[inline(always)]
fn to_emoji(s: &str) -> String {
    s.split('-')
        .take(1)
        .map(|c| char::from_u32(u32::from_str_radix(c, 16).unwrap()).unwrap())
        .collect::<String>()
}

async fn get_latest_version() -> Version {
    // return 0.0.0 if any error occurs while fetching and parsing the version data from GitHub
    let result: Result<Version> = async {
        let mut blob: Vec<Tag> = Client::builder()
            .user_agent("slack_emojify/0.1.1")
            .build()?
            .get("https://api.github.com/repos/iamcal/emoji-data/git/refs/tags")
            .send()
            .await?
            .json()
            .await?;
        blob.sort_by(|a, b| b.version.cmp(&a.version));
        Ok(blob.first().unwrap().version.clone())
    }
    .await;

    result.unwrap_or_else(|_| Version::new(0, 0, 0))
}
