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

            if current == latest {
                info!("The data is up-to-date. Finished.");
                return Ok(());
            }
            info!("The latest version is available: {current} → {latest}. Downloading.");

            let blob = get("https://raw.githubusercontent.com/iamcal/emoji-data/master/emoji.json")
                .await?
                .bytes()
                .await?;
            info!("Downloaded the emoji data: {:.2} MB", blob.len() as f64 / 1024.0 / 1024.0);

            let emojis = from_slice::<Vec<Emoji>>(&blob)?
                .into_iter()
                .flat_map(|emoji| {
                    emoji
                        .short_names
                        .into_iter()
                        .map(|name| (format!(":{name}:"), to_emoji(&emoji.unified)))
                        .collect::<Vec<(String, String)>>()
                })
                .collect::<BTreeMap<String, String>>();

            let output_dir = PathBuf::from(var("CARGO_WORKSPACE_DIR")?).join("assets");
            create_dir_all(&output_dir).await?;
            info!("Created the output directory: {output_dir:?}");

            let output_file = output_dir.join("emoji.json");
            write(&output_file, to_string_pretty(&emojis)?).await?;
            info!("Finished writing the emoji table: {output_file:?}");

            write(&local_version_file, &latest.to_string()).await?;
            info!("Updated the local version file: {local_version_file:?}");

            info!("Finished building the emoji table.");
        }
    }

    Ok(())
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
