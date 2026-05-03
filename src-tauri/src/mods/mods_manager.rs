use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::path::Path;
use tauri::State;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::server::server_manager::ServerManager;

#[derive(Serialize)]
pub struct ModSearchResponse {
    pub items: Vec<ModSearchItem>,
    pub total: u64,
}

#[derive(Serialize)]
pub struct ModSearchItem {
    pub provider: String,
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: Option<String>,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub project_url: String,
    pub side: String,
}

#[derive(Serialize)]
pub struct ModInstallResponse {
    pub installed: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct InstalledModRecord {
    pub provider: String,
    pub id: String,
    pub filename: String,
}

#[derive(Deserialize)]
struct ModrinthSearchResponse {
    hits: Vec<ModrinthHit>,
    total_hits: u64,
}

#[derive(Deserialize)]
struct ModrinthHit {
    project_id: String,
    slug: String,
    title: String,
    description: String,
    author: String,
    downloads: u64,
    icon_url: Option<String>,
    client_side: String,
    server_side: String,
}

#[derive(Deserialize, Clone)]
struct ModrinthVersion {
    id: String,
    project_id: String,
    dependencies: Vec<ModrinthDependency>,
    files: Vec<ModrinthFile>,
}

#[derive(Deserialize, Clone)]
struct ModrinthDependency {
    version_id: Option<String>,
    project_id: Option<String>,
    dependency_type: String,
}

#[derive(Deserialize, Clone)]
struct ModrinthFile {
    url: String,
    filename: String,
    primary: bool,
}

#[tauri::command]
pub async fn search_mods(
    provider: String,
    query: Option<String>,
    game_version: Option<String>,
    loader: Option<String>,
    index: Option<u32>,
    limit: Option<u32>,
) -> Result<ModSearchResponse, String> {
    let limit = limit.unwrap_or(20).clamp(1, 50);
    let index = index.unwrap_or(0);
    let query = query.unwrap_or_default();
    let client = Client::builder()
        .user_agent("moiCR/LocalCraft")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    match provider.as_str() {
        "modrinth" => search_modrinth(&client, &query, game_version, loader, index, limit).await,
        other => Err(format!("Unsupported mod provider: {}", other)),
    }
}

async fn search_modrinth(
    client: &Client,
    query: &str,
    game_version: Option<String>,
    loader: Option<String>,
    index: u32,
    limit: u32,
) -> Result<ModSearchResponse, String> {
    let mut facets = vec![vec!["project_type:mod".to_string()]];

    if let Some(version) = game_version.filter(|value| !value.trim().is_empty()) {
        facets.push(vec![format!("versions:{}", version)]);
    }

    if let Some(loader) = loader.filter(|value| !value.trim().is_empty()) {
        facets.push(vec![format!("categories:{}", loader.to_lowercase())]);
    }

    let mut url = Url::parse("https://api.modrinth.com/v2/search")
        .map_err(|e| format!("Failed to build Modrinth URL: {}", e))?;
    url.query_pairs_mut()
        .append_pair("query", query)
        .append_pair("facets", &json!(facets).to_string())
        .append_pair("index", "downloads")
        .append_pair("offset", &index.to_string())
        .append_pair("limit", &limit.to_string());

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Modrinth mods: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Modrinth API returned {}", response.status()));
    }

    let body = response
        .json::<ModrinthSearchResponse>()
        .await
        .map_err(|e| format!("Failed to parse Modrinth response: {}", e))?;

    Ok(ModSearchResponse {
        total: body.total_hits,
        items: body
            .hits
            .into_iter()
            .map(|item| ModSearchItem {
                provider: "modrinth".to_string(),
                id: item.project_id,
                slug: item.slug.clone(),
                title: item.title,
                description: item.description,
                author: Some(item.author),
                downloads: item.downloads,
                icon_url: item.icon_url,
                project_url: format!("https://modrinth.com/mod/{}", item.slug),
                side: modrinth_side(&item.client_side, &item.server_side),
            })
            .collect(),
    })
}

#[tauri::command]
pub async fn install_mod(
    server_id: String,
    provider: String,
    mod_id: String,
    game_version: String,
    loader: Option<String>,
    state: State<'_, ServerManager>,
) -> Result<ModInstallResponse, String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers.get(&server_id).cloned()
    }
    .ok_or("Server not found".to_string())?;

    let mods_dir = server.get_path()?.join("mods");
    fs::create_dir_all(&mods_dir)
        .await
        .map_err(|e| format!("Failed to create mods directory: {}", e))?;

    let client = Client::builder()
        .user_agent("moiCR/LocalCraft")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut installed = Vec::new();
    let mut installed_records = read_installed_mods_file(&mods_dir).await?;

    match provider.as_str() {
        "modrinth" => {
            install_modrinth_tree(
                &client,
                &mods_dir,
                &mod_id,
                &game_version,
                loader.as_deref(),
                &mut installed,
                &mut installed_records,
            )
            .await?;
        }
        other => return Err(format!("Unsupported mod provider: {}", other)),
    }

    write_installed_mods_file(&mods_dir, &installed_records).await?;
    Ok(ModInstallResponse { installed })
}

#[tauri::command]
pub async fn get_installed_mods(
    server_id: String,
    state: State<'_, ServerManager>,
) -> Result<Vec<InstalledModRecord>, String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers.get(&server_id).cloned()
    }
    .ok_or("Server not found".to_string())?;

    let mods_dir = server.get_path()?.join("mods");
    read_installed_mods_file(&mods_dir).await
}

async fn install_modrinth_tree(
    client: &Client,
    mods_dir: &Path,
    project_id: &str,
    game_version: &str,
    loader: Option<&str>,
    installed: &mut Vec<String>,
    installed_records: &mut Vec<InstalledModRecord>,
) -> Result<(), String> {
    let mut queue = vec![project_id.to_string()];
    let mut visited = HashSet::new();

    while let Some(current_project_id) = queue.pop() {
        if !visited.insert(current_project_id.clone()) {
            continue;
        }

        let version =
            get_modrinth_project_version(client, &current_project_id, game_version, loader).await?;
        let filename = download_modrinth_version(client, mods_dir, &version, installed).await?;
        upsert_installed_record(
            installed_records,
            "modrinth",
            &version.project_id,
            &filename,
        );

        for dependency in version
            .dependencies
            .iter()
            .filter(|dep| dep.dependency_type == "required")
        {
            if let Some(version_id) = &dependency.version_id {
                let dependency_version = get_modrinth_version(client, version_id).await?;
                if visited.insert(dependency_version.project_id.clone()) {
                    let filename =
                        download_modrinth_version(client, mods_dir, &dependency_version, installed)
                            .await?;
                    upsert_installed_record(
                        installed_records,
                        "modrinth",
                        &dependency_version.project_id,
                        &filename,
                    );
                    for nested in dependency_version
                        .dependencies
                        .iter()
                        .filter(|dep| dep.dependency_type == "required")
                    {
                        if let Some(project_id) = &nested.project_id {
                            queue.push(project_id.clone());
                        }
                    }
                }
            } else if let Some(project_id) = &dependency.project_id {
                queue.push(project_id.clone());
            }
        }
    }

    Ok(())
}

async fn get_modrinth_project_version(
    client: &Client,
    project_id: &str,
    game_version: &str,
    loader: Option<&str>,
) -> Result<ModrinthVersion, String> {
    let mut url = Url::parse(&format!(
        "https://api.modrinth.com/v2/project/{}/version",
        project_id
    ))
    .map_err(|e| format!("Failed to build Modrinth versions URL: {}", e))?;

    {
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("game_versions", &json!([game_version]).to_string())
            .append_pair("include_changelog", "false");
        if let Some(loader) = loader.filter(|value| !value.trim().is_empty()) {
            pairs.append_pair("loaders", &json!([loader]).to_string());
        }
    }

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Modrinth versions: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Modrinth versions API returned {}", response.status()));
    }

    response
        .json::<Vec<ModrinthVersion>>()
        .await
        .map_err(|e| format!("Failed to parse Modrinth versions: {}", e))?
        .into_iter()
        .next()
        .ok_or_else(|| format!("No compatible Modrinth version found for {}", project_id))
}

async fn get_modrinth_version(client: &Client, version_id: &str) -> Result<ModrinthVersion, String> {
    let response = client
        .get(format!("https://api.modrinth.com/v2/version/{}", version_id))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Modrinth version: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Modrinth version API returned {}", response.status()));
    }

    response
        .json::<ModrinthVersion>()
        .await
        .map_err(|e| format!("Failed to parse Modrinth version: {}", e))
}

async fn download_modrinth_version(
    client: &Client,
    mods_dir: &Path,
    version: &ModrinthVersion,
    installed: &mut Vec<String>,
) -> Result<String, String> {
    let file = version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| format!("Modrinth version {} has no downloadable files", version.id))?;

    download_file(client, &file.url, mods_dir, &file.filename, installed).await?;
    Ok(file.filename.clone())
}

async fn download_file(
    client: &Client,
    url: &str,
    mods_dir: &Path,
    filename: &str,
    installed: &mut Vec<String>,
) -> Result<(), String> {
    let target = mods_dir.join(filename);
    if target.exists() {
        return Ok(());
    }

    let bytes = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to download {}: {}", filename, e))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read {}: {}", filename, e))?;

    let mut file = fs::File::create(&target)
        .await
        .map_err(|e| format!("Failed to create {}: {}", filename, e))?;
    file.write_all(&bytes)
        .await
        .map_err(|e| format!("Failed to write {}: {}", filename, e))?;

    installed.push(filename.to_string());
    Ok(())
}

async fn read_installed_mods_file(mods_dir: &Path) -> Result<Vec<InstalledModRecord>, String> {
    let metadata_path = mods_dir.join(".localcraft-mods.json");
    if !metadata_path.exists() {
        return Ok(Vec::new());
    }

    let bytes = fs::read(&metadata_path)
        .await
        .map_err(|e| format!("Failed to read installed mods metadata: {}", e))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| format!("Failed to parse installed mods metadata: {}", e))
}

async fn write_installed_mods_file(
    mods_dir: &Path,
    records: &[InstalledModRecord],
) -> Result<(), String> {
    let metadata_path = mods_dir.join(".localcraft-mods.json");
    let json = serde_json::to_vec_pretty(records)
        .map_err(|e| format!("Failed to serialize installed mods metadata: {}", e))?;
    fs::write(metadata_path, json)
        .await
        .map_err(|e| format!("Failed to write installed mods metadata: {}", e))
}

fn upsert_installed_record(
    records: &mut Vec<InstalledModRecord>,
    provider: &str,
    id: &str,
    filename: &str,
) {
    if let Some(record) = records
        .iter_mut()
        .find(|record| record.provider == provider && record.id == id)
    {
        record.filename = filename.to_string();
        return;
    }

    records.push(InstalledModRecord {
        provider: provider.to_string(),
        id: id.to_string(),
        filename: filename.to_string(),
    });
}

pub fn remove_installed_mod_record_by_filename(
    mods_dir: &Path,
    filename: &str,
) -> Result<(), String> {
    let metadata_path = mods_dir.join(".localcraft-mods.json");
    if !metadata_path.exists() {
        return Ok(());
    }

    let bytes = std::fs::read(&metadata_path)
        .map_err(|e| format!("Failed to read installed mods metadata: {}", e))?;
    let mut records: Vec<InstalledModRecord> = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Failed to parse installed mods metadata: {}", e))?;
    let original_len = records.len();

    records.retain(|record| record.filename != filename);

    if records.len() == original_len {
        return Ok(());
    }

    let json = serde_json::to_vec_pretty(&records)
        .map_err(|e| format!("Failed to serialize installed mods metadata: {}", e))?;
    std::fs::write(metadata_path, json)
        .map_err(|e| format!("Failed to write installed mods metadata: {}", e))
}

fn modrinth_side(client_side: &str, server_side: &str) -> String {
    let client = client_side == "required" || client_side == "optional";
    let server = server_side == "required" || server_side == "optional";

    match (client, server) {
        (true, true) => "Client and Server",
        (true, false) => "Client",
        (false, true) => "Server",
        _ => "Client and Server",
    }
    .to_string()
}
