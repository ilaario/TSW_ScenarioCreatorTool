use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallScan {
    pub schema_version: u32,
    pub game_dir: String,
    pub pak_files: Vec<DiscoveredPak>,
    pub summary: InstallScanSummary,
}

impl InstallScan {
    pub fn matches_any_canonical_id(&self, install_ids: &[String]) -> bool {
        install_ids.iter().any(|install_id| self.matches_canonical_id(install_id))
    }

    pub fn matches_canonical_id(&self, install_id: &str) -> bool {
        self.pak_files
            .iter()
            .any(|pak| pak.matches_canonical_id(install_id))
    }

    pub fn matches_any_hint(&self, hints: &[String]) -> bool {
        hints.iter().any(|hint| self.matches_hint(hint))
    }

    pub fn matches_hint(&self, hint: &str) -> bool {
        self.pak_files.iter().any(|pak| pak.matches_hint(hint))
    }

    pub fn discovered_canonical_dlcs(&self) -> Vec<CanonicalDlcCandidate> {
        let mut discovered = Vec::new();
        let mut seen = HashSet::new();

        for pak in &self.pak_files {
            let Some(canonical_dlc_id) = pak.resolved_canonical_dlc_id() else {
                continue;
            };
            let normalized_key = normalize_for_match(&canonical_dlc_id);
            if normalized_key.is_empty() || !seen.insert(normalized_key) {
                continue;
            }

            discovered.push(CanonicalDlcCandidate {
                canonical_dlc_id,
                file_name: pak.file_name.clone(),
                relative_path: pak.relative_path.clone(),
            });
        }

        discovered
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallScanSummary {
    pub pak_count: usize,
    pub scanned_root_count: usize,
    #[serde(default)]
    pub canonical_dlc_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CanonicalDlcCandidate {
    pub canonical_dlc_id: String,
    pub file_name: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredPak {
    pub file_name: String,
    pub stem: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_dlc_id: Option<String>,
    pub relative_path: String,
    pub full_path: String,
    pub search_text: String,
    pub normalized_search_text: String,
}

impl DiscoveredPak {
    pub fn matches_hint(&self, hint: &str) -> bool {
        let raw_hint = hint.trim().to_ascii_lowercase();
        let normalized_hint = normalize_for_match(hint);

        if raw_hint.is_empty() && normalized_hint.is_empty() {
            return false;
        }

        (!raw_hint.is_empty() && self.search_text.contains(&raw_hint))
            || (!normalized_hint.is_empty()
                && self.normalized_search_text.contains(&normalized_hint))
    }

    pub fn matches_canonical_id(&self, install_id: &str) -> bool {
        let normalized_install_id = normalize_for_match(install_id);
        if normalized_install_id.is_empty() {
            return false;
        }

        self.resolved_canonical_dlc_id()
            .map(|canonical_dlc_id| normalize_for_match(&canonical_dlc_id) == normalized_install_id)
            .unwrap_or(false)
    }

    pub fn resolved_canonical_dlc_id(&self) -> Option<String> {
        self.canonical_dlc_id
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .or_else(|| derive_canonical_dlc_id(&self.stem))
    }
}

#[derive(Debug, Error)]
pub enum ScannerError {
    #[error("game directory '{path}' does not exist or is not a directory")]
    InvalidGameDir { path: PathBuf },
}

pub fn scan_game_install(game_dir: impl AsRef<Path>) -> Result<InstallScan> {
    let game_dir = game_dir.as_ref();
    if !game_dir.is_dir() {
        return Err(ScannerError::InvalidGameDir {
            path: game_dir.to_path_buf(),
        })
        .context("invalid game directory");
    }

    let mut pak_files = Vec::new();
    collect_paks(game_dir, game_dir, &mut pak_files)?;
    pak_files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let canonical_dlc_count = {
        let mut unique_ids = HashSet::new();
        for pak in &pak_files {
            if let Some(canonical_dlc_id) = pak.resolved_canonical_dlc_id() {
                let normalized_key = normalize_for_match(&canonical_dlc_id);
                if !normalized_key.is_empty() {
                    unique_ids.insert(normalized_key);
                }
            }
        }
        unique_ids.len()
    };

    Ok(InstallScan {
        schema_version: 2,
        game_dir: game_dir.display().to_string(),
        summary: InstallScanSummary {
            pak_count: pak_files.len(),
            scanned_root_count: 1,
            canonical_dlc_count,
        },
        pak_files,
    })
}

pub fn save_install_scan(scan: &InstallScan, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let json = serde_json::to_string_pretty(scan).context("failed to serialize install scan")?;
    fs::write(path, json)
        .with_context(|| format!("failed to write install scan '{}'", path.display()))
}

pub fn load_install_scan(path: impl AsRef<Path>) -> Result<InstallScan> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read install scan '{}'", path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse install scan '{}'", path.display()))
}

fn collect_paks(root: &Path, current_dir: &Path, pak_files: &mut Vec<DiscoveredPak>) -> Result<()> {
    for entry in fs::read_dir(current_dir)
        .with_context(|| format!("failed to read directory '{}'", current_dir.display()))?
    {
        let entry = entry.with_context(|| {
            format!("failed to enumerate entries in '{}'", current_dir.display())
        })?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            collect_paks(root, &entry_path, pak_files)?;
            continue;
        }

        let Some(extension) = entry_path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !extension.eq_ignore_ascii_case("pak") {
            continue;
        }

        let relative_path = entry_path
            .strip_prefix(root)
            .unwrap_or(&entry_path)
            .to_string_lossy()
            .replace('\\', "/");
        let file_name = entry.file_name().to_string_lossy().to_string();
        let stem = entry_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_string();
        let canonical_dlc_id = derive_canonical_dlc_id(&stem);

        let mut search_segments = vec![relative_path.clone(), stem.clone()];
        if let Some(canonical_dlc_id) = &canonical_dlc_id {
            search_segments.push(canonical_dlc_id.clone());
        }
        let search_text = search_segments.join(" ").to_ascii_lowercase();

        pak_files.push(DiscoveredPak {
            file_name,
            stem,
            canonical_dlc_id,
            relative_path,
            full_path: entry_path.display().to_string(),
            search_text: search_text.clone(),
            normalized_search_text: normalize_for_match(&search_text),
        });
    }

    Ok(())
}

fn derive_canonical_dlc_id(stem: &str) -> Option<String> {
    let mut candidate = stem.trim();
    let mut recognized_pattern = false;

    for prefix in [
        "TS2Prototype-WindowsNoEditor-",
        "TS2Prototype-WindowsNoEditor_",
        "TS2Prototype-",
        "WindowsNoEditor-",
    ] {
        if let Some(stripped) = candidate.strip_prefix(prefix) {
            candidate = stripped;
            recognized_pattern = true;
            break;
        }
    }

    for suffix in ["-WindowsNoEditor", "_WindowsNoEditor", "-coredata", "_coredata"] {
        if let Some(stripped) = candidate.strip_suffix(suffix) {
            candidate = stripped;
            recognized_pattern = true;
            break;
        }
    }

    if !recognized_pattern {
        return None;
    }

    let candidate = candidate.trim_matches(|character: char| character == '-' || character == '_');
    if candidate.is_empty()
        || candidate.eq_ignore_ascii_case("TS2Prototype")
        || candidate.eq_ignore_ascii_case("WindowsNoEditor")
    {
        return None;
    }

    Some(candidate.to_string())
}

fn normalize_for_match(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_workspace(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("tsw_scenario_scanner_{name}_{unique}"));
        fs::create_dir_all(&root).expect("failed to create temp workspace");
        root
    }

    #[test]
    fn scans_pak_files_and_matches_hints() {
        let workspace = temp_workspace("scan");
        let pak_dir = workspace.join("TS2Prototype").join("Content").join("DLC");
        fs::create_dir_all(&pak_dir).expect("failed to create pak dir");
        fs::write(
            pak_dir.join("TS2Prototype-WindowsNoEditor-RuhrSiegNord.pak"),
            "",
        )
        .expect("failed to write pak");
        fs::write(pak_dir.join("DB_BR422-Pack.pak"), "").expect("failed to write pak");

        let scan = scan_game_install(&workspace).expect("scan should succeed");

        assert_eq!(scan.summary.pak_count, 2);
        assert_eq!(scan.summary.canonical_dlc_count, 1);
        assert!(scan.matches_canonical_id("RuhrSiegNord"));
        assert!(scan.matches_hint("Ruhr-Sieg Nord"));
        assert!(scan.matches_hint("DB_BR422"));
        assert!(scan.matches_any_hint(&["foo".to_string(), "Ruhr-Sieg Nord".to_string()]));
        assert_eq!(scan.discovered_canonical_dlcs().len(), 1);
    }

    #[test]
    fn ignores_base_game_pak_when_deriving_canonical_ids() {
        let workspace = temp_workspace("base_pak");
        let pak_dir = workspace.join("TS2Prototype").join("Content").join("Paks");
        fs::create_dir_all(&pak_dir).expect("failed to create pak dir");
        fs::write(pak_dir.join("TS2Prototype-WindowsNoEditor.pak"), "")
            .expect("failed to write pak");

        let scan = scan_game_install(&workspace).expect("scan should succeed");

        assert_eq!(scan.summary.pak_count, 1);
        assert_eq!(scan.summary.canonical_dlc_count, 0);
        assert!(scan.discovered_canonical_dlcs().is_empty());
    }

    #[test]
    fn matches_old_scan_files_without_explicit_canonical_id() {
        let scan: InstallScan = serde_json::from_str(
            r#"{
                "schema_version": 1,
                "game_dir": "C:/TSW",
                "pak_files": [
                    {
                        "file_name": "TS2Prototype-WindowsNoEditor-RuhrSiegNord.pak",
                        "stem": "TS2Prototype-WindowsNoEditor-RuhrSiegNord",
                        "relative_path": "WindowsNoEditor/TS2Prototype/Content/DLC/TS2Prototype-WindowsNoEditor-RuhrSiegNord.pak",
                        "full_path": "C:/TSW/WindowsNoEditor/TS2Prototype/Content/DLC/TS2Prototype-WindowsNoEditor-RuhrSiegNord.pak",
                        "search_text": "windowsnoeditor/ts2prototype/content/dlc/ts2prototype-windowsnoeditor-ruhrsiegnord.pak ts2prototype-windowsnoeditor-ruhrsiegnord",
                        "normalized_search_text": "windowsnoeditorts2prototypecontentdlcts2prototypewindowsnoeditorruhrsiegnordpakts2prototypewindowsnoeditorruhrsiegnord"
                    }
                ],
                "summary": {
                    "pak_count": 1,
                    "scanned_root_count": 1
                }
            }"#,
        )
        .expect("scan JSON should deserialize");

        assert!(scan.matches_canonical_id("RuhrSiegNord"));
        assert_eq!(scan.discovered_canonical_dlcs()[0].canonical_dlc_id, "RuhrSiegNord");
    }

    #[test]
    fn saves_and_loads_install_scan() {
        let workspace = temp_workspace("save_load");
        let pak_dir = workspace.join("Content");
        fs::create_dir_all(&pak_dir).expect("failed to create dir");
        fs::write(pak_dir.join("TS2Prototype-WindowsNoEditor-SampleRoute.pak"), "")
            .expect("failed to write pak");

        let scan = scan_game_install(&workspace).expect("scan should succeed");
        let output = workspace.join("install_scan.json");
        save_install_scan(&scan, &output).expect("save should succeed");
        let loaded = load_install_scan(&output).expect("load should succeed");

        assert_eq!(scan, loaded);
    }
}