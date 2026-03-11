use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use tsw_scenario_core::{resolve_project_root, resolve_relative_to, TemplateBundle};
use tsw_scenario_schema::{
    CompiledScenario, CompiledScenarioSummary, CompiledTemplateInfo, PackageEntry,
    PackageEntryKind, PackagePlan, ScenarioProject,
};

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub project_root: PathBuf,
    pub output_root: PathBuf,
    pub game_version: String,
    pub clean: bool,
    pub package_namespace: String,
}

#[derive(Debug, Clone)]
pub struct BuildOutput {
    pub build_dir: PathBuf,
    pub template_output_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub scenario_path: PathBuf,
    pub compiled_scenario_path: PathBuf,
    pub staging_dir: PathBuf,
    pub package_plan_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct BuildManifest {
    schema_version: u32,
    scenario_id: String,
    title: String,
    author: String,
    game_version: String,
    route: String,
    template: String,
    template_name: String,
    template_output_subdir: String,
    consist: String,
    ai_service_count: usize,
    objective_count: usize,
    success_condition_count: usize,
    failure_condition_count: usize,
    package_namespace: String,
    package_entry_count: usize,
}

pub fn compile_project(
    project: &ScenarioProject,
    template_bundle: &TemplateBundle,
    options: &BuildOptions,
) -> Result<BuildOutput> {
    let project_root = resolve_project_root(&options.project_root);
    let output_root = resolve_relative_to(&project_root, &options.output_root);
    let build_dir = output_root.join(&project.meta.id);

    if options.clean && build_dir.exists() {
        fs::remove_dir_all(&build_dir).with_context(|| {
            format!("failed to clean build directory '{}'", build_dir.display())
        })?;
    }

    fs::create_dir_all(&build_dir).with_context(|| {
        format!("failed to create build directory '{}'", build_dir.display())
    })?;

    let template_output_dir = build_dir.join(template_bundle.definition.output_subdir());
    copy_template_directory(project, template_bundle, &template_output_dir)?;

    let compiled_scenario = build_compiled_scenario(project, template_bundle);
    let manifest_path = build_dir.join("manifest.json");
    let scenario_path = build_dir.join("scenario.yaml");
    let compiled_scenario_path = build_dir.join("compiled_scenario.json");

    let scenario_yaml =
        serde_yaml::to_string(project).context("failed to serialize scenario project")?;
    fs::write(&scenario_path, scenario_yaml)
        .with_context(|| format!("failed to write scenario '{}'", scenario_path.display()))?;
    write_json_file(&compiled_scenario_path, &compiled_scenario)?;

    let (staging_dir, package_plan) = build_package_staging(
        &build_dir,
        &compiled_scenario_path,
        &template_output_dir,
        options,
        &project.meta.id,
    )?;
    let package_plan_path = build_dir.join("package_plan.json");
    write_json_file(&package_plan_path, &package_plan)?;

    let manifest = build_manifest(
        project,
        template_bundle,
        options,
        &compiled_scenario,
        package_plan.entries.len(),
    );
    write_json_file(&manifest_path, &manifest)?;

    Ok(BuildOutput {
        build_dir,
        template_output_dir,
        manifest_path,
        scenario_path,
        compiled_scenario_path,
        staging_dir,
        package_plan_path,
    })
}

fn build_manifest(
    project: &ScenarioProject,
    template_bundle: &TemplateBundle,
    options: &BuildOptions,
    compiled_scenario: &CompiledScenario,
    package_entry_count: usize,
) -> BuildManifest {
    let summary = &compiled_scenario.summary;
    BuildManifest {
        schema_version: 1,
        scenario_id: project.meta.id.clone(),
        title: project.meta.title.clone(),
        author: project.meta.author.clone(),
        game_version: options.game_version.clone(),
        route: project.scenario.route.clone(),
        template: project.scenario.template.clone(),
        template_name: template_bundle.definition.name.clone(),
        template_output_subdir: template_bundle.definition.output_subdir().to_string(),
        consist: project.player_service.consist.clone(),
        ai_service_count: summary.ai_service_count,
        objective_count: summary.objective_count,
        success_condition_count: summary.success_condition_count,
        failure_condition_count: summary.failure_condition_count,
        package_namespace: normalized_package_namespace(&options.package_namespace),
        package_entry_count,
    }
}

fn build_compiled_scenario(
    project: &ScenarioProject,
    template_bundle: &TemplateBundle,
) -> CompiledScenario {
    CompiledScenario {
        schema_version: 1,
        meta: project.meta.clone(),
        scenario: project.scenario.clone(),
        player_service: project.player_service.clone(),
        ai_services: project.ai_services.clone(),
        objectives: project.objectives.clone(),
        completion: project.completion.clone(),
        template: CompiledTemplateInfo {
            id: template_bundle.definition.id.clone(),
            name: template_bundle.definition.name.clone(),
            output_subdir: template_bundle.definition.output_subdir().to_string(),
        },
        summary: CompiledScenarioSummary {
            ai_service_count: project.ai_services.len(),
            objective_count: project.objectives.len(),
            success_condition_count: project.completion.success.len(),
            failure_condition_count: project.completion.failure.len(),
        },
    }
}

fn build_package_staging(
    build_dir: &Path,
    compiled_scenario_path: &Path,
    template_output_dir: &Path,
    options: &BuildOptions,
    scenario_id: &str,
) -> Result<(PathBuf, PackagePlan)> {
    let namespace = normalized_package_namespace(&options.package_namespace);
    let namespace_path = namespace_path(&namespace);
    let staging_dir = build_dir.join("staging");
    let staging_root = staging_dir
        .join("Content")
        .join(&namespace_path)
        .join(scenario_id);

    fs::create_dir_all(&staging_root).with_context(|| {
        format!("failed to create staging root '{}'", staging_root.display())
    })?;

    let mount_root = format!("/Content/{namespace}/{scenario_id}");
    let mut entries = Vec::new();

    stage_single_file(
        build_dir,
        compiled_scenario_path,
        &staging_root.join("compiled_scenario.json"),
        &format!("{mount_root}/compiled_scenario.json"),
        PackageEntryKind::CompiledScenario,
        &mut entries,
    )?;

    let template_dir_name = template_output_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("template");
    stage_directory_recursive(
        build_dir,
        template_output_dir,
        template_output_dir,
        &staging_root.join(template_dir_name),
        &format!("{mount_root}/{}", normalize_path_component(template_dir_name)),
        PackageEntryKind::TemplateFile,
        &mut entries,
    )?;

    let plan = PackagePlan {
        schema_version: 1,
        scenario_id: scenario_id.to_string(),
        game_version: options.game_version.clone(),
        package_namespace: namespace.clone(),
        suggested_pak_name: format!("{scenario_id}.pak"),
        staging_root: normalize_path(&staging_root.strip_prefix(build_dir).unwrap_or(&staging_root)),
        mount_root,
        entries,
    };

    Ok((staging_dir, plan))
}

fn stage_single_file(
    build_dir: &Path,
    source_path: &Path,
    staged_path: &Path,
    mount_path: &str,
    kind: PackageEntryKind,
    entries: &mut Vec<PackageEntry>,
) -> Result<()> {
    if let Some(parent) = staged_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create staging directory '{}'", parent.display())
        })?;
    }

    fs::copy(source_path, staged_path).with_context(|| {
        format!(
            "failed to stage file '{}' to '{}'",
            source_path.display(),
            staged_path.display()
        )
    })?;

    entries.push(PackageEntry {
        kind,
        source_path: normalize_path(&source_path.strip_prefix(build_dir).unwrap_or(source_path)),
        staged_path: normalize_path(&staged_path.strip_prefix(build_dir).unwrap_or(staged_path)),
        mount_path: mount_path.to_string(),
    });

    Ok(())
}

fn stage_directory_recursive(
    build_dir: &Path,
    source_root: &Path,
    source_dir: &Path,
    staging_dir: &Path,
    mount_root: &str,
    kind: PackageEntryKind,
    entries: &mut Vec<PackageEntry>,
) -> Result<()> {
    fs::create_dir_all(staging_dir).with_context(|| {
        format!("failed to create staging directory '{}'", staging_dir.display())
    })?;

    for entry in fs::read_dir(source_dir)
        .with_context(|| format!("failed to read source directory '{}'", source_dir.display()))?
    {
        let entry = entry.with_context(|| {
            format!("failed to enumerate entries in '{}'", source_dir.display())
        })?;
        let entry_path = entry.path();
        let staged_path = staging_dir.join(entry.file_name());

        if entry_path.is_dir() {
            stage_directory_recursive(
                build_dir,
                source_root,
                &entry_path,
                &staged_path,
                mount_root,
                kind.clone(),
                entries,
            )?;
            continue;
        }

        fs::copy(&entry_path, &staged_path).with_context(|| {
            format!(
                "failed to stage file '{}' to '{}'",
                entry_path.display(),
                staged_path.display()
            )
        })?;

        let relative_mount = entry_path.strip_prefix(source_root).unwrap_or(&entry_path);
        entries.push(PackageEntry {
            kind: kind.clone(),
            source_path: normalize_path(&entry_path.strip_prefix(build_dir).unwrap_or(&entry_path)),
            staged_path: normalize_path(&staged_path.strip_prefix(build_dir).unwrap_or(&staged_path)),
            mount_path: format!("{mount_root}/{}", normalize_path(relative_mount)),
        });
    }

    Ok(())
}

fn copy_template_directory(
    project: &ScenarioProject,
    template_bundle: &TemplateBundle,
    destination_root: &Path,
) -> Result<()> {
    let render_extensions = resolved_render_extensions(&template_bundle.definition.render_extensions);
    copy_directory_recursive(
        &template_bundle.root_dir,
        destination_root,
        project,
        template_bundle,
        &render_extensions,
    )
}

fn copy_directory_recursive(
    source_dir: &Path,
    destination_dir: &Path,
    project: &ScenarioProject,
    template_bundle: &TemplateBundle,
    render_extensions: &HashSet<String>,
) -> Result<()> {
    fs::create_dir_all(destination_dir).with_context(|| {
        format!(
            "failed to create template output directory '{}'",
            destination_dir.display()
        )
    })?;

    for entry in fs::read_dir(source_dir)
        .with_context(|| format!("failed to read template directory '{}'", source_dir.display()))?
    {
        let entry = entry.with_context(|| {
            format!("failed to enumerate entries in '{}'", source_dir.display())
        })?;
        let entry_path = entry.path();
        let destination_path = destination_dir.join(entry.file_name());

        if entry_path.is_dir() {
            copy_directory_recursive(
                &entry_path,
                &destination_path,
                project,
                template_bundle,
                render_extensions,
            )?;
            continue;
        }

        if is_template_metadata_file(&entry_path) {
            continue;
        }

        if should_render_file(&entry_path, render_extensions) {
            let input = fs::read_to_string(&entry_path).with_context(|| {
                format!("failed to read template text file '{}'", entry_path.display())
            })?;
            let rendered = render_tokens(&input, project, template_bundle);
            fs::write(&destination_path, rendered).with_context(|| {
                format!("failed to write rendered template file '{}'", destination_path.display())
            })?;
        } else {
            fs::copy(&entry_path, &destination_path).with_context(|| {
                format!(
                    "failed to copy template file '{}' to '{}'",
                    entry_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }

    Ok(())
}

fn write_json_file<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    let json = serde_json::to_string_pretty(value)
        .with_context(|| format!("failed to serialize JSON for '{}'", path.display()))?;
    fs::write(path, json)
        .with_context(|| format!("failed to write JSON file '{}'", path.display()))
}

fn resolved_render_extensions(configured_extensions: &[String]) -> HashSet<String> {
    if configured_extensions.is_empty() {
        ["txt", "json", "yaml", "yml", "toml", "ini", "cfg", "md"]
            .into_iter()
            .map(str::to_string)
            .collect()
    } else {
        configured_extensions
            .iter()
            .map(|extension| extension.trim_start_matches('.').to_ascii_lowercase())
            .collect()
    }
}

fn should_render_file(path: &Path, render_extensions: &HashSet<String>) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| render_extensions.contains(&extension.to_ascii_lowercase()))
        .unwrap_or(false)
}

fn is_template_metadata_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("template.yaml" | "template.yml")
    )
}

fn render_tokens(input: &str, project: &ScenarioProject, template_bundle: &TemplateBundle) -> String {
    let replacements = vec![
        ("{{meta.id}}".to_string(), project.meta.id.clone()),
        ("{{meta.title}}".to_string(), project.meta.title.clone()),
        ("{{meta.author}}".to_string(), project.meta.author.clone()),
        ("{{scenario.route}}".to_string(), project.scenario.route.clone()),
        ("{{scenario.template}}".to_string(), project.scenario.template.clone()),
        ("{{scenario.start_time}}".to_string(), project.scenario.start_time.clone()),
        ("{{scenario.weather}}".to_string(), project.scenario.weather.clone()),
        (
            "{{player_service.consist}}".to_string(),
            project.player_service.consist.clone(),
        ),
        (
            "{{player_service.start_location}}".to_string(),
            project.player_service.start_location.clone(),
        ),
        (
            "{{player_service.destination}}".to_string(),
            project.player_service.destination.clone(),
        ),
        ("{{template.name}}".to_string(), template_bundle.definition.name.clone()),
        (
            "{{counts.ai_services}}".to_string(),
            project.ai_services.len().to_string(),
        ),
        (
            "{{counts.objectives}}".to_string(),
            project.objectives.len().to_string(),
        ),
        (
            "{{counts.success_conditions}}".to_string(),
            project.completion.success.len().to_string(),
        ),
        (
            "{{counts.failure_conditions}}".to_string(),
            project.completion.failure.len().to_string(),
        ),
    ];

    let mut rendered = input.to_string();
    for (token, value) in replacements {
        rendered = rendered.replace(&token, &value);
    }
    rendered
}

fn normalized_package_namespace(namespace: &str) -> String {
    let normalized = namespace.replace('\\', "/");
    let trimmed = normalized.trim_matches('/');
    if trimmed.is_empty() {
        "ScenarioMods".to_string()
    } else {
        trimmed.to_string()
    }
}

fn namespace_path(namespace: &str) -> PathBuf {
    let mut path = PathBuf::new();
    for segment in namespace.split('/') {
        if !segment.is_empty() {
            path.push(segment);
        }
    }
    path
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn normalize_path_component(component: &str) -> String {
    component.replace('\\', "/").trim_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tsw_scenario_core::TemplateBundle;
    use tsw_scenario_schema::{
        AiService, CompletionRules, Condition, ConditionKind, Meta, Objective, ObjectiveKind,
        PlayerService, Scenario, ScenarioProject, TemplateDefinition,
    };

    fn temp_workspace(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("tsw_scenario_tool_{name}_{unique}"));
        fs::create_dir_all(&root).expect("failed to create temp workspace");
        root
    }

    fn sample_project() -> ScenarioProject {
        ScenarioProject {
            meta: Meta {
                id: "rro_test_001".to_string(),
                title: "Morning Run".to_string(),
                author: "Dario".to_string(),
            },
            scenario: Scenario {
                route: "RRO".to_string(),
                template: "commuter_simple".to_string(),
                start_time: "08:15".to_string(),
                weather: "cloudy".to_string(),
            },
            player_service: PlayerService {
                consist: "DB_BR422".to_string(),
                start_location: "Essen_Hbf_P5".to_string(),
                destination: "Bochum_Hbf_P3".to_string(),
            },
            ai_services: vec![AiService {
                id: "ai_regional_01".to_string(),
                consist: "DB_BR422".to_string(),
                start_location: "Bochum_Hbf_P3".to_string(),
                destination: "Essen_Hbf_P5".to_string(),
                departure_time: Some("08:05".to_string()),
            }],
            objectives: vec![Objective {
                id: "stop_bochum".to_string(),
                description: "Reach Bochum Hbf".to_string(),
                kind: ObjectiveKind::StopAt,
                location: Some("Bochum_Hbf_P3".to_string()),
                time: None,
            }],
            completion: CompletionRules {
                success: vec![Condition {
                    kind: ConditionKind::AllObjectivesCompleted,
                    objective_id: None,
                    service_id: None,
                    time: None,
                    description: None,
                }],
                failure: vec![Condition {
                    kind: ConditionKind::TimeReached,
                    objective_id: None,
                    service_id: None,
                    time: Some("09:00".to_string()),
                    description: None,
                }],
            },
        }
    }

    #[test]
    fn copies_template_files_writes_compiled_json_and_builds_package_plan() {
        let workspace = temp_workspace("compiler");
        let template_root = workspace.join("templates").join("commuter_simple");
        fs::create_dir_all(template_root.join("content")).expect("failed to create template root");
        fs::write(
            template_root.join("template.yaml"),
            "id: commuter_simple\nname: Commuter Simple\noutput_subdir: generated\nrender_extensions:\n  - yaml\n",
        )
        .expect("failed to write template definition");
        fs::write(
            template_root.join("content").join("scenario_stub.yaml"),
            "id: {{meta.id}}\nroute: {{scenario.route}}\nai_services: {{counts.ai_services}}\n",
        )
        .expect("failed to write template file");

        let build_output = compile_project(
            &sample_project(),
            &TemplateBundle {
                root_dir: template_root,
                definition: TemplateDefinition {
                    id: "commuter_simple".to_string(),
                    name: "Commuter Simple".to_string(),
                    description: None,
                    output_subdir: Some("generated".to_string()),
                    render_extensions: vec!["yaml".to_string()],
                },
            },
            &BuildOptions {
                project_root: workspace.clone(),
                output_root: PathBuf::from("build"),
                game_version: "tsw5".to_string(),
                clean: true,
                package_namespace: "ScenarioMods/Test".to_string(),
            },
        )
        .expect("build should succeed");

        let rendered_file = build_output
            .template_output_dir
            .join("content")
            .join("scenario_stub.yaml");
        let rendered = fs::read_to_string(rendered_file).expect("failed to read rendered file");
        let compiled_json = fs::read_to_string(&build_output.compiled_scenario_path)
            .expect("failed to read compiled scenario json");
        let package_plan = fs::read_to_string(&build_output.package_plan_path)
            .expect("failed to read package plan");
        let staged_template = build_output
            .staging_dir
            .join("Content")
            .join("ScenarioMods")
            .join("Test")
            .join("rro_test_001")
            .join("generated")
            .join("content")
            .join("scenario_stub.yaml");
        let staged_compiled = build_output
            .staging_dir
            .join("Content")
            .join("ScenarioMods")
            .join("Test")
            .join("rro_test_001")
            .join("compiled_scenario.json");

        assert!(rendered.contains("id: rro_test_001"));
        assert!(rendered.contains("route: RRO"));
        assert!(rendered.contains("ai_services: 1"));
        assert!(compiled_json.contains("\"objective_count\": 1"));
        assert!(package_plan.contains("\"package_namespace\": \"ScenarioMods/Test\""));
        assert!(package_plan.contains("/Content/ScenarioMods/Test/rro_test_001/compiled_scenario.json"));
        assert!(staged_template.exists());
        assert!(staged_compiled.exists());
        assert!(build_output.manifest_path.exists());
        assert!(build_output.scenario_path.exists());
    }
}