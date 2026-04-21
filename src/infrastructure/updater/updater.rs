//! GitHub Release Update Checker
//!
//! Periodically queries the GitHub Releases API to detect new versions.
//! When an update is available, the user can trigger a download of the
//! installer `.exe` and launch it for a seamless upgrade.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::io::Write;

use log::{info, warn, error};
use serde::Deserialize;

// ── Configuration ────────────────────────────────────────────────────────

/// GitHub repository owner
const REPO_OWNER: &str = "Tailored-Alloys";
/// GitHub repository name
const REPO_NAME: &str = "toolpath_viewer";
/// GitHub Releases API endpoint
fn releases_url() -> String {
    format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        REPO_OWNER, REPO_NAME
    )
}

// ── Data types ───────────────────────────────────────────────────────────

/// Information about an available update
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    /// The new version tag (e.g. "v1.3.0")
    pub version: String,
    /// Release title / name
    pub title: String,
    /// Release notes (markdown body)
    pub body: String,
    /// Direct download URL for the installer .exe asset
    pub download_url: String,
    /// HTML URL to the release page on GitHub
    pub release_url: String,
}

/// Current state of the update system
#[derive(Debug, Clone)]
pub enum UpdateState {
    /// Not yet checked
    Idle,
    /// Currently checking for updates
    Checking,
    /// No update available (already on latest)
    UpToDate,
    /// A new version is available
    Available(UpdateInfo),
    /// Downloading the installer (progress 0.0–1.0)
    Downloading(f32),
    /// Download complete, ready to install
    ReadyToInstall(PathBuf),
    /// Installer has been launched, waiting for it to close us
    Installing,
    /// An error occurred
    Error(String),
}

/// Thread-safe shared update state
pub type SharedUpdateState = Arc<Mutex<UpdateState>>;

/// Create a new shared update state
pub fn new_shared_state() -> SharedUpdateState {
    Arc::new(Mutex::new(UpdateState::Idle))
}

// ── GitHub API response types ────────────────────────────────────────────

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

// ── Version parsing ──────────────────────────────────────────────────────

/// Parse a version string like "v1.2.0", "v1.2.0 (beta)", "1.2.0" into a semver::Version
fn parse_version(version_str: &str) -> Option<semver::Version> {
    // Strip leading 'v' and anything after a space (e.g. "(beta)")
    let cleaned = version_str
        .trim()
        .trim_start_matches('v')
        .split_whitespace()
        .next()?;
    semver::Version::parse(cleaned).ok()
}

// ── Public API ───────────────────────────────────────────────────────────

/// Check for updates in a background thread.
///
/// Updates `shared_state` as the check progresses.
/// `current_version` should be the APP_VERSION string (e.g. "v1.2.0 (beta)").
pub fn check_for_updates(current_version: &str, shared_state: SharedUpdateState) {
    let current = current_version.to_string();
    let state = shared_state.clone();

    std::thread::spawn(move || {
        // Set state to Checking
        if let Ok(mut s) = state.lock() {
            *s = UpdateState::Checking;
        }

        match do_check(&current) {
            Ok(Some(update_info)) => {
                info!("Update available: {} → {}", current, update_info.version);
                if let Ok(mut s) = state.lock() {
                    *s = UpdateState::Available(update_info);
                }
            }
            Ok(None) => {
                info!("Already on the latest version");
                if let Ok(mut s) = state.lock() {
                    *s = UpdateState::UpToDate;
                }
            }
            Err(e) => {
                warn!("Update check failed: {}", e);
                if let Ok(mut s) = state.lock() {
                    *s = UpdateState::Error(e.to_string());
                }
            }
        }
    });
}

/// Download the installer and launch it. Call from a background thread.
pub fn download_and_install(update_info: &UpdateInfo, shared_state: SharedUpdateState) {
    let info = update_info.clone();
    let state = shared_state.clone();

    std::thread::spawn(move || {
        if let Ok(mut s) = state.lock() {
            *s = UpdateState::Downloading(0.0);
        }

        match do_download(&info, &state) {
            Ok(path) => {
                info!("Installer downloaded to: {:?}", path);
                if let Ok(mut s) = state.lock() {
                    *s = UpdateState::ReadyToInstall(path);
                }
            }
            Err(e) => {
                error!("Download failed: {}", e);
                if let Ok(mut s) = state.lock() {
                    *s = UpdateState::Error(format!("Download failed: {}", e));
                }
            }
        }
    });
}

/// Launch the downloaded installer.
///
/// Does NOT use /SILENT because unsigned executables are blocked by
/// Windows SmartScreen in silent mode — the user must click
/// "More info → Run anyway".  The installer's `CloseApplications=force`
/// setting will close this running instance automatically, and the
/// `[Run]` post-install entry will re-launch the new version.
pub fn launch_installer(path: &std::path::Path, shared_state: SharedUpdateState) -> anyhow::Result<()> {
    info!("Launching installer: {:?}", path);

    // Detach the installer process so it is independent of us.
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

        std::process::Command::new(path)
            .args(["/UPDATE", "/CLOSEAPPLICATIONS", "/RESTARTAPPLICATIONS"])
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .spawn()
            .map_err(|e| {
                error!("Failed to spawn installer: {}", e);
                anyhow::anyhow!("Failed to spawn installer: {}", e)
            })?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new(path)
            .spawn()
            .map_err(|e| {
                error!("Failed to spawn installer: {}", e);
                anyhow::anyhow!("Failed to spawn installer: {}", e)
            })?;
    }

    // Transition to Installing state — the installer will close us via
    // CloseApplications=force and re-launch after install.
    if let Ok(mut s) = shared_state.lock() {
        *s = UpdateState::Installing;
    }

    Ok(())
}

// ── Internal helpers ─────────────────────────────────────────────────────

fn do_check(current_version: &str) -> anyhow::Result<Option<UpdateInfo>> {
    let current = parse_version(current_version)
        .ok_or_else(|| anyhow::anyhow!("Cannot parse current version: {}", current_version))?;

    let url = releases_url();
    let response = ureq::get(&url)
        .set("User-Agent", &format!("{}/{}", REPO_NAME, current_version))
        .set("Accept", "application/vnd.github.v3+json")
        .call();

    let response = match response {
        Ok(resp) => resp,
        Err(ureq::Error::Status(404, _)) => {
            // No releases published yet — not an error
            info!("No releases found (404) — treating as up-to-date");
            return Ok(None);
        }
        Err(ureq::Error::Status(403, _)) => {
            // Rate-limited or forbidden — silently skip
            info!("GitHub API returned 403 — skipping update check");
            return Ok(None);
        }
        Err(e) => return Err(e.into()),
    };

    let release: GitHubRelease = response.into_json()?;

    let latest = parse_version(&release.tag_name)
        .ok_or_else(|| anyhow::anyhow!("Cannot parse release version: {}", release.tag_name))?;

    if latest <= current {
        return Ok(None);
    }

    // Find the installer asset (look for .exe ending with _Setup_*.exe)
    let installer_asset = release
        .assets
        .iter()
        .find(|a| {
            a.name.ends_with(".exe")
                && (a.name.contains("Setup") || a.name.contains("setup") || a.name.contains("Installer"))
        })
        .ok_or_else(|| anyhow::anyhow!("No installer asset found in release {}", release.tag_name))?;

    Ok(Some(UpdateInfo {
        version: release.tag_name.clone(),
        title: release.name.unwrap_or_else(|| release.tag_name.clone()),
        body: release.body.unwrap_or_default(),
        download_url: installer_asset.browser_download_url.clone(),
        release_url: release.html_url,
    }))
}

fn do_download(info: &UpdateInfo, state: &SharedUpdateState) -> anyhow::Result<PathBuf> {
    let response = ureq::get(&info.download_url)
        .set("User-Agent", &format!("{}/updater", REPO_NAME))
        .call()?;

    let content_length: Option<usize> = response
        .header("Content-Length")
        .and_then(|v| v.parse().ok());

    // Download to temp directory
    let temp_dir = std::env::temp_dir();
    let filename = format!("ToolpathViewer_Setup_{}.exe", info.version.trim_start_matches('v'));
    let dest = temp_dir.join(&filename);

    let mut file = std::fs::File::create(&dest)?;
    let mut reader = response.into_reader();
    let mut downloaded: usize = 0;
    let mut buf = [0u8; 8192];

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        downloaded += n;

        // Update progress
        if let Some(total) = content_length {
            if total > 0 {
                let progress = (downloaded as f32) / (total as f32);
                if let Ok(mut s) = state.lock() {
                    *s = UpdateState::Downloading(progress.min(1.0));
                }
            }
        }
    }

    file.flush()?;
    Ok(dest)
}
