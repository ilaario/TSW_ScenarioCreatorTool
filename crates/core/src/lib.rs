use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use thiserror::Error;
use tsw_scenario_schema::{RouteProfile, ScenarioProject, TemplateDefinition};

#[derive(Debug, Clone)]
pub struct TemplateBundle {
    pub root_dir: PathBuf,
    pub definition: TemplateDefinition,
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("failed to read file '{path}': {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read directory '{path}': {source}")]
    ReadDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to enumerate directory '{path}': {source}")]
    ReadDirectoryEntry {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse YAML '{path}': {source}")]
    ParseYaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("route profile '{route_id}' was not found under '{profiles_root}'")]
    RouteProfileNotFound {
        route_id: String,
        profiles_root: PathBuf,
    },
    #[error("template directory '{template_id}' was not found under '{templates_root}'")]
    TemplateDirectoryNotFound {
        template_id: String,
        templates_root: PathBuf,
    },
    #[error("template definition for '{template_id}' was not found in '{template_dir}'")]
    TemplateDefinitionNotFound {
        template_id: String,
        template_dir: PathBuf,
    },
}

pub type Result<T> = std::result::Result<T, CoreError>;

pub fn load_scenario_from_path(path: impl AsRef<Path>) -> Result<ScenarioProject> {
    load_yaml_file(path)
}

pub fn load_route_profile_from_root(
    profiles_root: impl AsRef<Path>,
    route_id: &str,
) -> Result<RouteProfile> {
    let profiles_root = profiles_root.as_ref().to_path_buf();
    let profile_path = find_route_profile_path(&profiles_root, route_id).ok_or_else(|| {
        CoreError::RouteProfileNotFound {
            route_id: route_id.to_string(),
            profiles_root: profiles_root.clone(),
        }
    })?;

    load_yaml_file(profile_path)
}

pub fn load_all_route_profiles_from_root(profiles_root: impl AsRef<Path>) -> Result<Vec<RouteProfile>> {
    let profiles_root = profiles_root.as_ref().to_path_buf();
    if !profiles_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut profile_paths = Vec::new();
    for entry in fs::read_dir(&profiles_root).map_err(|source| CoreError::ReadDirectory {
        path: profiles_root.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| CoreError::ReadDirectoryEntry {
            path: profiles_root.clone(),
            source,
        })?;
        let entry_path = entry.path();
        if entry_path.is_file() && is_yaml_file(&entry_path) {
            profile_paths.push(entry_path);
        }
    }

    profile_paths.sort();

    let mut route_profiles = Vec::new();
    for profile_path in profile_paths {
        route_profiles.push(load_yaml_file(profile_path)?);
    }

    Ok(route_profiles)
}

pub fn load_template_bundle_from_root(
    templates_root: impl AsRef<Path>,
    template_id: &str,
) -> Result<TemplateBundle> {
    let templates_root = templates_root.as_ref().to_path_buf();
    let template_dir = find_template_dir(&templates_root, template_id).ok_or_else(|| {
        CoreError::TemplateDirectoryNotFound {
            template_id: template_id.to_string(),
            templates_root: templates_root.clone(),
        }
    })?;

    let definition_path = find_template_definition_path(&template_dir).ok_or_else(|| {
        CoreError::TemplateDefinitionNotFound {
            template_id: template_id.to_string(),
            template_dir: template_dir.clone(),
        }
    })?;

    let definition: TemplateDefinition = load_yaml_file(definition_path)?;

    Ok(TemplateBundle {
        root_dir: template_dir,
        definition,
    })
}

pub fn resolve_project_root(path: impl AsRef<Path>) -> PathBuf {
    resolve_relative_to(&current_dir_or_fallback(), path.as_ref())
}

pub fn resolve_relative_to(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

pub fn resolve_profiles_root(
    project_root: impl AsRef<Path>,
    profiles_root: impl AsRef<Path>,
    game_version: &str,
) -> PathBuf {
    resolve_relative_to(&resolve_project_root(project_root), profiles_root.as_ref())
        .join(game_version)
        .join("routes")
}

pub fn resolve_templates_root(
    project_root: impl AsRef<Path>,
    templates_root: impl AsRef<Path>,
    game_version: &str,
) -> PathBuf {
    resolve_relative_to(&resolve_project_root(project_root), templates_root.as_ref())
        .join(game_version)
}

fn current_dir_or_fallback() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn load_yaml_file<T>(path: impl AsRef<Path>) -> Result<T>
where
    T: DeserializeOwned,
{
    let path = path.as_ref().to_path_buf();
    let contents = fs::read_to_string(&path).map_err(|source| CoreError::ReadFile {
        path: path.clone(),
        source,
    })?;

    serde_yaml::from_str(&contents).map_err(|source| CoreError::ParseYaml { path, source })
}

fn find_route_profile_path(profiles_root: &Path, route_id: &str) -> Option<PathBuf> {
    for filename in id_variants(route_id) {
        for extension in ["yaml", "yml"] {
            let candidate = profiles_root.join(format!("{filename}.{extension}"));
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

fn find_template_dir(templates_root: &Path, template_id: &str) -> Option<PathBuf> {
    for directory_name in id_variants(template_id) {
        let candidate = templates_root.join(directory_name);
        if candidate.is_dir() {
            return Some(candidate);
        }
    }

    None
}

fn find_template_definition_path(template_dir: &Path) -> Option<PathBuf> {
    for filename in ["template.yaml", "template.yml"] {
        let candidate = template_dir.join(filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn id_variants(id: &str) -> Vec<String> {
    let mut variants = Vec::new();
    for candidate in [id.to_string(), id.to_ascii_lowercase(), id.to_ascii_uppercase()] {
        if !variants.contains(&candidate) {
            variants.push(candidate);
        }
    }

    variants
}

fn is_yaml_file(path: &Path) -> bool {
    matches!(path.extension().and_then(|extension| extension.to_str()), Some("yaml" | "yml"))
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
        let root = std::env::temp_dir().join(format!("tsw_scenario_tool_{name}_{unique}"));
        fs::create_dir_all(&root).expect("failed to create temp workspace");
        root
    }

    #[test]
    fn loads_route_profile_and_template_bundle_from_custom_roots() {
        let workspace = temp_workspace("core_loaders");
        let profiles_root = workspace.join("profiles").join("tsw5").join("routes");
        let templates_root = workspace.join("templates").join("tsw5").join("commuter_simple");
        fs::create_dir_all(&profiles_root).expect("failed to create profiles root");
        fs::create_dir_all(&templates_root).expect("failed to create templates root");

        fs::write(
            profiles_root.join("rro.yaml"),
            "id: RRO\nname: Ruhr-Sieg Nord\ntemplates:\n  - commuter_simple\n",
        )
        .expect("failed to write route profile");
        fs::write(
            templates_root.join("template.yaml"),
            "id: commuter_simple\nname: Commuter Simple\n",
        )
        .expect("failed to write template definition");

        let route_profile =
            load_route_profile_from_root(&profiles_root, "RRO").expect("route profile should load");
        let template_bundle = load_template_bundle_from_root(
            workspace.join("templates").join("tsw5"),
            "commuter_simple",
        )
        .expect("template bundle should load");

        assert_eq!(route_profile.id, "RRO");
        assert_eq!(template_bundle.definition.id, "commuter_simple");
        assert_eq!(template_bundle.definition.output_subdir(), "template");
    }

    #[test]
    fn loads_all_route_profiles_from_root() {
        let workspace = temp_workspace("all_profiles");
        let profiles_root = workspace.join("profiles").join("tsw5").join("routes");
        fs::create_dir_all(&profiles_root).expect("failed to create profiles root");

        fs::write(
            profiles_root.join("rro.yaml"),
            "id: RRO\nname: Ruhr-Sieg Nord\ninstall_ids:\n  - RuhrSiegNord\n",
        )
        .expect("failed to write route profile");
        fs::write(
            profiles_root.join("seh.yaml"),
            "id: SEH\nname: Southeastern Highspeed\ninstall_ids:\n  - SoutheasternHighSpeed\n",
        )
        .expect("failed to write route profile");

        let route_profiles =
            load_all_route_profiles_from_root(&profiles_root).expect("route profiles should load");

        assert_eq!(route_profiles.len(), 2);
        assert_eq!(route_profiles[0].id, "RRO");
        assert_eq!(route_profiles[1].id, "SEH");
    }

    #[test]
    fn resolves_profiles_and_templates_roots_relative_to_project_root() {
        let project_root = PathBuf::from("C:/workspace/tsw");

        let profiles_root = resolve_profiles_root(&project_root, Path::new("profiles"), "tsw5");
        let templates_root = resolve_templates_root(&project_root, Path::new("templates"), "tsw5");

        assert_eq!(profiles_root, project_root.join("profiles").join("tsw5").join("routes"));
        assert_eq!(templates_root, project_root.join("templates").join("tsw5"));
    }
}