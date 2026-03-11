use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;
use tsw_scenario_compiler::{compile_project, BuildOptions};
use tsw_scenario_core::{
    load_all_route_profiles_from_root, load_route_profile_from_root, load_scenario_from_path,
    load_template_bundle_from_root, resolve_profiles_root, resolve_project_root,
    resolve_relative_to, resolve_templates_root, TemplateBundle,
};
use tsw_scenario_scanner::{
    load_install_scan, save_install_scan, scan_game_install, CanonicalDlcCandidate, InstallScan,
};
use tsw_scenario_schema::{RouteProfile, ScenarioProject};
use tsw_scenario_validator::{validate_project, ValidationIssue, ValidationReport};

#[derive(Debug, Parser)]
#[command(
    name = "tswtool",
    version,
    about = "Validate, scan, and build Train Sim World scenario projects"
)]
struct Cli {
    #[arg(long, global = true, default_value = "tsw5")]
    game_version: String,
    #[arg(long, global = true, default_value = ".")]
    project_root: PathBuf,
    #[arg(long, global = true, default_value = "profiles")]
    profiles_root: PathBuf,
    #[arg(long, global = true, default_value = "templates")]
    templates_root: PathBuf,
    #[arg(long, global = true)]
    game_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    install_scan: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Scan {
        #[arg(long, default_value = "install_scan.json")]
        output: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Validate {
        scenario: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Build {
        scenario: PathBuf,
        #[arg(long, default_value = "build")]
        output: PathBuf,
        #[arg(long)]
        clean: bool,
        #[arg(long, default_value = "ScenarioMods")]
        package_namespace: String,
    },
}

#[derive(Debug)]
struct LoadedProjectContext {
    project: ScenarioProject,
    route_profile: RouteProfile,
    template_bundle: TemplateBundle,
    report: ValidationReport,
    install_scan: Option<InstallScan>,
}

#[derive(Debug, Clone, Serialize)]
struct ScanReport {
    scan: InstallScan,
    summary: ScanReportSummary,
    matched_routes: Vec<DetectedRouteMatch>,
    unknown_canonical_dlcs: Vec<UnknownCanonicalDlc>,
}

#[derive(Debug, Clone, Serialize)]
struct ScanReportSummary {
    pak_count: usize,
    canonical_dlc_count: usize,
    matched_route_count: usize,
    unknown_canonical_dlc_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct DetectedRouteMatch {
    canonical_dlc_id: String,
    route_id: String,
    route_name: String,
    matched_by: String,
    support_level: String,
    pak_file: String,
}

#[derive(Debug, Clone, Serialize)]
struct UnknownCanonicalDlc {
    canonical_dlc_id: String,
    pak_file: String,
}

fn main() -> ExitCode {
    match try_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn try_main() -> Result<()> {
    let cli = Cli::parse();
    let project_root = resolve_project_root(&cli.project_root);

    match cli.command {
        Commands::Scan { output, json } => run_scan(
            &project_root,
            &cli.game_version,
            &cli.profiles_root,
            cli.game_dir.as_deref(),
            output,
            json,
        ),
        Commands::Validate { scenario, json } => run_validate(
            &project_root,
            &cli.game_version,
            &cli.profiles_root,
            &cli.templates_root,
            cli.game_dir.as_deref(),
            cli.install_scan.as_deref(),
            &scenario,
            json,
        ),
        Commands::Build {
            scenario,
            output,
            clean,
            package_namespace,
        } => run_build(
            &project_root,
            &cli.game_version,
            &cli.profiles_root,
            &cli.templates_root,
            cli.game_dir.as_deref(),
            cli.install_scan.as_deref(),
            &scenario,
            output,
            clean,
            package_namespace,
        ),
    }
}

fn run_scan(
    project_root: &Path,
    game_version: &str,
    profiles_root: &Path,
    game_dir: Option<&Path>,
    output: PathBuf,
    json: bool,
) -> Result<()> {
    let game_dir = game_dir.context("--game-dir is required for the scan command")?;
    let resolved_game_dir = resolve_relative_to(project_root, game_dir);
    let scan = scan_game_install(&resolved_game_dir)
        .with_context(|| format!("failed to scan game directory '{}'", resolved_game_dir.display()))?;

    let output_path = resolve_relative_to(project_root, &output);
    save_install_scan(&scan, &output_path)
        .with_context(|| format!("failed to save install scan to '{}'", output_path.display()))?;

    let resolved_profiles_root = resolve_profiles_root(project_root, profiles_root, game_version);
    let route_profiles = load_all_route_profiles_from_root(&resolved_profiles_root)
        .with_context(|| format!("failed to load route profiles from '{}'", resolved_profiles_root.display()))?;
    let report = build_scan_report(&scan, &route_profiles);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .context("failed to serialize scan report as JSON")?
        );
    } else {
        println!("Scan completed for game directory '{}'.", resolved_game_dir.display());
        println!("Detected {} .pak file(s).", report.summary.pak_count);
        println!(
            "Detected {} canonical DLC id(s).",
            report.summary.canonical_dlc_count
        );

        if report.matched_routes.is_empty() {
            println!("Matched routes: none");
        } else {
            println!("Matched routes:");
            for matched_route in &report.matched_routes {
                println!(
                    "  - {} -> {} ({}) via {} [{}]",
                    matched_route.canonical_dlc_id,
                    matched_route.route_id,
                    matched_route.route_name,
                    matched_route.matched_by,
                    matched_route.support_level
                );
            }
        }

        if report.unknown_canonical_dlcs.is_empty() {
            println!("Unknown canonical DLC ids: none");
        } else {
            println!("Unknown canonical DLC ids:");
            for unknown_dlc in &report.unknown_canonical_dlcs {
                println!("  - {} ({})", unknown_dlc.canonical_dlc_id, unknown_dlc.pak_file);
            }
        }

        println!("Install scan cache: {}", output_path.display());
    }

    Ok(())
}

fn run_validate(
    project_root: &Path,
    game_version: &str,
    profiles_root: &Path,
    templates_root: &Path,
    game_dir: Option<&Path>,
    install_scan_path: Option<&Path>,
    scenario: &Path,
    json: bool,
) -> Result<()> {
    let context = load_and_validate(
        project_root,
        game_version,
        profiles_root,
        templates_root,
        game_dir,
        install_scan_path,
        scenario,
    )?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&context.report)
                .context("failed to serialize report as JSON")?
        );
    } else {
        print_validation_report(&context.report);
        if context.report.is_valid() {
            println!(
                "Validation succeeded for scenario '{}' on route '{}'.",
                context.project.meta.id, context.route_profile.id
            );
            if let Some(scan) = &context.install_scan {
                println!("Install scan in use: {} .pak file(s) detected.", scan.summary.pak_count);
            }
        }
    }

    if context.report.has_errors() {
        bail!("validation failed");
    }

    Ok(())
}

fn run_build(
    project_root: &Path,
    game_version: &str,
    profiles_root: &Path,
    templates_root: &Path,
    game_dir: Option<&Path>,
    install_scan_path: Option<&Path>,
    scenario: &Path,
    output_root: PathBuf,
    clean: bool,
    package_namespace: String,
) -> Result<()> {
    let context = load_and_validate(
        project_root,
        game_version,
        profiles_root,
        templates_root,
        game_dir,
        install_scan_path,
        scenario,
    )?;
    print_validation_report(&context.report);

    if context.report.has_errors() {
        bail!("build aborted because validation failed");
    }

    let build_output = compile_project(
        &context.project,
        &context.template_bundle,
        &BuildOptions {
            project_root: project_root.to_path_buf(),
            output_root,
            game_version: game_version.to_string(),
            clean,
            package_namespace,
        },
    )?;

    println!("Build completed for scenario '{}'.", context.project.meta.id);
    println!("Build directory: {}", build_output.build_dir.display());
    println!("Template output: {}", build_output.template_output_dir.display());
    println!("Manifest: {}", build_output.manifest_path.display());
    println!("Scenario copy: {}", build_output.scenario_path.display());
    println!("Compiled scenario: {}", build_output.compiled_scenario_path.display());
    println!("Staging directory: {}", build_output.staging_dir.display());
    println!("Package plan: {}", build_output.package_plan_path.display());

    Ok(())
}

fn load_and_validate(
    project_root: &Path,
    game_version: &str,
    profiles_root: &Path,
    templates_root: &Path,
    game_dir: Option<&Path>,
    install_scan_path: Option<&Path>,
    scenario: &Path,
) -> Result<LoadedProjectContext> {
    let scenario_path = resolve_relative_to(project_root, scenario);
    let project = load_scenario_from_path(&scenario_path)
        .with_context(|| format!("failed to load scenario file '{}'", scenario_path.display()))?;

    let resolved_profiles_root = resolve_profiles_root(project_root, profiles_root, game_version);
    let route_profile = load_route_profile_from_root(&resolved_profiles_root, &project.scenario.route)
        .with_context(|| {
            format!(
                "failed to load route profile '{}' from '{}'",
                project.scenario.route,
                resolved_profiles_root.display()
            )
        })?;

    let resolved_templates_root = resolve_templates_root(project_root, templates_root, game_version);
    let template_bundle =
        load_template_bundle_from_root(&resolved_templates_root, &project.scenario.template)
            .with_context(|| {
                format!(
                    "failed to load template '{}' from '{}'",
                    project.scenario.template,
                    resolved_templates_root.display()
                )
            })?;

    let mut report = validate_project(&project, &route_profile, &template_bundle.definition);
    let install_scan = resolve_install_scan(project_root, game_dir, install_scan_path)?;
    if let Some(scan) = &install_scan {
        integrate_install_scan(&mut report, &project, &route_profile, scan);
    }

    Ok(LoadedProjectContext {
        project,
        route_profile,
        template_bundle,
        report,
        install_scan,
    })
}

fn resolve_install_scan(
    project_root: &Path,
    game_dir: Option<&Path>,
    install_scan_path: Option<&Path>,
) -> Result<Option<InstallScan>> {
    if let Some(game_dir) = game_dir {
        let resolved_game_dir = resolve_relative_to(project_root, game_dir);
        let scan = scan_game_install(&resolved_game_dir).with_context(|| {
            format!("failed to scan game directory '{}'", resolved_game_dir.display())
        })?;
        return Ok(Some(scan));
    }

    if let Some(install_scan_path) = install_scan_path {
        let resolved_scan_path = resolve_relative_to(project_root, install_scan_path);
        let scan = load_install_scan(&resolved_scan_path).with_context(|| {
            format!("failed to load install scan '{}'", resolved_scan_path.display())
        })?;
        return Ok(Some(scan));
    }

    let default_scan_path = project_root.join("install_scan.json");
    if default_scan_path.exists() {
        let scan = load_install_scan(&default_scan_path).with_context(|| {
            format!("failed to load install scan '{}'", default_scan_path.display())
        })?;
        return Ok(Some(scan));
    }

    Ok(None)
}

fn integrate_install_scan(
    report: &mut ValidationReport,
    project: &ScenarioProject,
    route_profile: &RouteProfile,
    install_scan: &InstallScan,
) {
    let canonical_install_ids = route_profile.canonical_install_ids();
    let install_aliases = route_profile.install_aliases();

    let has_canonical_match = !canonical_install_ids.is_empty()
        && install_scan.matches_any_canonical_id(&canonical_install_ids);
    let has_alias_match = !install_aliases.is_empty() && install_scan.matches_any_hint(&install_aliases);

    if has_canonical_match || has_alias_match {
        return;
    }

    let identifiers = route_profile.install_identifiers();
    report.errors.push(ValidationIssue {
        code: "ROUTE_NOT_INSTALLED".to_string(),
        field: "scenario.route".to_string(),
        message: format!(
            "route '{}' could not be matched in the scanned installation using identifiers [{}]",
            project.scenario.route,
            identifiers.join(", ")
        ),
    });
}

fn build_scan_report(scan: &InstallScan, route_profiles: &[RouteProfile]) -> ScanReport {
    let mut matched_routes = Vec::new();
    let mut unknown_canonical_dlcs = Vec::new();

    for candidate in scan.discovered_canonical_dlcs() {
        if let Some((route_profile, matched_by)) = find_route_match(&candidate, route_profiles) {
            matched_routes.push(DetectedRouteMatch {
                canonical_dlc_id: candidate.canonical_dlc_id,
                route_id: route_profile.id.clone(),
                route_name: route_profile.name.clone(),
                matched_by: matched_by.to_string(),
                support_level: route_support_level(route_profile).to_string(),
                pak_file: candidate.file_name,
            });
        } else {
            unknown_canonical_dlcs.push(UnknownCanonicalDlc {
                canonical_dlc_id: candidate.canonical_dlc_id,
                pak_file: candidate.file_name,
            });
        }
    }

    ScanReport {
        summary: ScanReportSummary {
            pak_count: scan.summary.pak_count,
            canonical_dlc_count: scan.summary.canonical_dlc_count,
            matched_route_count: matched_routes.len(),
            unknown_canonical_dlc_count: unknown_canonical_dlcs.len(),
        },
        scan: scan.clone(),
        matched_routes,
        unknown_canonical_dlcs,
    }
}

fn find_route_match<'a>(
    candidate: &CanonicalDlcCandidate,
    route_profiles: &'a [RouteProfile],
) -> Option<(&'a RouteProfile, &'static str)> {
    for route_profile in route_profiles {
        let canonical_install_ids = route_profile.canonical_install_ids();
        if !canonical_install_ids.is_empty()
            && canonical_install_ids.iter().any(|install_id| {
                normalized_identifiers_match(install_id, &candidate.canonical_dlc_id)
            })
        {
            return Some((route_profile, "install_id"));
        }
    }

    for route_profile in route_profiles {
        let install_aliases = route_profile.install_aliases();
        if install_aliases.iter().any(|alias| {
            normalized_identifiers_match(alias, &candidate.canonical_dlc_id)
        }) {
            return Some((route_profile, "alias"));
        }
    }

    None
}

fn normalized_identifiers_match(left: &str, right: &str) -> bool {
    let left = normalize_identifier(left);
    let right = normalize_identifier(right);
    !left.is_empty() && left == right
}

fn route_support_level(route_profile: &RouteProfile) -> &'static str {
    if route_profile.supported_stock.is_empty()
        || route_profile.spawn_points.is_empty()
        || route_profile.templates.is_empty()
    {
        "discovery_only"
    } else {
        "curated"
    }
}

fn normalize_identifier(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn print_validation_report(report: &ValidationReport) {
    println!(
        "Validation report: {} error(s), {} warning(s)",
        report.error_count(),
        report.warning_count()
    );
    print_issues("Errors", &report.errors);
    print_issues("Warnings", &report.warnings);
}

fn print_issues(label: &str, issues: &[ValidationIssue]) {
    if issues.is_empty() {
        return;
    }

    println!("{label}:");
    for issue in issues {
        println!("  - [{}] {}: {}", issue.code, issue.field, issue.message);
    }
}