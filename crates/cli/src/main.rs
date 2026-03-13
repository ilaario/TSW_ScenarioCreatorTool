use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use tsw_scenario_compiler::{compile_project, BuildOptions};
use tsw_scenario_core::{
    load_all_route_profiles_from_root, load_route_profile_from_root, load_scenario_from_path,
    load_template_bundle_from_root, resolve_profiles_root, resolve_project_root,
    resolve_relative_to, resolve_templates_root, TemplateBundle,
};
use tsw_scenario_scanner::{
    build_location_catalog, build_stock_catalog, discover_formation_catalog, discover_route,
    load_formation_catalog, load_install_scan, load_location_catalog, load_route_discovery,
    load_stock_catalog, resolve_flattened_formation_entries, resolve_formation_matches,
    resolve_formation_reference_matches, resolve_location_matches,
    resolve_location_reference_matches, resolve_stock_matches,
    resolve_stock_reference_matches, save_formation_catalog, save_install_scan,
    save_location_catalog, save_route_discovery, save_stock_catalog, scan_game_install,
    CanonicalDlcCandidate, FlattenedFormationVehicle, FormationCatalog,
    FormationCatalogEntry, FormationMatchConstraints, InstallScan, LocationAllowedUse,
    LocationCatalog, LocationLookupUsage, LocationMatch, LocationMatchConstraints,
    RouteDiscovery, StockCatalog, StockCatalogEntry, StockMatchConstraints,
};
use tsw_scenario_schema::{RouteProfile, ScenarioAssetReference, ScenarioLocation, ScenarioProject};
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
    DiscoverRoute {
        route_id: String,
        #[arg(long)]
        route_dir: Option<PathBuf>,
        #[arg(long)]
        exports_dir: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    ListLocations {
        route_id: Option<String>,
        #[arg(long, value_enum)]
        usage: LocationUsage,
        #[arg(long)]
        json: bool,
    },
    ShowLocation {
        query: String,
        #[arg(long = "route")]
        route_id: Option<String>,
        #[arg(long, value_enum)]
        usage: LocationUsage,
        #[arg(long)]
        json: bool,
    },
    ListStock {
        route_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    ShowStock {
        query: String,
        #[arg(long = "route")]
        route_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    ListFormations {
        route_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    ShowFormation {
        query: String,
        #[arg(long = "route")]
        route_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Copy, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum LocationUsage {
    #[value(name = "player_spawn")]
    PlayerSpawn,
    #[value(name = "service")]
    Service,
    #[value(name = "objective")]
    Objective,
}

impl LocationUsage {
    fn as_str(self) -> &'static str {
        match self {
            Self::PlayerSpawn => "player_spawn",
            Self::Service => "service",
            Self::Objective => "objective",
        }
    }
}

#[derive(Debug)]
struct LoadedProjectContext {
    project: ScenarioProject,
    route_profile: RouteProfile,
    template_bundle: TemplateBundle,
    report: ValidationReport,
    install_scan: Option<InstallScan>,
    route_discovery: Option<RouteDiscovery>,
    location_catalog: Option<LocationCatalog>,
    stock_catalog: Option<StockCatalog>,
    formation_catalog: Option<FormationCatalog>,
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

#[derive(Debug, Clone, Serialize)]
struct LocationListOutput {
    route_id: String,
    usage: LocationUsage,
    count: usize,
    locations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ShowLocationOutput {
    route_id: String,
    query: String,
    usage: LocationUsage,
    match_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selected_catalog_id: Option<String>,
    matches: Vec<LocationMatch>,
}

#[derive(Debug, Clone, Serialize)]
struct StockListOutput {
    route_id: String,
    count: usize,
    stock: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ShowStockOutput {
    route_id: String,
    query: String,
    match_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selected_stock_id: Option<String>,
    matches: Vec<StockCatalogEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct FormationListOutput {
    route_id: String,
    count: usize,
    formations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ShowFormationOutput {
    route_id: String,
    query: String,
    match_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selected_formation_id: Option<String>,
    matches: Vec<FormationCatalogEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    flattened: Vec<FlattenedFormationVehicle>,
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
        Commands::DiscoverRoute {
            route_id,
            route_dir,
            exports_dir,
            output,
            json,
        } => run_discover_route(
            &project_root,
            &route_id,
            route_dir.as_deref(),
            exports_dir.as_deref(),
            output,
            json,
        ),
        Commands::ListLocations {
            route_id,
            usage,
            json,
        } => run_list_locations(
            &project_root,
            &cli.game_version,
            &cli.profiles_root,
            route_id.as_deref(),
            usage,
            json,
        ),
        Commands::ShowLocation {
            query,
            route_id,
            usage,
            json,
        } => run_show_location(
            &project_root,
            &cli.game_version,
            &cli.profiles_root,
            route_id.as_deref(),
            &query,
            usage,
            json,
        ),
        Commands::ListStock { route_id, json } => run_list_stock(
            &project_root,
            &cli.game_version,
            &cli.profiles_root,
            route_id.as_deref(),
            json,
        ),
        Commands::ShowStock {
            query,
            route_id,
            json,
        } => run_show_stock(
            &project_root,
            &cli.game_version,
            &cli.profiles_root,
            route_id.as_deref(),
            &query,
            json,
        ),
        Commands::ListFormations { route_id, json } => run_list_formations(
            &project_root,
            &cli.game_version,
            &cli.profiles_root,
            route_id.as_deref(),
            json,
        ),
        Commands::ShowFormation {
            query,
            route_id,
            json,
        } => run_show_formation(
            &project_root,
            &cli.game_version,
            &cli.profiles_root,
            route_id.as_deref(),
            &query,
            json,
        ),
    }
}

fn run_discover_route(
    project_root: &Path,
    route_id: &str,
    route_dir: Option<&Path>,
    exports_dir: Option<&Path>,
    output: Option<PathBuf>,
    json: bool,
) -> Result<()> {
    let resolved_route_dir = route_dir.map(|path| resolve_relative_to(project_root, path));
    let resolved_exports_dir = exports_dir.map(|path| resolve_relative_to(project_root, path));
    let discovery = discover_route(
        route_id,
        resolved_route_dir.as_deref(),
        resolved_exports_dir.as_deref(),
    )?;

    let output_path = output
        .map(|path| resolve_relative_to(project_root, &path))
        .unwrap_or_else(|| default_route_discovery_path(project_root, route_id));
    save_route_discovery(&discovery, &output_path).with_context(|| {
        format!("failed to save route discovery to '{}'", output_path.display())
    })?;

    let location_catalog = build_location_catalog(&discovery);
    let location_catalog_path = derive_location_catalog_path(project_root, route_id, &output_path);
    save_location_catalog(&location_catalog, &location_catalog_path).with_context(|| {
        format!("failed to save location catalog to '{}'", location_catalog_path.display())
    })?;

    let stock_catalog = build_stock_catalog(&discovery);
    let stock_catalog_path = derive_stock_catalog_path(project_root, route_id, &output_path);
    save_stock_catalog(&stock_catalog, &stock_catalog_path).with_context(|| {
        format!("failed to save stock catalog to '{}'", stock_catalog_path.display())
    })?;

    let formation_catalog = discover_formation_catalog(
        route_id,
        discovery.route_overview.as_ref(),
        resolved_exports_dir.as_deref(),
    )?;
    let formation_catalog_path = derive_formation_catalog_path(project_root, route_id, &output_path);
    save_formation_catalog(&formation_catalog, &formation_catalog_path).with_context(|| {
        format!("failed to save formation catalog to '{}'", formation_catalog_path.display())
    })?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&discovery)
                .context("failed to serialize route discovery as JSON")?
        );
    } else {
        println!("Route discovery completed for '{}'.", route_id);
        if let Some(route_overview) = &discovery.route_overview {
            if let Some(display_name) = &route_overview.display_name {
                println!("Route: {}", display_name);
            }
        }
        println!("Discovered {} frontend spawn point(s).", discovery.summary.frontend_spawn_point_count);
        println!("Discovered {} location candidate(s).", discovery.summary.location_count);
        println!("Discovered {} stock id(s).", discovery.summary.stock_count);
        println!("Route discovery artifact: {}", output_path.display());
        println!("Location catalog artifact: {}", location_catalog_path.display());
        println!("Stock catalog artifact: {}", stock_catalog_path.display());
        println!("Catalog stations: {}, grouped locations: {}, ungrouped locations: {}.",
            location_catalog.summary.station_count,
            location_catalog.summary.grouped_location_count,
            location_catalog.summary.ungrouped_location_count
        );
        println!(
            "Stock catalog entries: {}, plugins: {}.",
            stock_catalog.summary.stock_count,
            stock_catalog.summary.plugin_count
        );
    }

    Ok(())
}

fn run_list_locations(
    project_root: &Path,
    game_version: &str,
    profiles_root: &Path,
    route_id: Option<&str>,
    usage: LocationUsage,
    json: bool,
) -> Result<()> {
    let route_id = resolve_route_id_for_list_locations(
        project_root,
        game_version,
        profiles_root,
        route_id,
    )?;
    let resolved_profiles_root = resolve_profiles_root(project_root, profiles_root, game_version);
    let mut route_profile = load_route_profile_from_root(&resolved_profiles_root, &route_id)
        .with_context(|| {
            format!(
                "failed to load route profile '{}' from '{}'",
                route_id,
                resolved_profiles_root.display()
            )
        })?;

    let route_discovery = resolve_route_discovery(project_root, &route_id)?;
    if let Some(discovery) = &route_discovery {
        merge_route_discovery_into_route_profile(&mut route_profile, discovery);
    }

    let location_catalog = resolve_location_catalog(project_root, &route_id)?;
    if let Some(catalog) = &location_catalog {
        merge_location_catalog_into_route_profile(&mut route_profile, catalog);
    }

    let locations = if let Some(catalog) = &location_catalog {
        list_locations_from_catalog(catalog, usage)
    } else {
        list_locations_from_route_profile(&route_profile, usage)
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&LocationListOutput {
                route_id,
                usage,
                count: locations.len(),
                locations,
            })
            .context("failed to serialize location list as JSON")?
        );
    } else {
        println!(
            "Locations for route '{}' and usage '{}': {}",
            route_id,
            usage.as_str(),
            locations.len()
        );
        for location in locations {
            println!("  - {}", location);
        }
    }

    Ok(())
}

fn run_show_location(
    project_root: &Path,
    game_version: &str,
    profiles_root: &Path,
    route_id: Option<&str>,
    query: &str,
    usage: LocationUsage,
    json: bool,
) -> Result<()> {
    let route_id = resolve_route_id_for_list_locations(
        project_root,
        game_version,
        profiles_root,
        route_id,
    )?;
    let location_catalog = resolve_location_catalog(project_root, &route_id)?.with_context(|| {
        format!(
            "location catalog '{}' not found; run discover-route first",
            default_location_catalog_path(project_root, &route_id).display()
        )
    })?;

    let matches = resolve_location_matches(&location_catalog, query, location_lookup_usage(usage));
    let selected_catalog_id = matches.first().map(|location| location.catalog_id.clone());

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&ShowLocationOutput {
                route_id,
                query: query.to_string(),
                usage,
                match_count: matches.len(),
                selected_catalog_id,
                matches,
            })
            .context("failed to serialize location matches as JSON")?
        );
    } else {
        println!(
            "Location matches for route '{}' and usage '{}': {}",
            route_id,
            usage.as_str(),
            matches.len()
        );
        println!("Query: {}", query);

        if matches.is_empty() {
            println!("No matching catalog entries.");
            return Ok(());
        }

        println!("Selected candidate:");
        println!("  - {}", format_location_match_details(&matches[0]));

        if matches.len() > 1 {
            println!("All candidates:");
            for (index, location) in matches.iter().enumerate() {
                let selected_marker = if index == 0 { " (selected)" } else { "" };
                println!("  - {}{}", format_location_match_details(location), selected_marker);
            }
        }
    }

    Ok(())
}
fn run_list_stock(
    project_root: &Path,
    game_version: &str,
    profiles_root: &Path,
    route_id: Option<&str>,
    json: bool,
) -> Result<()> {
    let route_id = resolve_route_id_for_list_locations(
        project_root,
        game_version,
        profiles_root,
        route_id,
    )?;
    let resolved_profiles_root = resolve_profiles_root(project_root, profiles_root, game_version);
    let mut route_profile = load_route_profile_from_root(&resolved_profiles_root, &route_id)
        .with_context(|| {
            format!(
                "failed to load route profile '{}' from '{}'",
                route_id,
                resolved_profiles_root.display()
            )
        })?;

    let route_discovery = resolve_route_discovery(project_root, &route_id)?;
    if let Some(discovery) = &route_discovery {
        merge_route_discovery_into_route_profile(&mut route_profile, discovery);
    }

    let stock = if let Some(catalog) = resolve_stock_catalog(project_root, &route_id)? {
        list_stock_from_catalog(&catalog)
    } else {
        list_stock_from_route_profile(&route_profile)
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&StockListOutput {
                route_id,
                count: stock.len(),
                stock,
            })
            .context("failed to serialize stock list as JSON")?
        );
    } else {
        println!("Stock for route '{}': {}", route_id, stock.len());
        for stock_id in stock {
            println!("  - {}", stock_id);
        }
    }

    Ok(())
}

fn run_show_stock(
    project_root: &Path,
    game_version: &str,
    profiles_root: &Path,
    route_id: Option<&str>,
    query: &str,
    json: bool,
) -> Result<()> {
    let route_id = resolve_route_id_for_list_locations(
        project_root,
        game_version,
        profiles_root,
        route_id,
    )?;
    let stock_catalog = resolve_stock_catalog(project_root, &route_id)?.with_context(|| {
        format!(
            "stock catalog '{}' not found; run discover-route first",
            default_stock_catalog_path(project_root, &route_id).display()
        )
    })?;

    let matches = resolve_stock_matches(&stock_catalog, query);
    let selected_stock_id = matches.first().map(|entry| entry.id.clone());

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&ShowStockOutput {
                route_id,
                query: query.to_string(),
                match_count: matches.len(),
                selected_stock_id,
                matches,
            })
            .context("failed to serialize stock matches as JSON")?
        );
    } else {
        println!("Stock matches for route '{}': {}", route_id, matches.len());
        println!("Query: {}", query);

        if matches.is_empty() {
            println!("No matching stock entries.");
            return Ok(());
        }

        println!("Selected candidate:");
        println!("  - {}", format_stock_match_details(&matches[0]));

        if matches.len() > 1 {
            println!("All candidates:");
            for (index, stock) in matches.iter().enumerate() {
                let selected_marker = if index == 0 { " (selected)" } else { "" };
                println!("  - {}{}", format_stock_match_details(stock), selected_marker);
            }
        }
    }

    Ok(())
}

fn run_list_formations(
    project_root: &Path,
    game_version: &str,
    profiles_root: &Path,
    route_id: Option<&str>,
    json: bool,
) -> Result<()> {
    let route_id = resolve_route_id_for_list_locations(
        project_root,
        game_version,
        profiles_root,
        route_id,
    )?;
    let formation_catalog = resolve_formation_catalog(project_root, &route_id)?.with_context(|| {
        format!(
            "formation catalog '{}' not found; run discover-route first",
            default_formation_catalog_path(project_root, &route_id).display()
        )
    })?;

    let formations = list_formations_from_catalog(&formation_catalog);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&FormationListOutput {
                route_id,
                count: formations.len(),
                formations,
            })
            .context("failed to serialize formation list as JSON")?
        );
    } else {
        println!("Formations for route '{}': {}", route_id, formations.len());
        for formation_id in formations {
            println!("  - {}", formation_id);
        }
    }

    Ok(())
}

fn run_show_formation(
    project_root: &Path,
    game_version: &str,
    profiles_root: &Path,
    route_id: Option<&str>,
    query: &str,
    json: bool,
) -> Result<()> {
    let route_id = resolve_route_id_for_list_locations(
        project_root,
        game_version,
        profiles_root,
        route_id,
    )?;
    let formation_catalog = resolve_formation_catalog(project_root, &route_id)?.with_context(|| {
        format!(
            "formation catalog '{}' not found; run discover-route first",
            default_formation_catalog_path(project_root, &route_id).display()
        )
    })?;

    let matches = resolve_formation_matches(&formation_catalog, query);
    let selected_formation_id = matches.first().map(|entry| entry.id.clone());
    let flattened = if let Some(selected) = matches.first() {
        resolve_flattened_formation_entries(&formation_catalog, &selected.id)?
    } else {
        Vec::new()
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&ShowFormationOutput {
                route_id,
                query: query.to_string(),
                match_count: matches.len(),
                selected_formation_id,
                matches,
                flattened,
            })
            .context("failed to serialize formation matches as JSON")?
        );
    } else {
        println!("Formation matches for route '{}': {}", route_id, matches.len());
        println!("Query: {}", query);

        if matches.is_empty() {
            println!("No matching formation entries.");
            return Ok(());
        }

        println!("Selected candidate:");
        println!("  - {}", format_formation_match_details(&matches[0]));

        if matches.len() > 1 {
            println!("All candidates:");
            for (index, formation) in matches.iter().enumerate() {
                let selected_marker = if index == 0 { " (selected)" } else { "" };
                println!("  - {}{}", format_formation_match_details(formation), selected_marker);
            }
        }

        if !flattened.is_empty() {
            println!("Flattened vehicles:");
            for vehicle in &flattened {
                println!("  - {}", format_flattened_formation_vehicle(vehicle));
            }
        }
    }

    Ok(())
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
            if let Some(route_discovery) = &context.route_discovery {
                println!(
                    "Route discovery in use: {} frontend spawn point(s), {} location(s), {} stock id(s).",
                    route_discovery.summary.frontend_spawn_point_count,
                    route_discovery.summary.location_count,
                    route_discovery.summary.stock_count
                );
            }
            if let Some(location_catalog) = &context.location_catalog {
                println!(
                    "Location catalog in use: {} station(s), {} grouped location(s), {} ungrouped location(s).",
                    location_catalog.summary.station_count,
                    location_catalog.summary.grouped_location_count,
                    location_catalog.summary.ungrouped_location_count
                );
            }        }
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
            location_catalog: context.location_catalog.clone(),
            stock_catalog: context.stock_catalog.clone(),
            formation_catalog: context.formation_catalog.clone(),
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
    if let Some(route_discovery) = &context.route_discovery {
        println!(
            "Route discovery in use: {} frontend spawn point(s), {} location(s), {} stock id(s).",
            route_discovery.summary.frontend_spawn_point_count,
            route_discovery.summary.location_count,
            route_discovery.summary.stock_count
        );
    }
    if let Some(location_catalog) = &context.location_catalog {
        println!(
            "Location catalog in use: {} station(s), {} grouped location(s), {} ungrouped location(s).",
            location_catalog.summary.station_count,
            location_catalog.summary.grouped_location_count,
            location_catalog.summary.ungrouped_location_count
        );
    }

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
    let mut route_profile = load_route_profile_from_root(&resolved_profiles_root, &project.scenario.route)
        .with_context(|| {
            format!(
                "failed to load route profile '{}' from '{}'",
                project.scenario.route,
                resolved_profiles_root.display()
            )
        })?;
    let route_discovery = resolve_route_discovery(project_root, &project.scenario.route)?;
    if let Some(discovery) = &route_discovery {
        merge_route_discovery_into_route_profile(&mut route_profile, discovery);
    }
    let location_catalog = resolve_location_catalog(project_root, &project.scenario.route)?;
    if let Some(catalog) = &location_catalog {
        merge_location_catalog_into_route_profile(&mut route_profile, catalog);
    }
    let stock_catalog = resolve_stock_catalog(project_root, &project.scenario.route)?;
    if let Some(catalog) = &stock_catalog {
        merge_stock_catalog_into_route_profile(&mut route_profile, catalog);
    }
    let formation_catalog = resolve_formation_catalog(project_root, &project.scenario.route)?;

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
    if let Some(catalog) = &location_catalog {
        add_location_resolution_issues(&mut report, &project, catalog);
    }
    add_service_reference_issues(
        &mut report,
        &project,
        stock_catalog.as_ref(),
        formation_catalog.as_ref(),
    );
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
        route_discovery,
        location_catalog,
        stock_catalog,
        formation_catalog,
    })
}

fn resolve_route_discovery(project_root: &Path, route_id: &str) -> Result<Option<RouteDiscovery>> {
    let path = default_route_discovery_path(project_root, route_id);
    if !path.exists() {
        return Ok(None);
    }

    let discovery = load_route_discovery(&path)
        .with_context(|| format!("failed to load route discovery '{}'", path.display()))?;
    Ok(Some(discovery))
}

fn resolve_location_catalog(project_root: &Path, route_id: &str) -> Result<Option<LocationCatalog>> {
    let path = default_location_catalog_path(project_root, route_id);
    if !path.exists() {
        return Ok(None);
    }

    let catalog = load_location_catalog(&path)
        .with_context(|| format!("failed to load location catalog '{}'", path.display()))?;
    Ok(Some(catalog))
}

fn resolve_stock_catalog(project_root: &Path, route_id: &str) -> Result<Option<StockCatalog>> {
    let path = default_stock_catalog_path(project_root, route_id);
    if !path.exists() {
        return Ok(None);
    }

    let catalog = load_stock_catalog(&path)
        .with_context(|| format!("failed to load stock catalog '{}'", path.display()))?;
    Ok(Some(catalog))
}

fn resolve_formation_catalog(project_root: &Path, route_id: &str) -> Result<Option<FormationCatalog>> {
    let path = default_formation_catalog_path(project_root, route_id);
    if !path.exists() {
        return Ok(None);
    }

    let catalog = load_formation_catalog(&path)
        .with_context(|| format!("failed to load formation catalog '{}'", path.display()))?;
    Ok(Some(catalog))
}

fn derive_location_catalog_path(project_root: &Path, route_id: &str, discovery_output_path: &Path) -> PathBuf {
    let default_path = default_location_catalog_path(project_root, route_id);

    let Some(file_name) = discovery_output_path.file_name().and_then(|value| value.to_str()) else {
        return default_path;
    };

    if file_name.ends_with(".route_discovery.json") {
        let catalog_name = file_name.replace(".route_discovery.json", ".location_catalog.json");
        return discovery_output_path.with_file_name(catalog_name);
    }

    let mut custom_file_name = file_name.to_string();
    custom_file_name.push_str(".location_catalog.json");
    discovery_output_path.with_file_name(custom_file_name)
}

fn default_location_catalog_path(project_root: &Path, route_id: &str) -> PathBuf {
    project_root
        .join("artifacts")
        .join("discovery")
        .join(format!("{}.location_catalog.json", discovery_slug(route_id)))
}

fn derive_stock_catalog_path(project_root: &Path, route_id: &str, discovery_output_path: &Path) -> PathBuf {
    let default_path = default_stock_catalog_path(project_root, route_id);

    let Some(file_name) = discovery_output_path.file_name().and_then(|value| value.to_str()) else {
        return default_path;
    };

    if file_name.ends_with(".route_discovery.json") {
        let stock_name = file_name.replace(".route_discovery.json", ".stock_catalog.json");
        return discovery_output_path.with_file_name(stock_name);
    }

    let mut custom_file_name = file_name.to_string();
    custom_file_name.push_str(".stock_catalog.json");
    discovery_output_path.with_file_name(custom_file_name)
}

fn default_stock_catalog_path(project_root: &Path, route_id: &str) -> PathBuf {
    project_root
        .join("artifacts")
        .join("discovery")
        .join(format!("{}.stock_catalog.json", discovery_slug(route_id)))
}

fn derive_formation_catalog_path(project_root: &Path, route_id: &str, discovery_output_path: &Path) -> PathBuf {
    let default_path = default_formation_catalog_path(project_root, route_id);

    let Some(file_name) = discovery_output_path.file_name().and_then(|value| value.to_str()) else {
        return default_path;
    };

    if file_name.ends_with(".route_discovery.json") {
        let formation_name = file_name.replace(".route_discovery.json", ".formation_catalog.json");
        return discovery_output_path.with_file_name(formation_name);
    }

    let mut custom_file_name = file_name.to_string();
    custom_file_name.push_str(".formation_catalog.json");
    discovery_output_path.with_file_name(custom_file_name)
}

fn default_formation_catalog_path(project_root: &Path, route_id: &str) -> PathBuf {
    project_root
        .join("artifacts")
        .join("discovery")
        .join(format!("{}.formation_catalog.json", discovery_slug(route_id)))
}

fn default_route_discovery_path(project_root: &Path, route_id: &str) -> PathBuf {
    project_root
        .join("artifacts")
        .join("discovery")
        .join(format!("{}.route_discovery.json", discovery_slug(route_id)))
}

fn discovery_slug(route_id: &str) -> String {
    let slug: String = route_id
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect();

    if slug.is_empty() {
        "route".to_string()
    } else {
        slug
    }
}

fn merge_route_discovery_into_route_profile(
    route_profile: &mut RouteProfile,
    route_discovery: &RouteDiscovery,
) {
    for spawn_point in &route_discovery.frontend_spawn_points {
        push_unique_case_insensitive(&mut route_profile.spawn_points, &spawn_point.display_name);
        push_unique_case_insensitive(&mut route_profile.spawn_points, &spawn_point.tag);
        push_unique_case_insensitive(
            &mut route_profile.player_spawn_points,
            &spawn_point.display_name,
        );
        push_unique_case_insensitive(&mut route_profile.player_spawn_points, &spawn_point.tag);
    }

    for location in &route_discovery.locations {
        match location.kind.as_str() {
            "platform" | "siding" | "track" | "yard" => {
                push_location_identifier(&mut route_profile.service_locations, &location.display_name);
                push_location_identifier(&mut route_profile.service_locations, &location.id);
                push_location_identifier(&mut route_profile.objective_locations, &location.display_name);
                push_location_identifier(&mut route_profile.objective_locations, &location.id);
            }
            "portal" => {
                push_location_identifier(&mut route_profile.service_locations, &location.display_name);
                push_location_identifier(&mut route_profile.service_locations, &location.id);
            }
            _ => {}
        }
    }

    for stock in &route_discovery.stock {
        push_unique_case_insensitive(&mut route_profile.supported_stock, &stock.id);
    }

    if let Some(route_overview) = &route_discovery.route_overview {
        if let Some(display_name) = &route_overview.display_name {
            push_unique_case_insensitive(&mut route_profile.install_hints, display_name);
        }
    }

    for route_title in &route_discovery.route_titles {
        push_unique_case_insensitive(&mut route_profile.install_hints, route_title);
    }
}

fn merge_stock_catalog_into_route_profile(
    route_profile: &mut RouteProfile,
    stock_catalog: &StockCatalog,
) {
    for stock in &stock_catalog.stock {
        push_unique_case_insensitive(&mut route_profile.supported_stock, &stock.id);
    }
}

fn merge_location_catalog_into_route_profile(
    route_profile: &mut RouteProfile,
    location_catalog: &LocationCatalog,
) {
    for station in &location_catalog.stations {
        if station
            .allowed_uses
            .iter()
            .any(|use_case| *use_case == LocationAllowedUse::PlayerSpawn)
        {
            push_unique_case_insensitive(&mut route_profile.spawn_points, &station.display_name);
            push_unique_case_insensitive(&mut route_profile.spawn_points, &station.id);
            push_unique_case_insensitive(
                &mut route_profile.player_spawn_points,
                &station.display_name,
            );
            push_unique_case_insensitive(&mut route_profile.player_spawn_points, &station.id);
            for tag in &station.tags {
                push_unique_case_insensitive(&mut route_profile.spawn_points, tag);
                push_unique_case_insensitive(&mut route_profile.player_spawn_points, tag);
            }
        }

        for frontend_spawn_point in &station.frontend_spawn_points {
            push_unique_case_insensitive(
                &mut route_profile.spawn_points,
                &frontend_spawn_point.display_name,
            );
            push_unique_case_insensitive(&mut route_profile.spawn_points, &frontend_spawn_point.tag);
            push_unique_case_insensitive(
                &mut route_profile.player_spawn_points,
                &frontend_spawn_point.display_name,
            );
            push_unique_case_insensitive(
                &mut route_profile.player_spawn_points,
                &frontend_spawn_point.tag,
            );
        }

        for location in &station.locations {
            merge_location_catalog_entry(route_profile, location);
        }
    }

    for location in &location_catalog.ungrouped_locations {
        merge_location_catalog_entry(route_profile, location);
    }
}

fn merge_location_catalog_entry(
    route_profile: &mut RouteProfile,
    location: &tsw_scenario_scanner::LocationCatalogEntry,
) {
    for use_case in &location.allowed_uses {
        match use_case {
            LocationAllowedUse::PlayerSpawn => {
                push_location_identifier(&mut route_profile.player_spawn_points, &location.display_name);
                push_location_identifier(&mut route_profile.player_spawn_points, &location.id);
                if let Some(internal_ref) = &location.internal_ref {
                    push_location_identifier(&mut route_profile.player_spawn_points, internal_ref);
                }
                push_location_identifier(&mut route_profile.spawn_points, &location.display_name);
                push_location_identifier(&mut route_profile.spawn_points, &location.id);
                if let Some(internal_ref) = &location.internal_ref {
                    push_location_identifier(&mut route_profile.spawn_points, internal_ref);
                }
            }
            LocationAllowedUse::ServiceStart | LocationAllowedUse::ServiceEnd => {
                push_location_identifier(&mut route_profile.service_locations, &location.display_name);
                push_location_identifier(&mut route_profile.service_locations, &location.id);
                if let Some(internal_ref) = &location.internal_ref {
                    push_location_identifier(&mut route_profile.service_locations, internal_ref);
                }
            }
            LocationAllowedUse::ObjectiveLocation => {
                push_location_identifier(&mut route_profile.objective_locations, &location.display_name);
                push_location_identifier(&mut route_profile.objective_locations, &location.id);
                if let Some(internal_ref) = &location.internal_ref {
                    push_location_identifier(&mut route_profile.objective_locations, internal_ref);
                }
            }
            LocationAllowedUse::ReviewOnly => {}
        }
    }
}

fn resolve_route_id_for_list_locations(
    project_root: &Path,
    game_version: &str,
    profiles_root: &Path,
    route_id: Option<&str>,
) -> Result<String> {
    if let Some(route_id) = route_id {
        return Ok(route_id.to_string());
    }

    let mut discovered_route_ids = Vec::new();
    let discovery_root = project_root.join("artifacts").join("discovery");
    if discovery_root.is_dir() {
        for entry in fs::read_dir(&discovery_root)
            .with_context(|| format!("failed to read discovery directory '{}'", discovery_root.display()))?
        {
            let entry = entry.with_context(|| {
                format!("failed to enumerate discovery directory '{}'", discovery_root.display())
            })?;
            let path = entry.path();
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !file_name.ends_with(".location_catalog.json") {
                continue;
            }

            let catalog = load_location_catalog(&path).with_context(|| {
                format!("failed to load location catalog '{}'", path.display())
            })?;
            push_unique_case_insensitive(&mut discovered_route_ids, &catalog.route_id);
        }
    }

    if discovered_route_ids.len() == 1 {
        return Ok(discovered_route_ids.remove(0));
    }

    if discovered_route_ids.len() > 1 {
        discovered_route_ids.sort();
        bail!(
            "multiple location catalogs found; pass a route id: {}",
            discovered_route_ids.join(", ")
        );
    }

    let resolved_profiles_root = resolve_profiles_root(project_root, profiles_root, game_version);
    let route_profiles = load_all_route_profiles_from_root(&resolved_profiles_root)
        .with_context(|| {
            format!(
                "failed to load route profiles from '{}'",
                resolved_profiles_root.display()
            )
        })?;

    if route_profiles.len() == 1 {
        return Ok(route_profiles[0].id.clone());
    }

    bail!(
        "could not resolve a route for the location command; pass a route id or create exactly one location catalog artifact"
    )
}

fn location_lookup_usage(usage: LocationUsage) -> LocationLookupUsage {
    match usage {
        LocationUsage::PlayerSpawn => LocationLookupUsage::PlayerSpawn,
        LocationUsage::Service => LocationLookupUsage::Service,
        LocationUsage::Objective => LocationLookupUsage::Objective,
    }
}

fn list_locations_from_catalog(catalog: &LocationCatalog, usage: LocationUsage) -> Vec<String> {
    let mut locations = BTreeSet::new();

    match usage {
        LocationUsage::PlayerSpawn => {
            collect_player_spawn_locations_from_catalog(catalog, &mut locations);
        }
        LocationUsage::Service => {
            collect_player_spawn_locations_from_catalog(catalog, &mut locations);
            collect_locations_by_allowed_uses(
                catalog,
                &[LocationAllowedUse::ServiceStart, LocationAllowedUse::ServiceEnd],
                &mut locations,
            );
        }
        LocationUsage::Objective => {
            collect_player_spawn_locations_from_catalog(catalog, &mut locations);
            collect_locations_by_allowed_uses(
                catalog,
                &[
                    LocationAllowedUse::ServiceStart,
                    LocationAllowedUse::ServiceEnd,
                    LocationAllowedUse::ObjectiveLocation,
                ],
                &mut locations,
            );
        }
    }

    locations.into_iter().collect()
}

fn collect_player_spawn_locations_from_catalog(
    catalog: &LocationCatalog,
    locations: &mut BTreeSet<String>,
) {
    for station in &catalog.stations {
        if station
            .allowed_uses
            .iter()
            .any(|use_case| *use_case == LocationAllowedUse::PlayerSpawn)
        {
            insert_location_label(locations, &station.display_name);
            for tag in &station.tags {
                insert_location_label(locations, tag);
            }
        }

        for frontend_spawn_point in &station.frontend_spawn_points {
            insert_location_label(locations, &frontend_spawn_point.display_name);
            insert_location_label(locations, &frontend_spawn_point.tag);
        }
    }
}

fn collect_locations_by_allowed_uses(
    catalog: &LocationCatalog,
    allowed_uses: &[LocationAllowedUse],
    locations: &mut BTreeSet<String>,
) {
    for station in &catalog.stations {
        for location in &station.locations {
            if location
                .allowed_uses
                .iter()
                .any(|use_case| allowed_uses.contains(use_case))
            {
                insert_location_label(locations, &location.display_name);
            }
        }
    }

    for location in &catalog.ungrouped_locations {
        if location
            .allowed_uses
            .iter()
            .any(|use_case| allowed_uses.contains(use_case))
        {
            insert_location_label(locations, &location.display_name);
        }
    }
}

fn list_locations_from_route_profile(
    route_profile: &RouteProfile,
    usage: LocationUsage,
) -> Vec<String> {
    let mut locations = Vec::new();

    match usage {
        LocationUsage::PlayerSpawn => {
            for location in &route_profile.spawn_points {
                push_unique_case_insensitive(&mut locations, location);
            }
            for location in &route_profile.player_spawn_points {
                push_unique_case_insensitive(&mut locations, location);
            }
        }
        LocationUsage::Service => {
            for location in &route_profile.spawn_points {
                push_unique_case_insensitive(&mut locations, location);
            }
            for location in &route_profile.player_spawn_points {
                push_unique_case_insensitive(&mut locations, location);
            }
            for location in &route_profile.service_locations {
                push_unique_case_insensitive(&mut locations, location);
            }
        }
        LocationUsage::Objective => {
            for location in &route_profile.spawn_points {
                push_unique_case_insensitive(&mut locations, location);
            }
            for location in &route_profile.player_spawn_points {
                push_unique_case_insensitive(&mut locations, location);
            }
            for location in &route_profile.service_locations {
                push_unique_case_insensitive(&mut locations, location);
            }
            for location in &route_profile.objective_locations {
                push_unique_case_insensitive(&mut locations, location);
            }
        }
    }

    locations.sort();
    locations
}

fn list_stock_from_catalog(catalog: &StockCatalog) -> Vec<String> {
    let mut stock = catalog
        .stock
        .iter()
        .map(|entry| entry.id.clone())
        .collect::<Vec<_>>();
    stock.sort();
    stock.dedup();
    stock
}

fn list_formations_from_catalog(catalog: &FormationCatalog) -> Vec<String> {
    let mut formations = catalog
        .formations
        .iter()
        .map(|entry| entry.id.clone())
        .collect::<Vec<_>>();
    formations.sort();
    formations.dedup();
    formations
}

fn list_stock_from_route_profile(route_profile: &RouteProfile) -> Vec<String> {
    let mut stock = route_profile.supported_stock.clone();
    stock.sort();
    stock.dedup();
    stock
}

fn format_stock_match_details(stock: &StockCatalogEntry) -> String {
    let mut segments = vec![stock.id.clone()];

    if let Some(plugin) = &stock.plugin {
        segments.push(format!("plugin={plugin}"));
    }

    segments.push(format!("source={}", stock.source));
    segments.push(format!("evidence={}", stock.evidence));
    segments.join(", ")
}

fn format_formation_match_details(formation: &FormationCatalogEntry) -> String {
    let mut segments = vec![formation.id.clone()];

    if let Some(plugin) = &formation.plugin {
        segments.push(format!("plugin={plugin}"));
    }

    segments.push(format!("entries={}", formation.entries.len()));
    if let Some(drivable) = formation.drivable {
        segments.push(format!("drivable={drivable}"));
    }
    segments.push(format!("source={}", formation.source));
    segments.join(", ")
}

fn format_flattened_formation_vehicle(vehicle: &FlattenedFormationVehicle) -> String {
    let mut segments = vec![vehicle.vehicle_id.clone()];
    if vehicle.flipped {
        segments.push("flipped=true".to_string());
    }
    if let Some(cargo_asset) = &vehicle.cargo_asset {
        segments.push(format!("cargo={cargo_asset}"));
    }
    if let Some(cargo_units) = vehicle.cargo_units {
        segments.push(format!("cargo_units={cargo_units}"));
    }
    segments.join(", ")
}

fn insert_location_label(locations: &mut BTreeSet<String>, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }

    locations.insert(trimmed.to_string());
}
fn push_location_identifier(values: &mut Vec<String>, value: &str) {
    push_unique_case_insensitive(values, value);
}
fn push_unique_case_insensitive(values: &mut Vec<String>, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }

    if values
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(trimmed))
    {
        return;
    }

    values.push(trimmed.to_string());
}

fn add_service_reference_issues(
    report: &mut ValidationReport,
    project: &ScenarioProject,
    stock_catalog: Option<&StockCatalog>,
    formation_catalog: Option<&FormationCatalog>,
) {
    add_service_reference_issue(
        report,
        "player_service",
        project.player_service.consist.as_ref(),
        project.player_service.formation_ref.as_ref(),
        stock_catalog,
        formation_catalog,
    );

    for (index, service) in project.ai_services.iter().enumerate() {
        add_service_reference_issue(
            report,
            &format!("ai_services[{index}]"),
            service.consist.as_ref(),
            service.formation_ref.as_ref(),
            stock_catalog,
            formation_catalog,
        );
    }
}

fn add_service_reference_issue(
    report: &mut ValidationReport,
    field_prefix: &str,
    consist: Option<&ScenarioAssetReference>,
    formation_ref: Option<&ScenarioAssetReference>,
    stock_catalog: Option<&StockCatalog>,
    formation_catalog: Option<&FormationCatalog>,
) {
    if let Some(reference) = consist {
        if let Some(stock_catalog) = stock_catalog {
            let matches = resolve_stock_reference_matches(
                stock_catalog,
                reference.id().unwrap_or_default(),
                &stock_match_constraints(reference),
            );
            if matches.is_empty() && reference.has_explicit_selector() {
                report.errors.push(ValidationIssue {
                    code: "INVALID_STOCK_SELECTOR".to_string(),
                    field: format!("{field_prefix}.consist"),
                    message: format!(
                        "stock '{}' does not match any discovered stock entry with the requested selector(s)",
                        reference.display_label()
                    ),
                });
            } else if matches.len() > 1 {
                report.warnings.push(ValidationIssue {
                    code: "AMBIGUOUS_STOCK_REFERENCE".to_string(),
                    field: format!("{field_prefix}.consist"),
                    message: format!(
                        "stock '{}' matches {} discovered stock entries; the first match will be used",
                        reference.display_label(),
                        matches.len()
                    ),
                });
            }
        }
    }

    if let Some(reference) = formation_ref {
        let Some(formation_catalog) = formation_catalog else {
            report.errors.push(ValidationIssue {
                code: "MISSING_FORMATION_CATALOG".to_string(),
                field: format!("{field_prefix}.formation_ref"),
                message: "formation_ref requires a route discovery formation catalog; run discover-route first".to_string(),
            });
            return;
        };

        let matches = resolve_formation_reference_matches(
            formation_catalog,
            reference.id().unwrap_or_default(),
            &formation_match_constraints(reference),
        );

        if matches.is_empty() {
            report.errors.push(ValidationIssue {
                code: "INVALID_FORMATION_SELECTOR".to_string(),
                field: format!("{field_prefix}.formation_ref"),
                message: format!(
                    "formation '{}' does not match any discovered formation entry with the requested selector(s)",
                    reference.display_label()
                ),
            });
            return;
        }

        if matches.len() > 1 {
            report.warnings.push(ValidationIssue {
                code: "AMBIGUOUS_FORMATION_REFERENCE".to_string(),
                field: format!("{field_prefix}.formation_ref"),
                message: format!(
                    "formation '{}' matches {} discovered formation entries; the first match will be used",
                    reference.display_label(),
                    matches.len()
                ),
            });
        }
    }
}

fn stock_match_constraints(reference: &ScenarioAssetReference) -> StockMatchConstraints {
    StockMatchConstraints {
        plugin: reference.plugin().map(str::to_string),
        source: reference.source().map(str::to_string),
    }
}

fn formation_match_constraints(reference: &ScenarioAssetReference) -> FormationMatchConstraints {
    FormationMatchConstraints {
        plugin: reference.plugin().map(str::to_string),
        source: reference.source().map(str::to_string),
    }
}

fn add_location_resolution_issues(
    report: &mut ValidationReport,
    project: &ScenarioProject,
    location_catalog: &LocationCatalog,
) {
    add_location_resolution_issue(
        report,
        location_catalog,
        "player_service.start_location",
        &project.player_service.start_location,
        LocationLookupUsage::PlayerSpawn,
    );
    add_location_resolution_issue(
        report,
        location_catalog,
        "player_service.destination",
        &project.player_service.destination,
        LocationLookupUsage::Service,
    );

    for (index, ai_service) in project.ai_services.iter().enumerate() {
        add_location_resolution_issue(
            report,
            location_catalog,
            &format!("ai_services[{index}].start_location"),
            &ai_service.start_location,
            LocationLookupUsage::Service,
        );
        add_location_resolution_issue(
            report,
            location_catalog,
            &format!("ai_services[{index}].destination"),
            &ai_service.destination,
            LocationLookupUsage::Service,
        );
    }

    for (index, objective) in project.objectives.iter().enumerate() {
        if let Some(location) = objective.location.as_ref() {
            add_location_resolution_issue(
                report,
                location_catalog,
                &format!("objectives[{index}].location"),
                location,
                LocationLookupUsage::Objective,
            );
        }
    }
}

fn add_location_resolution_issue(
    report: &mut ValidationReport,
    location_catalog: &LocationCatalog,
    field: &str,
    location: &ScenarioLocation,
    usage: LocationLookupUsage,
) {
    let matches = resolve_location_reference_matches(
        location_catalog,
        location_lookup_query(location),
        usage,
        &location_match_constraints(location),
    );

    if matches.is_empty() {
        if location.has_explicit_selector() {
            report.errors.push(ValidationIssue {
                code: "INVALID_LOCATION_SELECTOR".to_string(),
                field: field.to_string(),
                message: format!(
                    "location '{}' does not match any catalog entry for usage '{}' with the requested selector(s)",
                    location.display_label(),
                    format_location_lookup_usage(usage)
                ),
            });
        }
        return;
    }

    if matches.len() <= 1 {
        return;
    }

    let selected = &matches[0];
    let preview = matches
        .iter()
        .take(3)
        .map(format_location_match_preview)
        .collect::<Vec<_>>()
        .join(", ");

    report.warnings.push(ValidationIssue {
        code: "AMBIGUOUS_LOCATION".to_string(),
        field: field.to_string(),
        message: format!(
            "location '{}' matches {} catalog entries; compiler will use '{}' [{}]{}. Candidates: {}",
            location.display_label(),
            matches.len(),
            selected.display_name,
            selected.catalog_id,
            selected
                .internal_ref
                .as_deref()
                .map(|internal_ref| format!(", internal_ref={internal_ref}"))
                .unwrap_or_default(),
            preview
        ),
    });
}

fn location_lookup_query(location: &ScenarioLocation) -> Option<&str> {
    location.name().or_else(|| location.primary_value())
}

fn location_match_constraints(location: &ScenarioLocation) -> LocationMatchConstraints {
    LocationMatchConstraints {
        catalog_id: location.catalog_id().map(str::to_string),
        spawn_tag: location.spawn_tag().map(str::to_string),
        internal_ref: location.internal_ref().map(str::to_string),
    }
}

fn format_location_lookup_usage(usage: LocationLookupUsage) -> &'static str {
    match usage {
        LocationLookupUsage::PlayerSpawn => "player_spawn",
        LocationLookupUsage::Service => "service",
        LocationLookupUsage::Objective => "objective",
    }
}

fn format_location_match_preview(location: &tsw_scenario_scanner::LocationMatch) -> String {
    match (&location.spawn_tag, &location.internal_ref) {
        (Some(spawn_tag), Some(internal_ref)) => {
            format!("{} [{} | {}]", location.display_name, spawn_tag, internal_ref)
        }
        (Some(spawn_tag), None) => format!("{} [{}]", location.display_name, spawn_tag),
        (None, Some(internal_ref)) => {
            format!("{} [{} | {}]", location.display_name, location.catalog_id, internal_ref)
        }
        (None, None) => format!("{} [{}]", location.display_name, location.catalog_id),
    }
}

fn format_location_match_details(location: &LocationMatch) -> String {
    let mut segments = vec![format!(
        "{} [{}]",
        location.display_name, location.catalog_id
    )];
    segments.push(format!("match={}", format_location_match_kind(location)));
    segments.push(format!("kind={}", location.kind));
    segments.push(format!("confidence={}", format_location_confidence(location)));

    if let Some(spawn_tag) = &location.spawn_tag {
        if spawn_tag != &location.catalog_id {
            segments.push(format!("spawn_tag={spawn_tag}"));
        }
    }

    if let Some(internal_ref) = &location.internal_ref {
        segments.push(format!("internal_ref={internal_ref}"));
    }

    if let Some(source_kind) = location.source_kind {
        segments.push(format!("source_kind={}", format_location_source_kind(source_kind)));
    }

    if let Some(source) = &location.source {
        segments.push(format!("source={source}"));
    }

    segments.join(", ")
}

fn format_location_match_kind(location: &LocationMatch) -> &'static str {
    match location.match_kind {
        tsw_scenario_scanner::LocationMatchKind::FrontendSpawnPoint => "frontend_spawn_point",
        tsw_scenario_scanner::LocationMatchKind::Station => "station",
        tsw_scenario_scanner::LocationMatchKind::CatalogEntry => "catalog_entry",
    }
}

fn format_location_source_kind(source_kind: tsw_scenario_scanner::LocationSourceKind) -> &'static str {
    match source_kind {
        tsw_scenario_scanner::LocationSourceKind::FrontendSpawnPoint => "frontend_spawn_point",
        tsw_scenario_scanner::LocationSourceKind::RouteDefinition => "route_definition",
        tsw_scenario_scanner::LocationSourceKind::Timetable => "timetable",
        tsw_scenario_scanner::LocationSourceKind::Scenario => "scenario",
        tsw_scenario_scanner::LocationSourceKind::Training => "training",
        tsw_scenario_scanner::LocationSourceKind::Localization => "localization",
        tsw_scenario_scanner::LocationSourceKind::Unknown => "unknown",
    }
}

fn format_location_confidence(location: &LocationMatch) -> &'static str {
    match location.confidence {
        tsw_scenario_scanner::LocationConfidence::High => "high",
        tsw_scenario_scanner::LocationConfidence::Medium => "medium",
        tsw_scenario_scanner::LocationConfidence::Low => "low",
    }
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




















