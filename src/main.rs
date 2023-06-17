use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use bytes::Bytes;
use clap::Parser;
use digest::Digest;
use futures::prelude::*;
use reqwest::Url;
use structs::{Index, IndexFile, Pack};
use toml::Table;

mod structs;

const MAXIMUM_CONCURRENT_OPERATIONS: usize = 8;

/// Generate a packwiz modpack based on another modpack
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to pack.toml (or its parent directory)
    /// Current working directory by default
    #[arg(value_name = "pack.toml")]
    pack_toml: Option<PathBuf>,
    /// Output path to generated modpack
    /// Must be an empty directory
    /// Will be created if doesn't exist
    #[arg(long, short)]
    output: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut pack_toml = args.pack_toml.unwrap_or(std::env::current_dir()?);
    if !pack_toml.ends_with("pack.toml") {
        pack_toml.push("pack.toml");
    }
    if !pack_toml.exists() {
        anyhow::bail!("pack.toml doesn't exist!");
    }

    if !args.output.exists() {
        fs::create_dir(&args.output)?;
    }

    // check if folder is empty
    if args.output.is_dir() && fs::read_dir(&args.output)?.next().is_some() {
        anyhow::bail!("directory is not empty!");
    }

    // ---- local pack initialization ----
    let pack: Pack = toml::from_str(&fs::read_to_string(pack_toml.clone())?)?;
    let pack_base = pack_toml.parent().unwrap();
    let index: Index = toml::from_str(&fs::read_to_string(
        pack_base.join(pack.index.file.clone()),
    )?)?;

    // ---- parent pack initialization ----
    let parent_pack_url = Url::parse(
        &pack
            .options
            .expect("Pack must have parent specified!")
            .parent
            .expect("Pack must have parent specified!"),
    )?;
    let parent_pack_toml = reqwest::get(parent_pack_url.clone()).await?.text().await?;
    let parent_pack: Pack = toml::from_str(&parent_pack_toml)?;
    let mut parent_index_url = parent_pack_url.clone();
    parent_index_url
        .path_segments_mut()
        .expect("cannot be base")
        .pop() // remove pack.toml from the end of url
        .push(&parent_pack.index.file); // add index.toml to the end of url
    let parent_index: Index = toml::from_str(std::str::from_utf8(
        &download_and_verify_hash(
            parent_index_url,
            parent_pack.index.hash,
            parent_pack.index.hash_format,
        )
        .await?,
    )?)?;

    // ---- combined indexes ----
    let mut final_index_files: HashMap<String, IndexFile> = HashMap::new();
    for index_file in parent_index.files.unwrap() {
        let mut index_file_clone = index_file.clone();
        if parent_index.hash_format != index.hash_format && index_file_clone.hash_format.is_none() {
            index_file_clone.hash_format = Some(parent_index.hash_format);
        }
        final_index_files.insert(index_file.file.clone(), index_file_clone);
    }
    for index_file in index.files.unwrap() {
        final_index_files.insert(index_file.file.clone(), index_file.clone());
    }
    // ---- Download files to new combined modpack ----
    // refrences for use in closure
    let output: &Path = &args.output;
    let base_url = &parent_pack_url;
    stream::iter(final_index_files.values())
        .map(Ok::<&IndexFile, anyhow::Error>)
        .try_for_each_concurrent(MAXIMUM_CONCURRENT_OPERATIONS, |indexfile| async move {
            if pack_base.join(indexfile.file.clone()).exists() {
                let output_file = output.join(&indexfile.file);
                fs::create_dir_all(output_file.parent().unwrap())?;
                fs::copy(pack_base.join(&indexfile.file), output_file)?;
                // no hash verification for now
            } else {
                let mut file_url = base_url.clone();
                file_url
                    .path_segments_mut()
                    .expect("cannot be base")
                    .pop()
                    .push(&indexfile.file);
                let bytes = download_and_verify_hash(
                    file_url,
                    indexfile.hash.clone(),
                    indexfile.hash_format.unwrap_or(parent_index.hash_format),
                )
                .await?;
                let output_file = output.join(&indexfile.file);
                fs::create_dir_all(output_file.parent().unwrap())?;
                fs::write(output_file, bytes)?;
            }
            Ok(())
        })
        .await?;

    let final_index = Index {
        files: Some(final_index_files.into_values().collect()),
        hash_format: index.hash_format,
    };
    let final_index_toml = toml::to_string(&final_index)?;
    let final_index_hash = format!(
        "{:x}",
        sha2::Sha256::new()
            .chain_update(final_index_toml.as_bytes())
            .finalize()
    );

    fs::write(args.output.join(pack.index.file), final_index_toml)?;

    // not using structs::Pack here so i don't have to keep up with packwiz's format as i don't use most of it
    let mut final_pack: Table = toml::from_str(&fs::read_to_string(pack_toml)?)?;
    final_pack["options"]
        .as_table_mut()
        .unwrap()
        .remove("parent");
    final_pack["index"].as_table_mut().unwrap()["hash"] = toml::Value::String(final_index_hash);
    final_pack["index"].as_table_mut().unwrap()["hash-format"] =
        toml::Value::String("sha256".to_string());
    let final_pack_toml = toml::to_string(&final_pack)?;

    fs::write(args.output.join("pack.toml"), final_pack_toml)?;
    Ok(())
}

async fn download_and_verify_hash(
    url: Url,
    hash: String,
    hash_format: structs::HashFormat,
) -> anyhow::Result<Bytes> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    let file_hash: String = hash_format.get_hash(&bytes);
    anyhow::ensure!(file_hash == hash.to_lowercase(), anyhow!("Bad hash!"));
    Ok(bytes)
}
