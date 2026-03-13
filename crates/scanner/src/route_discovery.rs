use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::normalize_for_match;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteOverview {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stat_tracking_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_point: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_point: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level_asset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_map_widget: Option<String>,
}

impl RouteOverview {
    fn is_empty(&self) -> bool {
        self.display_name.is_none()
            && self.stat_tracking_name.is_none()
            && self.start_point.is_none()
            && self.end_point.is_none()
            && self.country.is_none()
            && self.level_asset.is_none()
            && self.route_map_widget.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrontendSpawnPoint {
    pub tag: String,
    pub display_name: String,
    #[serde(default)]
    pub available_in_frontend: bool,
    #[serde(default)]
    pub available_in_fast_travel: bool,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteDiscovery {
    pub schema_version: u32,
    pub route_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_overview: Option<RouteOverview>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frontend_spawn_points: Vec<FrontendSpawnPoint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route_titles: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route_definitions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<DiscoveredLocation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stock: Vec<DiscoveredStock>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    pub summary: RouteDiscoverySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteDiscoverySummary {
    pub source_count: usize,
    pub route_title_count: usize,
    pub route_definition_count: usize,
    #[serde(default)]
    pub frontend_spawn_point_count: usize,
    pub location_count: usize,
    pub stock_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredLocation {
    pub id: String,
    pub display_name: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_ref: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredStock {
    pub id: String,
    pub source: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StockCatalog {
    pub schema_version: u32,
    pub route_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_overview: Option<RouteOverview>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stock: Vec<StockCatalogEntry>,
    pub summary: StockCatalogSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StockCatalogEntry {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    pub source: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StockCatalogSummary {
    pub stock_count: usize,
    pub plugin_count: usize,
    pub source_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FormationCatalog {
    pub schema_version: u32,
    pub route_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_overview: Option<RouteOverview>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formations: Vec<FormationCatalogEntry>,
    pub summary: FormationCatalogSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FormationCatalogEntry {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    pub source: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<FormationCatalogMember>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drivable: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FormationCatalogMember {
    pub index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vehicle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_asset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_units: Option<u32>,
    #[serde(default)]
    pub flipped: bool,
    #[serde(default)]
    pub cargo_loaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FormationCatalogSummary {
    pub formation_count: usize,
    pub plugin_count: usize,
    pub source_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StockMatchConstraints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FormationMatchConstraints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlattenedFormationVehicle {
    pub vehicle_id: String,
    #[serde(default)]
    pub flipped: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_asset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_units: Option<u32>,
}

#[derive(Debug, Error)]
pub enum RouteDiscoveryError {
    #[error("at least one route discovery source must be provided")]
    MissingRouteDiscoverySource,
    #[error("route directory '{path}' does not exist or is not a directory")]
    InvalidRouteDir { path: PathBuf },
    #[error("exports directory '{path}' does not exist or is not a directory")]
    InvalidExportsDir { path: PathBuf },
}

pub fn discover_route(
    route_id: &str,
    route_dir: Option<&Path>,
    exports_dir: Option<&Path>,
) -> Result<RouteDiscovery> {
    if route_dir.is_none() && exports_dir.is_none() {
        return Err(RouteDiscoveryError::MissingRouteDiscoverySource)
            .context("route discovery requires at least one source");
    }

    let mut builder = RouteDiscoveryBuilder::new(route_id);

    if let Some(route_dir) = route_dir {
        if !route_dir.is_dir() {
            return Err(RouteDiscoveryError::InvalidRouteDir {
                path: route_dir.to_path_buf(),
            })
            .context("invalid route directory");
        }
        scan_route_dir(route_dir, &mut builder)?;
    }

    if let Some(exports_dir) = exports_dir {
        if !exports_dir.is_dir() {
            return Err(RouteDiscoveryError::InvalidExportsDir {
                path: exports_dir.to_path_buf(),
            })
            .context("invalid exports directory");
        }
        scan_exports_dir(exports_dir, &mut builder)?;
    }

    Ok(builder.build())
}

pub fn save_route_discovery(discovery: &RouteDiscovery, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let json = serde_json::to_string_pretty(discovery)
        .context("failed to serialize route discovery")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory '{}'", parent.display()))?;
    }
    fs::write(path, json)
        .with_context(|| format!("failed to write route discovery '{}'", path.display()))
}

pub fn load_route_discovery(path: impl AsRef<Path>) -> Result<RouteDiscovery> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read route discovery '{}'", path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse route discovery '{}'", path.display()))
}

pub fn build_stock_catalog(discovery: &RouteDiscovery) -> StockCatalog {
    let mut stock = discovery
        .stock
        .iter()
        .map(|entry| StockCatalogEntry {
            id: entry.id.clone(),
            plugin: extract_plugin_name(&entry.source),
            source: entry.source.clone(),
            evidence: entry.evidence.clone(),
        })
        .collect::<Vec<_>>();

    stock.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.plugin.cmp(&right.plugin))
            .then_with(|| left.source.cmp(&right.source))
    });

    let plugin_count = stock
        .iter()
        .filter_map(|entry| entry.plugin.as_deref())
        .map(normalize_for_match)
        .filter(|value| !value.is_empty())
        .collect::<HashSet<_>>()
        .len();
    let source_count = stock
        .iter()
        .map(|entry| normalize_for_match(&entry.source))
        .filter(|value| !value.is_empty())
        .collect::<HashSet<_>>()
        .len();

    StockCatalog {
        schema_version: 1,
        route_id: discovery.route_id.clone(),
        route_overview: discovery.route_overview.clone(),
        stock,
        summary: StockCatalogSummary {
            stock_count: discovery.stock.len(),
            plugin_count,
            source_count,
        },
    }
}

pub fn save_stock_catalog(catalog: &StockCatalog, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let json = serde_json::to_string_pretty(catalog)
        .context("failed to serialize stock catalog")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory '{}'", parent.display()))?;
    }
    fs::write(path, json)
        .with_context(|| format!("failed to write stock catalog '{}'", path.display()))
}

pub fn load_stock_catalog(path: impl AsRef<Path>) -> Result<StockCatalog> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read stock catalog '{}'", path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse stock catalog '{}'", path.display()))
}

pub fn resolve_stock_matches(catalog: &StockCatalog, query: &str) -> Vec<StockCatalogEntry> {
    let normalized_query = normalize_for_match(query);
    if normalized_query.is_empty() {
        return Vec::new();
    }

    let mut matches = catalog
        .stock
        .iter()
        .filter_map(|entry| {
            let score = stock_query_score(entry, query, &normalized_query)?;
            Some((score, entry.clone()))
        })
        .collect::<Vec<_>>();

    matches.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.id.cmp(&right.1.id))
            .then_with(|| left.1.plugin.cmp(&right.1.plugin))
            .then_with(|| left.1.source.cmp(&right.1.source))
    });

    matches.into_iter().map(|(_, entry)| entry).collect()
}

pub fn resolve_stock_reference_matches(
    catalog: &StockCatalog,
    query: &str,
    constraints: &StockMatchConstraints,
) -> Vec<StockCatalogEntry> {
    resolve_stock_matches(catalog, query)
        .into_iter()
        .filter(|entry| stock_entry_matches_constraints(entry, constraints))
        .collect()
}

fn stock_query_score(entry: &StockCatalogEntry, query: &str, normalized_query: &str) -> Option<u8> {
    let normalized_id = normalize_for_match(&entry.id);
    let normalized_plugin = entry.plugin.as_deref().map(normalize_for_match);
    let normalized_source = normalize_for_match(&entry.source);
    let normalized_evidence = normalize_for_match(&entry.evidence);
    let trimmed_query = query.trim();

    if normalized_id == normalized_query {
        return Some(5);
    }

    if entry
        .plugin
        .as_deref()
        .map(|plugin| plugin.eq_ignore_ascii_case(trimmed_query))
        .unwrap_or(false)
    {
        return Some(4);
    }

    if normalized_plugin.as_deref() == Some(normalized_query) {
        return Some(4);
    }

    if normalized_id.contains(normalized_query) {
        return Some(3);
    }

    if normalized_plugin
        .as_deref()
        .map(|plugin| plugin.contains(normalized_query))
        .unwrap_or(false)
    {
        return Some(2);
    }

    if normalized_source.contains(normalized_query) || normalized_evidence.contains(normalized_query) {
        return Some(1);
    }

    None
}

fn stock_entry_matches_constraints(
    entry: &StockCatalogEntry,
    constraints: &StockMatchConstraints,
) -> bool {
    matches_optional_text(entry.plugin.as_deref(), constraints.plugin.as_deref())
        && matches_optional_text(Some(entry.source.as_str()), constraints.source.as_deref())
}

fn formation_entry_matches_constraints(
    entry: &FormationCatalogEntry,
    constraints: &FormationMatchConstraints,
) -> bool {
    matches_optional_text(entry.plugin.as_deref(), constraints.plugin.as_deref())
        && matches_optional_text(Some(entry.source.as_str()), constraints.source.as_deref())
}

fn matches_optional_text(actual: Option<&str>, expected: Option<&str>) -> bool {
    let Some(expected) = expected else {
        return true;
    };

    let expected = expected.trim();
    if expected.is_empty() {
        return true;
    }

    actual
        .map(|value| value.eq_ignore_ascii_case(expected))
        .unwrap_or(false)
}

fn extract_plugin_name(source: &str) -> Option<String> {
    let normalized = source.replace('\\', "/");
    let segments = normalized.split('/').collect::<Vec<_>>();

    for window in segments.windows(3) {
        if window[0] == "Plugins" && window[1] == "DLC" {
            let plugin = window[2].trim();
            if !plugin.is_empty() {
                return Some(plugin.to_string());
            }
        }
    }

    None
}

pub fn discover_formation_catalog(
    route_id: &str,
    route_overview: Option<&RouteOverview>,
    exports_dir: Option<&Path>,
) -> Result<FormationCatalog> {
    let Some(exports_dir) = exports_dir else {
        return Ok(FormationCatalogBuilder::new(route_id, route_overview.cloned()).build());
    };

    let mut builder = FormationCatalogBuilder::new(route_id, route_overview.cloned());
    scan_formation_exports_dir(exports_dir, &mut builder)?;
    Ok(builder.build())
}

pub fn save_formation_catalog(catalog: &FormationCatalog, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let json = serde_json::to_string_pretty(catalog)
        .context("failed to serialize formation catalog")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory '{}'", parent.display()))?;
    }
    fs::write(path, json)
        .with_context(|| format!("failed to write formation catalog '{}'", path.display()))
}

pub fn load_formation_catalog(path: impl AsRef<Path>) -> Result<FormationCatalog> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read formation catalog '{}'", path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse formation catalog '{}'", path.display()))
}

pub fn resolve_formation_matches(catalog: &FormationCatalog, query: &str) -> Vec<FormationCatalogEntry> {
    let normalized_query = normalize_for_match(query);
    if normalized_query.is_empty() {
        return Vec::new();
    }

    let mut matches = catalog
        .formations
        .iter()
        .filter_map(|entry| {
            let score = formation_query_score(entry, query, &normalized_query)?;
            Some((score, entry.clone()))
        })
        .collect::<Vec<_>>();

    matches.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.id.cmp(&right.1.id))
            .then_with(|| left.1.plugin.cmp(&right.1.plugin))
            .then_with(|| left.1.source.cmp(&right.1.source))
    });

    matches.into_iter().map(|(_, entry)| entry).collect()
}

pub fn resolve_formation_reference_matches(
    catalog: &FormationCatalog,
    query: &str,
    constraints: &FormationMatchConstraints,
) -> Vec<FormationCatalogEntry> {
    resolve_formation_matches(catalog, query)
        .into_iter()
        .filter(|entry| formation_entry_matches_constraints(entry, constraints))
        .collect()
}

pub fn resolve_flattened_formation_entries(
    catalog: &FormationCatalog,
    formation_id: &str,
) -> Result<Vec<FlattenedFormationVehicle>> {
    let mut stack = Vec::new();
    flatten_formation_entries(catalog, formation_id, &mut stack)
}

fn formation_query_score(
    entry: &FormationCatalogEntry,
    query: &str,
    normalized_query: &str,
) -> Option<u8> {
    let normalized_id = normalize_for_match(&entry.id);
    let normalized_plugin = entry.plugin.as_deref().map(normalize_for_match);
    let normalized_source = normalize_for_match(&entry.source);
    let trimmed_query = query.trim();

    if normalized_id == normalized_query {
        return Some(5);
    }

    if entry
        .plugin
        .as_deref()
        .map(|plugin| plugin.eq_ignore_ascii_case(trimmed_query))
        .unwrap_or(false)
    {
        return Some(4);
    }

    if normalized_plugin.as_deref() == Some(normalized_query) {
        return Some(4);
    }

    if normalized_id.contains(normalized_query) {
        return Some(3);
    }

    if normalized_plugin
        .as_deref()
        .map(|plugin| plugin.contains(normalized_query))
        .unwrap_or(false)
    {
        return Some(2);
    }

    if normalized_source.contains(normalized_query) {
        return Some(1);
    }

    None
}

fn flatten_formation_entries(
    catalog: &FormationCatalog,
    formation_id: &str,
    stack: &mut Vec<String>,
) -> Result<Vec<FlattenedFormationVehicle>> {
    if stack.iter().any(|candidate| candidate.eq_ignore_ascii_case(formation_id)) {
        let mut cycle = stack.clone();
        cycle.push(formation_id.to_string());
        bail!("formation catalog cycle detected: {}", cycle.join(" -> "));
    }

    let formation = catalog
        .formations
        .iter()
        .find(|entry| entry.id.eq_ignore_ascii_case(formation_id))
        .with_context(|| format!("formation '{}' is not present in the formation catalog", formation_id))?;

    stack.push(formation.id.clone());
    let mut flattened = Vec::new();

    for entry in &formation.entries {
        if let Some(vehicle_id) = &entry.vehicle_id {
            flattened.push(FlattenedFormationVehicle {
                vehicle_id: vehicle_id.clone(),
                flipped: entry.flipped,
                cargo_asset: entry.cargo_asset.clone(),
                cargo_units: entry.cargo_units,
            });
            continue;
        }

        if let Some(nested_formation_id) = &entry.formation_id {
            let nested = flatten_formation_entries(catalog, nested_formation_id, stack)?;
            for mut vehicle in nested {
                vehicle.flipped ^= entry.flipped;
                if entry.cargo_asset.is_some() {
                    vehicle.cargo_asset = entry.cargo_asset.clone();
                }
                if entry.cargo_units.is_some() {
                    vehicle.cargo_units = entry.cargo_units;
                }
                flattened.push(vehicle);
            }
        }
    }

    stack.pop();
    Ok(flattened)
}

fn scan_formation_exports_dir(
    exports_dir: &Path,
    builder: &mut FormationCatalogBuilder,
) -> Result<()> {
    let mut json_files = Vec::new();
    collect_files_with_extension(exports_dir, exports_dir, "json", &mut json_files)?;

    for json_file in json_files {
        let source = json_file
            .strip_prefix(exports_dir)
            .unwrap_or(&json_file)
            .to_string_lossy()
            .replace('\\', "/");
        if !should_scan_export_source(&source) {
            continue;
        }

        let contents = fs::read_to_string(&json_file)
            .with_context(|| format!("failed to read formation exports JSON '{}'", json_file.display()))?;
        let value: Value = match serde_json::from_str(&contents) {
            Ok(value) => value,
            Err(_) => continue,
        };

        collect_formations_from_value(&value, &source, builder);
    }

    Ok(())
}

fn collect_formations_from_value(
    value: &Value,
    source: &str,
    builder: &mut FormationCatalogBuilder,
) {
    match value {
        Value::Object(map) => {
            if let Some(entry) = parse_formation_entry_object(map, source) {
                builder.add_formation(entry);
            }

            for child in map.values() {
                collect_formations_from_value(child, source, builder);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_formations_from_value(child, source, builder);
            }
        }
        _ => {}
    }
}

fn parse_formation_entry_object(
    map: &Map<String, Value>,
    source: &str,
) -> Option<FormationCatalogEntry> {
    let type_name = map.get("Type").and_then(Value::as_str).unwrap_or_default();
    let class_name = map.get("Class").and_then(Value::as_str).unwrap_or_default();
    if type_name != "TrainFormation" && !class_name.contains("TrainFormation") {
        return None;
    }

    let id = map.get("Name").and_then(Value::as_str)?.trim();
    if id.is_empty() {
        return None;
    }

    let properties = map.get("Properties")?.as_object()?;
    let entries = properties
        .get("Formation")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .enumerate()
                .filter_map(|(index, value)| parse_formation_member(index, value))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let drivable = properties.get("bIsDrivable").and_then(Value::as_bool);

    Some(FormationCatalogEntry {
        id: id.to_string(),
        plugin: extract_plugin_name(source),
        source: source.to_string(),
        entries,
        drivable,
    })
}

fn parse_formation_member(index: usize, value: &Value) -> Option<FormationCatalogMember> {
    let map = value.as_object()?;
    let train_entry = map.get("TrainEntry")?;
    let object_name = train_entry
        .as_object()
        .and_then(|object| object.get("ObjectName"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let object_path = train_entry
        .as_object()
        .and_then(|object| object.get("ObjectPath"))
        .and_then(Value::as_str);

    let vehicle_id = extract_quoted_identifier(object_name, "RailVehicleDefinition")
        .or_else(|| object_path.and_then(extract_identifier_from_object_path));
    let formation_id = extract_quoted_identifier(object_name, "TrainFormation")
        .or_else(|| {
            if vehicle_id.is_some() {
                None
            } else {
                object_path.and_then(extract_identifier_from_object_path)
            }
        });

    let cargo_asset = map
        .get("CargoPreLoad")
        .and_then(Value::as_object)
        .and_then(|preload| preload.get("Cargo"))
        .and_then(extract_asset_path_name)
        .filter(|value| value != "None");
    let cargo_units = map
        .get("CargoPreLoad")
        .and_then(Value::as_object)
        .and_then(|preload| preload.get("NumberOfUnits"))
        .and_then(Value::as_f64)
        .and_then(|value| if value > 0.0 { Some(value.round() as u32) } else { None });
    let flipped = map.get("bFlipped").and_then(Value::as_bool).unwrap_or(false);
    let cargo_loaded = map
        .get("bCargoLoaded")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if vehicle_id.is_none() && formation_id.is_none() {
        return None;
    }

    Some(FormationCatalogMember {
        index,
        vehicle_id,
        formation_id,
        cargo_asset,
        cargo_units,
        flipped,
        cargo_loaded,
    })
}

fn extract_identifier_from_object_path(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "None" {
        return None;
    }

    let object_name = trimmed
        .rsplit('/')
        .next()
        .unwrap_or(trimmed)
        .split('.')
        .next()
        .unwrap_or(trimmed)
        .trim();
    if object_name.is_empty() || object_name == "0" {
        return None;
    }

    Some(object_name.to_string())
}

#[derive(Debug)]
struct FormationCatalogBuilder {
    route_id: String,
    route_overview: Option<RouteOverview>,
    formations: Vec<FormationCatalogEntry>,
    seen_formations: HashSet<String>,
    seen_sources: HashSet<String>,
}

impl FormationCatalogBuilder {
    fn new(route_id: &str, route_overview: Option<RouteOverview>) -> Self {
        Self {
            route_id: route_id.to_string(),
            route_overview,
            formations: Vec::new(),
            seen_formations: HashSet::new(),
            seen_sources: HashSet::new(),
        }
    }

    fn add_formation(&mut self, entry: FormationCatalogEntry) {
        let key = format!(
            "{}|{}",
            normalize_for_match(&entry.id),
            normalize_for_match(&entry.source)
        );
        if key.is_empty() || !self.seen_formations.insert(key) {
            return;
        }

        let normalized_source = normalize_for_match(&entry.source);
        if !normalized_source.is_empty() {
            self.seen_sources.insert(normalized_source);
        }

        self.formations.push(entry);
    }

    fn build(mut self) -> FormationCatalog {
        self.formations.sort_by(|left, right| {
            left.id
                .cmp(&right.id)
                .then_with(|| left.plugin.cmp(&right.plugin))
                .then_with(|| left.source.cmp(&right.source))
        });

        let plugin_count = self
            .formations
            .iter()
            .filter_map(|entry| entry.plugin.as_deref())
            .map(normalize_for_match)
            .filter(|value| !value.is_empty())
            .collect::<HashSet<_>>()
            .len();

        FormationCatalog {
            schema_version: 1,
            route_id: self.route_id,
            route_overview: self.route_overview,
            summary: FormationCatalogSummary {
                formation_count: self.formations.len(),
                plugin_count,
                source_count: self.seen_sources.len(),
            },
            formations: self.formations,
        }
    }
}

fn scan_route_dir(route_dir: &Path, builder: &mut RouteDiscoveryBuilder) -> Result<()> {
    let mut locres_files = Vec::new();
    collect_files_with_extension(route_dir, route_dir, "locres", &mut locres_files)?;

    for locres_file in locres_files {
        let bytes = fs::read(&locres_file)
            .with_context(|| format!("failed to read route localization '{}'", locres_file.display()))?;
        let source = locres_file
            .strip_prefix(route_dir)
            .unwrap_or(&locres_file)
            .to_string_lossy()
            .replace('\\', "/");
        builder.add_source(&source);

        for candidate in extract_ascii_strings(&bytes, 6) {
            if let Some(kind) = detect_location_kind(&candidate) {
                builder.add_location(&candidate, None, kind, &source);
            }
            if looks_like_route_title(&candidate) {
                builder.add_route_title(&candidate);
            }
        }
    }

    Ok(())
}

fn scan_exports_dir(exports_dir: &Path, builder: &mut RouteDiscoveryBuilder) -> Result<()> {
    let mut json_files = Vec::new();
    collect_files_with_extension(exports_dir, exports_dir, "json", &mut json_files)?;

    for json_file in json_files {
        let source = json_file
            .strip_prefix(exports_dir)
            .unwrap_or(&json_file)
            .to_string_lossy()
            .replace('\\', "/");
        if !should_scan_export_source(&source) {
            continue;
        }

        let contents = fs::read_to_string(&json_file)
            .with_context(|| format!("failed to read exports JSON '{}'", json_file.display()))?;
        let value: Value = match serde_json::from_str(&contents) {
            Ok(value) => value,
            Err(_) => continue,
        };
        builder.add_source(&source);
        visit_json_value(&value, &source, builder);
    }

    Ok(())
}
fn visit_json_value(value: &Value, source: &str, builder: &mut RouteDiscoveryBuilder) {
    match value {
        Value::Object(map) => {
            try_collect_route_definition_object(map, source, builder);

            if let Some(name) = map.get("Name").and_then(Value::as_str) {
                let internal_ref = map
                    .get("Location")
                    .and_then(extract_ribbon_reference)
                    .or_else(|| {
                        map.get("RibbonReference")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    });
                if internal_ref.is_some() || detect_location_kind(name).is_some() {
                    builder.add_location(
                        name,
                        internal_ref.as_deref(),
                        detect_location_kind(name).unwrap_or("location"),
                        source,
                    );
                }
            }

            for (key, child) in map {
                if let Some(string_value) = child.as_str() {
                    collect_string_evidence(builder, Some(key.as_str()), string_value, source);
                }
                visit_json_value(child, source, builder);
            }
        }
        Value::Array(values) => {
            for child in values {
                visit_json_value(child, source, builder);
            }
        }
        Value::String(string_value) => {
            collect_string_evidence(builder, None, string_value, source);
        }
        _ => {}
    }
}

fn try_collect_route_definition_object(
    map: &Map<String, Value>,
    source: &str,
    builder: &mut RouteDiscoveryBuilder,
) {
    let Some(type_name) = map.get("Type").and_then(Value::as_str) else {
        return;
    };
    if type_name != "RouteDefinition" {
        return;
    }

    let Some(properties) = map.get("Properties").and_then(Value::as_object) else {
        return;
    };

    if let Some(display_name) = properties.get("DisplayName").and_then(extract_localized_string) {
        builder.set_route_display_name(&display_name);
        builder.add_route_title(&display_name);
    }

    if let Some(stat_tracking_name) = properties.get("StatTrackingName").and_then(Value::as_str) {
        builder.set_route_stat_tracking_name(stat_tracking_name);
    }

    if let Some(level_asset) = properties.get("Level").and_then(extract_asset_path_name) {
        builder.set_route_level_asset(&level_asset);
    }

    if let Some(route_map_widget) = properties.get("RouteMapWidget").and_then(extract_asset_path_name) {
        builder.set_route_map_widget(&route_map_widget);
    }

    if let Some(route_details) = properties.get("RouteDetails").and_then(Value::as_object) {
        if let Some(start_point) = route_details.get("StartPoint").and_then(extract_localized_string) {
            builder.set_route_start_point(&start_point);
        }
        if let Some(end_point) = route_details.get("EndPoint").and_then(extract_localized_string) {
            builder.set_route_end_point(&end_point);
        }
        if let Some(country) = route_details.get("Country").and_then(extract_localized_string) {
            builder.set_route_country(&country);
        }

        if let Some(spawn_points) = route_details.get("SpawnPoints").and_then(Value::as_array) {
            for spawn_point in spawn_points {
                let Some(spawn_point_map) = spawn_point.as_object() else {
                    continue;
                };
                let Some(tag) = spawn_point_map.get("PointTag").and_then(Value::as_str) else {
                    continue;
                };
                let display_name = spawn_point_map
                    .get("PointName")
                    .and_then(extract_localized_string)
                    .unwrap_or_else(|| tag.to_string());
                let available_in_frontend = spawn_point_map
                    .get("bAvailableInFrontEnd")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let available_in_fast_travel = spawn_point_map
                    .get("bAvailableInFastTravel")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                builder.add_frontend_spawn_point(
                    tag,
                    &display_name,
                    available_in_frontend,
                    available_in_fast_travel,
                    source,
                );
                builder.add_location(&display_name, None, "station", source);
                builder.add_location(tag, None, "spawn_tag", source);
            }
        }
    }
}

fn should_scan_export_source(source: &str) -> bool {
    let normalized = source.replace('\\', "/").to_ascii_lowercase();

    if normalized.ends_with("routedefinition.json")
        || normalized.ends_with("_definition.json")
        || normalized.ends_with("_timetable.json")
        || normalized.ends_with("timetable.json")
        || normalized.ends_with("_objectives.json")
    {
        return true;
    }
    if normalized.contains("/formations/") {
        return true;
    }
    if normalized.contains("/data/rvd/") || normalized.contains("/data/railvehicledefinition/") {
        return true;
    }
    if normalized.contains("/content/timetable/datatracks/") {
        return false;
    }
    if normalized == "route_definition" || normalized.contains("/content/routedefinition/") {
        return true;
    }
    if normalized == "timetable" || normalized.contains("/content/timetable/") {
        return true;
    }
    if normalized.contains("/content/scenarios/") || normalized.contains("/content/training/") {
        return normalized.contains("/formations/")
            || normalized.ends_with("_definition.json")
            || normalized.ends_with("_timetable.json")
        || normalized.ends_with("timetable.json")
            || normalized.ends_with("_objectives.json");
    }

    false
}

fn collect_string_evidence(
    builder: &mut RouteDiscoveryBuilder,
    key: Option<&str>,
    value: &str,
    source: &str,
) {
    let trimmed = collapse_whitespace(value);
    if trimmed.is_empty() {
        return;
    }

    if let Some(stock_id) = extract_quoted_identifier(&trimmed, "RailVehicleDefinition") {
        builder.add_stock(&stock_id, source, &trimmed);
    }

    if let Some(route_definition) = extract_quoted_identifier(&trimmed, "RouteDefinition") {
        builder.add_route_definition(&route_definition);
    }

    if matches!(key, Some("StartLocationTag" | "EndLocationTag")) {
        builder.add_location(
            &trimmed,
            None,
            detect_location_kind(&trimmed).unwrap_or("tag"),
            source,
        );
    }

    if looks_like_route_title(&trimmed) {
        builder.add_route_title(&trimmed);
    }
}

fn extract_localized_string(value: &Value) -> Option<String> {
    let object = value.as_object()?;
    object
        .get("LocalizedString")
        .and_then(Value::as_str)
        .or_else(|| object.get("SourceString").and_then(Value::as_str))
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty())
}

fn extract_asset_path_name(value: &Value) -> Option<String> {
    value
        .as_object()?
        .get("AssetPathName")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "None")
        .map(str::to_string)
}

fn extract_ribbon_reference(value: &Value) -> Option<String> {
    value
        .as_object()
        .and_then(|map| map.get("RibbonReference"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "00000000-00000000-00000000-00000000")
        .map(str::to_string)
}

fn collect_files_with_extension(
    root: &Path,
    current_dir: &Path,
    extension: &str,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(current_dir)
        .with_context(|| format!("failed to read directory '{}'", current_dir.display()))?
    {
        let entry = entry.with_context(|| {
            format!("failed to enumerate entries in '{}'", current_dir.display())
        })?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            collect_files_with_extension(root, &entry_path, extension, files)?;
            continue;
        }

        let Some(file_extension) = entry_path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if file_extension.eq_ignore_ascii_case(extension) {
            files.push(entry_path);
        }
    }

    files.sort_by(|left, right| {
        left.strip_prefix(root)
            .unwrap_or(left)
            .cmp(right.strip_prefix(root).unwrap_or(right))
    });
    Ok(())
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_ascii_strings(bytes: &[u8], minimum_length: usize) -> Vec<String> {
    let mut strings = Vec::new();
    let mut current = Vec::new();

    for byte in bytes {
        if (32..=126).contains(byte) {
            current.push(*byte);
            continue;
        }

        flush_ascii_string(&mut current, minimum_length, &mut strings);
    }

    flush_ascii_string(&mut current, minimum_length, &mut strings);
    strings
}

fn flush_ascii_string(current: &mut Vec<u8>, minimum_length: usize, strings: &mut Vec<String>) {
    if current.len() < minimum_length {
        current.clear();
        return;
    }

    let raw = String::from_utf8_lossy(current).to_string();
    current.clear();

    let candidate = collapse_whitespace(&raw);
    if candidate.len() < minimum_length
        || candidate.contains('/')
        || candidate.contains('\\')
        || candidate.len() > 120
    {
        return;
    }

    strings.push(candidate);
}

fn looks_like_route_title(value: &str) -> bool {
    value.contains(" - ")
        && !value.contains('/')
        && !value.contains('\\')
        && value.len() >= 10
        && value.len() <= 100
}

fn detect_location_kind(value: &str) -> Option<&'static str> {
    let normalized = format!(" {} ", value.to_ascii_lowercase());
    if normalized.contains(" platform ") || normalized.contains(" pl ") {
        return Some("platform");
    }
    if normalized.contains(" siding ") {
        return Some("siding");
    }
    if normalized.contains(" track ") {
        return Some("track");
    }
    if normalized.contains(" yard ") || normalized.contains(" depot ") {
        return Some("yard");
    }
    if normalized.contains(" portal") || normalized.contains(" portal ") {
        return Some("portal");
    }
    None
}

fn extract_quoted_identifier(value: &str, class_name: &str) -> Option<String> {
    let needle = format!("{class_name}'");
    let start = value.find(&needle)? + needle.len();
    let tail = &value[start..];
    let end = tail.find('\'')?;
    let identifier = tail[..end].trim();
    if identifier.is_empty() {
        None
    } else {
        Some(identifier.to_string())
    }
}

fn slugify_identifier(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_separator = false;

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            slug.push('_');
            previous_was_separator = true;
        }
    }

    slug.trim_matches('_').to_string()
}
#[derive(Debug)]
struct RouteDiscoveryBuilder {
    route_id: String,
    route_overview: RouteOverview,
    frontend_spawn_points: Vec<FrontendSpawnPoint>,
    route_titles: Vec<String>,
    route_definitions: Vec<String>,
    locations: Vec<DiscoveredLocation>,
    stock: Vec<DiscoveredStock>,
    sources: Vec<String>,
    seen_frontend_spawn_points: HashSet<String>,
    seen_route_titles: HashSet<String>,
    seen_route_definitions: HashSet<String>,
    seen_locations: HashSet<String>,
    seen_stock: HashSet<String>,
    seen_sources: HashSet<String>,
}

impl RouteDiscoveryBuilder {
    fn new(route_id: &str) -> Self {
        Self {
            route_id: route_id.to_string(),
            route_overview: RouteOverview::default(),
            frontend_spawn_points: Vec::new(),
            route_titles: Vec::new(),
            route_definitions: Vec::new(),
            locations: Vec::new(),
            stock: Vec::new(),
            sources: Vec::new(),
            seen_frontend_spawn_points: HashSet::new(),
            seen_route_titles: HashSet::new(),
            seen_route_definitions: HashSet::new(),
            seen_locations: HashSet::new(),
            seen_stock: HashSet::new(),
            seen_sources: HashSet::new(),
        }
    }

    fn add_source(&mut self, source: &str) {
        let normalized = normalize_for_match(source);
        if normalized.is_empty() || !self.seen_sources.insert(normalized) {
            return;
        }
        self.sources.push(source.to_string());
    }

    fn set_route_display_name(&mut self, value: &str) {
        set_optional_string(&mut self.route_overview.display_name, value);
    }

    fn set_route_stat_tracking_name(&mut self, value: &str) {
        set_optional_string(&mut self.route_overview.stat_tracking_name, value);
    }

    fn set_route_start_point(&mut self, value: &str) {
        set_optional_string(&mut self.route_overview.start_point, value);
    }

    fn set_route_end_point(&mut self, value: &str) {
        set_optional_string(&mut self.route_overview.end_point, value);
    }

    fn set_route_country(&mut self, value: &str) {
        set_optional_string(&mut self.route_overview.country, value);
    }

    fn set_route_level_asset(&mut self, value: &str) {
        set_optional_string(&mut self.route_overview.level_asset, value);
    }

    fn set_route_map_widget(&mut self, value: &str) {
        set_optional_string(&mut self.route_overview.route_map_widget, value);
    }

    fn add_frontend_spawn_point(
        &mut self,
        tag: &str,
        display_name: &str,
        available_in_frontend: bool,
        available_in_fast_travel: bool,
        source: &str,
    ) {
        let normalized = normalize_for_match(tag);
        if normalized.is_empty() || !self.seen_frontend_spawn_points.insert(normalized) {
            return;
        }

        self.frontend_spawn_points.push(FrontendSpawnPoint {
            tag: tag.trim().to_string(),
            display_name: display_name.trim().to_string(),
            available_in_frontend,
            available_in_fast_travel,
            source: source.to_string(),
        });
    }

    fn add_route_title(&mut self, title: &str) {
        let title = collapse_whitespace(title);
        if title.is_empty() {
            return;
        }

        let normalized = normalize_for_match(&title);
        if normalized.is_empty() || !self.seen_route_titles.insert(normalized) {
            return;
        }
        self.route_titles.push(title);
    }

    fn add_route_definition(&mut self, route_definition: &str) {
        let normalized = normalize_for_match(route_definition);
        if normalized.is_empty() || !self.seen_route_definitions.insert(normalized) {
            return;
        }
        self.route_definitions.push(route_definition.to_string());
    }

    fn add_location(&mut self, display_name: &str, internal_ref: Option<&str>, kind: &str, source: &str) {
        let display_name = collapse_whitespace(display_name);
        if display_name.is_empty() {
            return;
        }

        let normalized_display = normalize_for_match(&display_name);
        if normalized_display.is_empty() {
            return;
        }

        let location_key = match internal_ref {
            Some(internal_ref) if !internal_ref.trim().is_empty() => {
                format!("{}|{}", normalized_display, internal_ref.trim().to_ascii_lowercase())
            }
            _ => normalized_display.clone(),
        };
        if !self.seen_locations.insert(location_key) {
            return;
        }

        let id = slugify_identifier(&display_name);
        self.locations.push(DiscoveredLocation {
            id: if id.is_empty() { normalized_display } else { id },
            display_name,
            kind: kind.to_string(),
            internal_ref: internal_ref.map(str::trim).filter(|value| !value.is_empty()).map(str::to_string),
            source: source.to_string(),
        });
    }

    fn add_stock(&mut self, stock_id: &str, source: &str, evidence: &str) {
        let normalized = normalize_for_match(stock_id);
        if normalized.is_empty() || !self.seen_stock.insert(normalized) {
            return;
        }

        self.stock.push(DiscoveredStock {
            id: stock_id.to_string(),
            source: source.to_string(),
            evidence: evidence.to_string(),
        });
    }

    fn build(mut self) -> RouteDiscovery {
        self.frontend_spawn_points
            .sort_by(|left, right| left.display_name.cmp(&right.display_name));
        self.route_titles.sort();
        self.route_definitions.sort();
        self.locations.sort_by(|left, right| {
            left.display_name
                .cmp(&right.display_name)
                .then_with(|| left.internal_ref.cmp(&right.internal_ref))
        });
        self.stock.sort_by(|left, right| left.id.cmp(&right.id));
        self.sources.sort();

        let route_overview = if self.route_overview.is_empty() {
            None
        } else {
            Some(self.route_overview)
        };

        RouteDiscovery {
            schema_version: 2,
            route_id: self.route_id,
            route_overview,
            frontend_spawn_points: self.frontend_spawn_points,
            route_titles: self.route_titles,
            route_definitions: self.route_definitions,
            locations: self.locations,
            stock: self.stock,
            sources: self.sources,
            summary: RouteDiscoverySummary {
                source_count: self.seen_sources.len(),
                route_title_count: self.seen_route_titles.len(),
                route_definition_count: self.seen_route_definitions.len(),
                frontend_spawn_point_count: self.seen_frontend_spawn_points.len(),
                location_count: self.seen_locations.len(),
                stock_count: self.seen_stock.len(),
            },
        }
    }
}

fn set_optional_string(target: &mut Option<String>, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    if target.is_none() {
        *target = Some(trimmed.to_string());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocationCatalog {
    pub schema_version: u32,
    pub route_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_overview: Option<RouteOverview>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stations: Vec<LocationCatalogStation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ungrouped_locations: Vec<LocationCatalogEntry>,
    pub summary: LocationCatalogSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocationCatalogSummary {
    pub station_count: usize,
    pub frontend_spawn_point_count: usize,
    pub grouped_location_count: usize,
    pub ungrouped_location_count: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum LocationSourceKind {
    FrontendSpawnPoint,
    RouteDefinition,
    Timetable,
    Scenario,
    Training,
    Localization,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum LocationConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum LocationAllowedUse {
    PlayerSpawn,
    ServiceStart,
    ServiceEnd,
    ObjectiveLocation,
    ReviewOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocationCatalogStation {
    pub id: String,
    pub display_name: String,
    pub confidence: LocationConfidence,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_uses: Vec<LocationAllowedUse>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frontend_spawn_points: Vec<FrontendSpawnPoint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<LocationCatalogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocationCatalogEntry {
    pub id: String,
    pub display_name: String,
    pub kind: String,
    pub source_kind: LocationSourceKind,
    pub confidence: LocationConfidence,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_uses: Vec<LocationAllowedUse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_ref: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocationLookupUsage {
    PlayerSpawn,
    Service,
    Objective,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocationMatchKind {
    FrontendSpawnPoint,
    Station,
    CatalogEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocationMatch {
    pub catalog_id: String,
    pub display_name: String,
    pub kind: String,
    pub match_kind: LocationMatchKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawn_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<LocationSourceKind>,
    pub confidence: LocationConfidence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocationMatchConstraints {
    pub catalog_id: Option<String>,
    pub spawn_tag: Option<String>,
    pub internal_ref: Option<String>,
}

impl LocationMatchConstraints {
    pub fn is_empty(&self) -> bool {
        self.catalog_id.as_deref().map(str::trim).unwrap_or_default().is_empty()
            && self.spawn_tag.as_deref().map(str::trim).unwrap_or_default().is_empty()
            && self.internal_ref.as_deref().map(str::trim).unwrap_or_default().is_empty()
    }
}

pub fn build_location_catalog(discovery: &RouteDiscovery) -> LocationCatalog {
    let mut stations = Vec::new();

    for frontend_spawn_point in &discovery.frontend_spawn_points {
        add_station_from_frontend_spawn_point(&mut stations, frontend_spawn_point);
    }

    for location in &discovery.locations {
        if location.kind == "station" {
            ensure_station_from_location(&mut stations, location);
        }
    }

    let mut grouped_location_count = 0;
    let mut ungrouped_locations = Vec::new();

    for location in &discovery.locations {
        if location.kind == "spawn_tag" || location.kind == "station" {
            continue;
        }

        let entry = LocationCatalogEntry {
            id: location.id.clone(),
            display_name: location.display_name.clone(),
            kind: location.kind.clone(),
            source_kind: detect_location_source_kind(&location.source),
            confidence: infer_location_confidence(location),
            allowed_uses: infer_location_allowed_uses(location),
            internal_ref: location.internal_ref.clone(),
            source: location.source.clone(),
        };

        if let Some(station_index) = find_best_station_index(&stations, &location.display_name) {
            if !stations[station_index]
                .locations
                .iter()
                .any(|candidate| candidate.display_name == entry.display_name
                    && candidate.internal_ref == entry.internal_ref)
            {
                stations[station_index].locations.push(entry);
                grouped_location_count += 1;
            }
        } else {
            ungrouped_locations.push(entry);
        }
    }

    for station in &mut stations {
        station.tags.sort();
        station.tags.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        station
            .frontend_spawn_points
            .sort_by(|left, right| left.display_name.cmp(&right.display_name));
        station.allowed_uses.sort();
        station.allowed_uses.dedup();
        station.locations.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| left.display_name.cmp(&right.display_name))
                .then_with(|| left.internal_ref.cmp(&right.internal_ref))
        });
    }

    stations.sort_by(|left, right| left.display_name.cmp(&right.display_name));
    ungrouped_locations.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.display_name.cmp(&right.display_name))
            .then_with(|| left.internal_ref.cmp(&right.internal_ref))
    });

    LocationCatalog {
        schema_version: 2,
        route_id: discovery.route_id.clone(),
        route_overview: discovery.route_overview.clone(),
        summary: LocationCatalogSummary {
            station_count: stations.len(),
            frontend_spawn_point_count: discovery.summary.frontend_spawn_point_count,
            grouped_location_count,
            ungrouped_location_count: ungrouped_locations.len(),
        },
        stations,
        ungrouped_locations,
    }
}

pub fn save_location_catalog(catalog: &LocationCatalog, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let json = serde_json::to_string_pretty(catalog)
        .context("failed to serialize location catalog")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory '{}'", parent.display()))?;
    }
    fs::write(path, json)
        .with_context(|| format!("failed to write location catalog '{}'", path.display()))
}

pub fn load_location_catalog(path: impl AsRef<Path>) -> Result<LocationCatalog> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read location catalog '{}'", path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse location catalog '{}'", path.display()))
}

pub fn resolve_location_matches(
    catalog: &LocationCatalog,
    query: &str,
    usage: LocationLookupUsage,
) -> Vec<LocationMatch> {
    resolve_location_reference_matches(
        catalog,
        Some(query),
        usage,
        &LocationMatchConstraints::default(),
    )
}

pub fn resolve_location_reference_matches(
    catalog: &LocationCatalog,
    query: Option<&str>,
    usage: LocationLookupUsage,
    constraints: &LocationMatchConstraints,
) -> Vec<LocationMatch> {
    let trimmed_query = query.and_then(|value| non_empty_trimmed(value).map(str::to_string));
    let normalized_query = trimmed_query
        .as_deref()
        .map(normalize_location_lookup)
        .filter(|value| !value.is_empty());

    if trimmed_query.is_none() && constraints.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();

    for station in &catalog.stations {
        for frontend_spawn_point in &station.frontend_spawn_points {
            let candidate = LocationMatch {
                catalog_id: frontend_spawn_point.tag.clone(),
                display_name: frontend_spawn_point.display_name.clone(),
                kind: "spawn_tag".to_string(),
                match_kind: LocationMatchKind::FrontendSpawnPoint,
                spawn_tag: Some(frontend_spawn_point.tag.clone()),
                internal_ref: None,
                source_kind: Some(LocationSourceKind::RouteDefinition),
                confidence: LocationConfidence::High,
                source: Some(frontend_spawn_point.source.clone()),
            };
            let query_keys = [&frontend_spawn_point.display_name[..], &frontend_spawn_point.tag[..]];
            push_location_match_candidate(
                &mut matches,
                &candidate,
                &query_keys,
                usage,
                trimmed_query.as_deref(),
                normalized_query.as_deref(),
                constraints,
            );
        }

        if station.frontend_spawn_points.is_empty() && station_matches_lookup_usage(station, usage) {
            let mut station_keys = vec![station.display_name.as_str(), station.id.as_str()];
            for tag in &station.tags {
                station_keys.push(tag);
            }

            let candidate = LocationMatch {
                catalog_id: station.id.clone(),
                display_name: station.display_name.clone(),
                kind: "station".to_string(),
                match_kind: LocationMatchKind::Station,
                spawn_tag: station.tags.first().cloned(),
                internal_ref: None,
                source_kind: None,
                confidence: station.confidence,
                source: None,
            };
            push_location_match_candidate(
                &mut matches,
                &candidate,
                &station_keys,
                usage,
                trimmed_query.as_deref(),
                normalized_query.as_deref(),
                constraints,
            );
        }

        for location in &station.locations {
            if !location_entry_matches_lookup_usage(location, usage) {
                continue;
            }

            let candidate = LocationMatch {
                catalog_id: location.id.clone(),
                display_name: location.display_name.clone(),
                kind: location.kind.clone(),
                match_kind: LocationMatchKind::CatalogEntry,
                spawn_tag: None,
                internal_ref: location.internal_ref.clone(),
                source_kind: Some(location.source_kind),
                confidence: location.confidence,
                source: Some(location.source.clone()),
            };
            let mut query_keys = vec![location.display_name.as_str(), location.id.as_str()];
            if let Some(internal_ref) = location.internal_ref.as_deref() {
                query_keys.push(internal_ref);
            }
            push_location_match_candidate(
                &mut matches,
                &candidate,
                &query_keys,
                usage,
                trimmed_query.as_deref(),
                normalized_query.as_deref(),
                constraints,
            );
        }
    }

    for location in &catalog.ungrouped_locations {
        if !location_entry_matches_lookup_usage(location, usage) {
            continue;
        }

        let candidate = LocationMatch {
            catalog_id: location.id.clone(),
            display_name: location.display_name.clone(),
            kind: location.kind.clone(),
            match_kind: LocationMatchKind::CatalogEntry,
            spawn_tag: None,
            internal_ref: location.internal_ref.clone(),
            source_kind: Some(location.source_kind),
            confidence: location.confidence,
            source: Some(location.source.clone()),
        };
        let mut query_keys = vec![location.display_name.as_str(), location.id.as_str()];
        if let Some(internal_ref) = location.internal_ref.as_deref() {
            query_keys.push(internal_ref);
        }
        push_location_match_candidate(
            &mut matches,
            &candidate,
            &query_keys,
            usage,
            trimmed_query.as_deref(),
            normalized_query.as_deref(),
            constraints,
        );
    }

    sort_and_deduplicate_location_matches(matches, usage)
}

fn push_location_match_candidate(
    matches: &mut Vec<(u8, u8, LocationMatch)>,
    candidate: &LocationMatch,
    query_keys: &[&str],
    usage: LocationLookupUsage,
    query: Option<&str>,
    normalized_query: Option<&str>,
    constraints: &LocationMatchConstraints,
) {
    if !location_matches_constraints(candidate, constraints) {
        return;
    }

    let score = match (query, normalized_query) {
        (Some(query), Some(normalized_query)) => {
            let Some(score) = location_query_score(query, normalized_query, query_keys) else {
                return;
            };
            score
        }
        _ => 0,
    };

    matches.push((
        score,
        location_lookup_priority(usage, candidate.match_kind),
        candidate.clone(),
    ));
}

fn location_matches_constraints(
    candidate: &LocationMatch,
    constraints: &LocationMatchConstraints,
) -> bool {
    constraint_matches(
        constraints.catalog_id.as_deref(),
        Some(candidate.catalog_id.as_str()),
    ) && constraint_matches(
        constraints.spawn_tag.as_deref(),
        candidate.spawn_tag.as_deref(),
    ) && constraint_matches(
        constraints.internal_ref.as_deref(),
        candidate.internal_ref.as_deref(),
    )
}

fn constraint_matches(expected: Option<&str>, actual: Option<&str>) -> bool {
    let Some(expected) = non_empty_trimmed(expected.unwrap_or_default()) else {
        return true;
    };
    let Some(actual) = actual.and_then(non_empty_trimmed) else {
        return false;
    };

    normalize_location_lookup(expected) == normalize_location_lookup(actual)
}

fn sort_and_deduplicate_location_matches(
    mut matches: Vec<(u8, u8, LocationMatch)>,
    usage: LocationLookupUsage,
) -> Vec<LocationMatch> {
    matches.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| {
                location_match_confidence_rank(right.2.confidence)
                    .cmp(&location_match_confidence_rank(left.2.confidence))
            })
            .then_with(|| {
                location_match_reference_rank(&right.2, usage)
                    .cmp(&location_match_reference_rank(&left.2, usage))
            })
            .then_with(|| left.2.display_name.cmp(&right.2.display_name))
            .then_with(|| left.2.catalog_id.cmp(&right.2.catalog_id))
            .then_with(|| left.2.internal_ref.cmp(&right.2.internal_ref))
    });

    let mut deduped = Vec::new();
    let mut seen = HashSet::new();
    for (_, _, candidate) in matches {
        let signature = format!(
            "{:?}|{}|{}|{}|{}|{}",
            candidate.match_kind,
            candidate.catalog_id,
            candidate.display_name,
            candidate.kind,
            candidate.spawn_tag.as_deref().unwrap_or(""),
            candidate.internal_ref.as_deref().unwrap_or("")
        );
        if seen.insert(signature) {
            deduped.push(candidate);
        }
    }

    deduped
}

fn non_empty_trimmed(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn location_query_score(query: &str, normalized_query: &str, candidates: &[&str]) -> Option<u8> {
    let lower_query = query.trim().to_lowercase();
    if lower_query.is_empty() {
        return None;
    }

    if candidates
        .iter()
        .any(|candidate| candidate.trim().to_lowercase() == lower_query)
    {
        return Some(2);
    }

    if candidates.iter().any(|candidate| normalize_location_lookup(candidate) == normalized_query) {
        return Some(1);
    }

    None
}

fn location_lookup_priority(usage: LocationLookupUsage, match_kind: LocationMatchKind) -> u8 {
    match usage {
        LocationLookupUsage::PlayerSpawn => match match_kind {
            LocationMatchKind::FrontendSpawnPoint => 3,
            LocationMatchKind::Station => 2,
            LocationMatchKind::CatalogEntry => 1,
        },
        LocationLookupUsage::Service | LocationLookupUsage::Objective => match match_kind {
            LocationMatchKind::CatalogEntry => 3,
            LocationMatchKind::FrontendSpawnPoint => 2,
            LocationMatchKind::Station => 1,
        },
    }
}

fn location_match_confidence_rank(confidence: LocationConfidence) -> u8 {
    match confidence {
        LocationConfidence::High => 3,
        LocationConfidence::Medium => 2,
        LocationConfidence::Low => 1,
    }
}

fn location_match_reference_rank(location: &LocationMatch, usage: LocationLookupUsage) -> u8 {
    match usage {
        LocationLookupUsage::PlayerSpawn => {
            if location.spawn_tag.is_some() {
                3
            } else if location.internal_ref.is_some() {
                2
            } else {
                1
            }
        }
        LocationLookupUsage::Service | LocationLookupUsage::Objective => {
            if location.internal_ref.is_some() {
                3
            } else if location.spawn_tag.is_some() {
                2
            } else {
                1
            }
        }
    }
}

fn station_matches_lookup_usage(
    station: &LocationCatalogStation,
    usage: LocationLookupUsage,
) -> bool {
    match usage {
        LocationLookupUsage::PlayerSpawn => station
            .allowed_uses
            .iter()
            .any(|use_case| *use_case == LocationAllowedUse::PlayerSpawn),
        LocationLookupUsage::Service | LocationLookupUsage::Objective => true,
    }
}

fn location_entry_matches_lookup_usage(
    location: &LocationCatalogEntry,
    usage: LocationLookupUsage,
) -> bool {
    match usage {
        LocationLookupUsage::PlayerSpawn => location
            .allowed_uses
            .iter()
            .any(|use_case| *use_case == LocationAllowedUse::PlayerSpawn),
        LocationLookupUsage::Service => location.allowed_uses.iter().any(|use_case| {
            matches!(
                use_case,
                LocationAllowedUse::ServiceStart | LocationAllowedUse::ServiceEnd
            )
        }),
        LocationLookupUsage::Objective => location.allowed_uses.iter().any(|use_case| {
            matches!(
                use_case,
                LocationAllowedUse::ServiceStart
                    | LocationAllowedUse::ServiceEnd
                    | LocationAllowedUse::ObjectiveLocation
            )
        }),
    }
}

fn normalize_location_lookup(value: &str) -> String {
    let mut normalized = String::new();

    for character in value.chars() {
        match character {
            '\u{00E4}' | '\u{00C4}' => normalized.push_str("ae"),
            '\u{00F6}' | '\u{00D6}' => normalized.push_str("oe"),
            '\u{00FC}' | '\u{00DC}' => normalized.push_str("ue"),
            '\u{00DF}' => normalized.push_str("ss"),
            '\u{00E0}' | '\u{00E1}' | '\u{00E2}' | '\u{00E3}' | '\u{00E5}' | '\u{00C0}' | '\u{00C1}' | '\u{00C2}' | '\u{00C3}' | '\u{00C5}' => {
                normalized.push('a')
            }
            '\u{00E8}' | '\u{00E9}' | '\u{00EA}' | '\u{00EB}' | '\u{00C8}' | '\u{00C9}' | '\u{00CA}' | '\u{00CB}' => normalized.push('e'),
            '\u{00EC}' | '\u{00ED}' | '\u{00EE}' | '\u{00EF}' | '\u{00CC}' | '\u{00CD}' | '\u{00CE}' | '\u{00CF}' => normalized.push('i'),
            '\u{00F2}' | '\u{00F3}' | '\u{00F4}' | '\u{00F5}' | '\u{00D2}' | '\u{00D3}' | '\u{00D4}' | '\u{00D5}' => normalized.push('o'),
            '\u{00F9}' | '\u{00FA}' | '\u{00FB}' | '\u{00D9}' | '\u{00DA}' | '\u{00DB}' => normalized.push('u'),
            _ if character.is_ascii_alphanumeric() => {
                normalized.extend(character.to_lowercase());
            }
            _ => {}
        }
    }

    normalized
}

fn add_station_from_frontend_spawn_point(
    stations: &mut Vec<LocationCatalogStation>,
    frontend_spawn_point: &FrontendSpawnPoint,
) {
    let display_name = frontend_spawn_point.display_name.trim();
    let tag = frontend_spawn_point.tag.trim();

    if let Some(index) = find_station_index_by_alias(stations, display_name, tag) {
        let station = &mut stations[index];
        station.confidence = LocationConfidence::High;
        push_unique_copy(&mut station.allowed_uses, LocationAllowedUse::PlayerSpawn);
        push_unique_case_insensitive(&mut station.tags, tag);
        if !station
            .frontend_spawn_points
            .iter()
            .any(|candidate| candidate.tag.eq_ignore_ascii_case(tag))
        {
            station.frontend_spawn_points.push(frontend_spawn_point.clone());
        }
        return;
    }

    stations.push(LocationCatalogStation {
        id: slugify_identifier(display_name),
        display_name: display_name.to_string(),
        confidence: LocationConfidence::High,
        allowed_uses: vec![LocationAllowedUse::PlayerSpawn],
        tags: vec![tag.to_string()],
        frontend_spawn_points: vec![frontend_spawn_point.clone()],
        locations: Vec::new(),
    });
}

fn ensure_station_from_location(stations: &mut Vec<LocationCatalogStation>, location: &DiscoveredLocation) {
    let display_name = location.display_name.trim();
    if display_name.is_empty() {
        return;
    }

    if find_station_index_by_alias(stations, display_name, display_name).is_some() {
        return;
    }

    let source_kind = detect_location_source_kind(&location.source);
    stations.push(LocationCatalogStation {
        id: slugify_identifier(display_name),
        display_name: display_name.to_string(),
        confidence: infer_station_confidence_from_source(source_kind),
        allowed_uses: infer_station_allowed_uses_from_source(source_kind),
        tags: Vec::new(),
        frontend_spawn_points: Vec::new(),
        locations: Vec::new(),
    });
}
fn find_station_index_by_alias(
    stations: &[LocationCatalogStation],
    display_name: &str,
    tag: &str,
) -> Option<usize> {
    stations.iter().position(|station| {
        normalized_identifiers_match(&station.display_name, display_name)
            || station
                .tags
                .iter()
                .any(|candidate| normalized_identifiers_match(candidate, tag))
    })
}

fn find_best_station_index(stations: &[LocationCatalogStation], location_name: &str) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None;

    for (index, station) in stations.iter().enumerate() {
        let mut station_best_score = station_alias_match_score(&station.display_name, location_name);
        for tag in &station.tags {
            station_best_score = station_best_score.max(station_alias_match_score(tag, location_name));
        }

        let Some(score) = station_best_score else {
            continue;
        };

        match best {
            Some((_, best_score)) if best_score >= score => {}
            _ => best = Some((index, score)),
        }
    }

    best.map(|(index, _)| index)
}

fn station_alias_match_score(alias: &str, location_name: &str) -> Option<usize> {
    let normalized_alias = normalize_for_match(alias);
    let normalized_location = normalize_for_match(location_name);

    if normalized_alias.is_empty() || normalized_location.is_empty() {
        return None;
    }
    if normalized_alias == normalized_location {
        return Some(1000 + normalized_alias.len());
    }
    if normalized_location.starts_with(&normalized_alias) {
        return Some(900 + normalized_alias.len());
    }
    if normalized_location.contains(&normalized_alias) && normalized_alias.len() >= 6 {
        return Some(700 + normalized_alias.len());
    }

    let alias_tokens = match_tokens(alias);
    let location_tokens = match_tokens(location_name);
    if alias_tokens.is_empty() || location_tokens.is_empty() {
        return None;
    }
    if !tokens_in_order(&alias_tokens, &location_tokens) {
        return None;
    }

    let mut score = 500 + alias_tokens.len() * 20;
    if location_tokens.starts_with(&alias_tokens) {
        score += 100;
    }
    if alias_tokens.len() >= 2 {
        score += 20;
    }
    Some(score)
}

fn match_tokens(value: &str) -> Vec<String> {
    let mut prepared = String::new();
    let mut previous_was_lower_or_digit = false;

    for character in value.chars() {
        if character.is_ascii_uppercase() && previous_was_lower_or_digit {
            prepared.push(' ');
        }

        if character.is_ascii_alphanumeric() {
            prepared.push(character.to_ascii_lowercase());
        } else {
            prepared.push(' ');
        }

        previous_was_lower_or_digit = character.is_ascii_lowercase() || character.is_ascii_digit();
    }

    prepared
        .split_whitespace()
        .map(|token| {
            token
                .replace("ae", "a")
                .replace("oe", "o")
                .replace("ue", "u")
        })
        .filter(|token| !token.is_empty())
        .collect()
}

fn tokens_in_order(needles: &[String], haystack: &[String]) -> bool {
    let mut haystack_index = 0;

    for needle in needles {
        let mut matched = false;
        while haystack_index < haystack.len() {
            if haystack[haystack_index] == *needle {
                matched = true;
                haystack_index += 1;
                break;
            }
            haystack_index += 1;
        }

        if !matched {
            return false;
        }
    }

    true
}

fn normalized_identifiers_match(left: &str, right: &str) -> bool {
    let left = normalize_for_match(left);
    let right = normalize_for_match(right);
    !left.is_empty() && left == right
}

fn detect_location_source_kind(source: &str) -> LocationSourceKind {
    let normalized = source.replace('\\', "/").to_ascii_lowercase();

    if normalized.ends_with(".locres") || normalized.contains("/localization/") {
        return LocationSourceKind::Localization;
    }
    if normalized == "route_definition" || normalized.contains("/content/routedefinition/") {
        return LocationSourceKind::RouteDefinition;
    }
    if normalized == "scenario" || normalized.contains("/content/scenarios/") {
        return LocationSourceKind::Scenario;
    }
    if normalized == "training" || normalized.contains("/content/training/") {
        return LocationSourceKind::Training;
    }
    if normalized == "timetable" || normalized.contains("/content/timetable/") {
        return LocationSourceKind::Timetable;
    }

    LocationSourceKind::Unknown
}

fn infer_location_confidence(location: &DiscoveredLocation) -> LocationConfidence {
    match detect_location_source_kind(&location.source) {
        LocationSourceKind::RouteDefinition => LocationConfidence::High,
        LocationSourceKind::Timetable if location.internal_ref.is_some() => LocationConfidence::High,
        LocationSourceKind::Scenario | LocationSourceKind::Training
            if location.internal_ref.is_some() =>
        {
            LocationConfidence::Medium
        }
        LocationSourceKind::Timetable
        | LocationSourceKind::Scenario
        | LocationSourceKind::Training => LocationConfidence::Medium,
        LocationSourceKind::Localization | LocationSourceKind::Unknown => LocationConfidence::Low,
        LocationSourceKind::FrontendSpawnPoint => LocationConfidence::High,
    }
}

fn infer_location_allowed_uses(location: &DiscoveredLocation) -> Vec<LocationAllowedUse> {
    match location.kind.as_str() {
        "platform" | "siding" | "track" | "yard" => vec![
            LocationAllowedUse::ServiceStart,
            LocationAllowedUse::ServiceEnd,
            LocationAllowedUse::ObjectiveLocation,
        ],
        "portal" => vec![
            LocationAllowedUse::ServiceStart,
            LocationAllowedUse::ServiceEnd,
        ],
        _ => vec![LocationAllowedUse::ReviewOnly],
    }
}

fn infer_station_confidence_from_source(source_kind: LocationSourceKind) -> LocationConfidence {
    match source_kind {
        LocationSourceKind::RouteDefinition | LocationSourceKind::FrontendSpawnPoint => {
            LocationConfidence::High
        }
        LocationSourceKind::Timetable
        | LocationSourceKind::Scenario
        | LocationSourceKind::Training => LocationConfidence::Medium,
        LocationSourceKind::Localization | LocationSourceKind::Unknown => LocationConfidence::Low,
    }
}

fn infer_station_allowed_uses_from_source(source_kind: LocationSourceKind) -> Vec<LocationAllowedUse> {
    match source_kind {
        LocationSourceKind::RouteDefinition | LocationSourceKind::FrontendSpawnPoint => {
            vec![LocationAllowedUse::PlayerSpawn]
        }
        _ => Vec::new(),
    }
}

fn push_unique_copy<T>(values: &mut Vec<T>, value: T)
where
    T: Copy + PartialEq,
{
    if values.contains(&value) {
        return;
    }

    values.push(value);
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
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_workspace(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("tsw_scenario_route_discovery_{name}_{unique}"));
        fs::create_dir_all(&root).expect("failed to create temp workspace");
        root
    }

    #[test]
    fn discovers_route_locations_from_locres_text() {
        let workspace = temp_workspace("locres");
        let locres_dir = workspace
            .join("Content")
            .join("Localization")
            .join("FrankfurtFulda")
            .join("en");
        fs::create_dir_all(&locres_dir).expect("failed to create locres dir");
        fs::write(
            locres_dir.join("FrankfurtFulda.locres"),
            b"Frankfurt - Fulda\0Hanau Hbf Pl 6\0Fulda Platform 1\0Frankfurt (Main) Sud Pl 4\0",
        )
        .expect("failed to write locres");

        let discovery = discover_route("FrankfurtFulda", Some(&workspace), None)
            .expect("route discovery should succeed");

        assert!(discovery.route_titles.contains(&"Frankfurt - Fulda".to_string()));
        assert!(discovery
            .locations
            .iter()
            .any(|location| location.display_name == "Hanau Hbf Pl 6" && location.kind == "platform"));
        assert!(discovery
            .locations
            .iter()
            .any(|location| location.display_name == "Fulda Platform 1"));
    }

    #[test]
    fn discovers_route_overview_and_spawn_points_from_route_definition() {
        let workspace = temp_workspace("route_definition");
        let exports_dir = workspace.join("Exports");
        fs::create_dir_all(&exports_dir).expect("failed to create exports dir");
        fs::write(
            exports_dir.join("FrankfurtFuldaRouteDefinition.json"),
            r#"[
  {
    "Type": "RouteDefinition",
    "Name": "FrankfurtFuldaRouteDefinition",
    "Properties": {
      "DisplayName": {
        "SourceString": "Frankfurt - Fulda: Kinzigtalbahn"
      },
      "StatTrackingName": "FrankfurtFuldaKinzigtalbahn",
      "Level": {
        "AssetPathName": "/FrankfurtFulda/Map/FrankfurtFuldaMap.FrankfurtFuldaMap"
      },
      "RouteMapWidget": {
        "AssetPathName": "/FrankfurtFulda/RouteMapWidget/FrankfurtFuldaRouteMapWidget.FrankfurtFuldaRouteMapWidget_C"
      },
      "RouteDetails": {
        "StartPoint": {
          "SourceString": "Frankfurt"
        },
        "EndPoint": {
          "SourceString": "Fulda"
        },
        "Country": {
          "SourceString": "Deutschland"
        },
        "SpawnPoints": [
          {
            "PointTag": "Hanau Hbf",
            "PointName": {
              "SourceString": "Hanau Hbf"
            },
            "bAvailableInFrontEnd": true,
            "bAvailableInFastTravel": true
          },
          {
            "PointTag": "Fulda",
            "PointName": {
              "SourceString": "Fulda"
            },
            "bAvailableInFrontEnd": true,
            "bAvailableInFastTravel": true
          }
        ]
      }
    }
  }
]"#,
        )
        .expect("failed to write route definition json");

        let discovery = discover_route("FrankfurtFulda", None, Some(&exports_dir))
            .expect("route discovery should succeed");

        assert_eq!(discovery.summary.frontend_spawn_point_count, 2);
        assert_eq!(
            discovery
                .route_overview
                .as_ref()
                .and_then(|overview| overview.display_name.as_deref()),
            Some("Frankfurt - Fulda: Kinzigtalbahn")
        );
        assert!(discovery
            .frontend_spawn_points
            .iter()
            .any(|spawn_point| spawn_point.tag == "Hanau Hbf" && spawn_point.available_in_fast_travel));
        assert!(discovery
            .locations
            .iter()
            .any(|location| location.display_name == "Fulda" && location.kind == "station"));
    }

    #[test]
    fn discovers_stock_and_ribbon_locations_from_exports_json() {
        let workspace = temp_workspace("exports");
        let exports_dir = workspace.join("Exports");
        fs::create_dir_all(&exports_dir).expect("failed to create exports dir");
        fs::write(
            exports_dir.join("timetable.json"),
            r#"{
  "Type": "RouteTimetableDefinition",
  "Properties": {
    "Route": {
      "ObjectName": "RouteDefinition'BremenOldenburgRouteDefinition'"
    },
    "Services": [
      {
        "Instructions": [
          {
            "Destination": {
              "Name": "Delmenhorst Platform 2",
              "Location": {
                "RibbonReference": "334F23BD-4CE392EE-2D00C38C-5EE7E774"
              }
            }
          }
        ]
      }
    ],
    "PlayerStock": "RailVehicleDefinition'RVD_BRO_Press_BR155'"
  }
}"#,
        )
        .expect("failed to write exports json");

        let discovery = discover_route("BremenOldenburg", None, Some(&exports_dir))
            .expect("route discovery should succeed");

        assert!(discovery
            .route_definitions
            .contains(&"BremenOldenburgRouteDefinition".to_string()));
        assert!(discovery
            .stock
            .iter()
            .any(|stock| stock.id == "RVD_BRO_Press_BR155"));
        assert!(discovery.locations.iter().any(|location| {
            location.display_name == "Delmenhorst Platform 2"
                && location.internal_ref.as_deref()
                    == Some("334F23BD-4CE392EE-2D00C38C-5EE7E774")
        }));
    }

    #[test]
    fn saves_and_loads_route_discovery() {
        let workspace = temp_workspace("io");
        let output = workspace.join("route_discovery.json");
        let discovery = RouteDiscovery {
            schema_version: 2,
            route_id: "TestRoute".to_string(),
            route_overview: Some(RouteOverview {
                display_name: Some("Test Route".to_string()),
                stat_tracking_name: Some("TestRoute".to_string()),
                start_point: Some("A".to_string()),
                end_point: Some("B".to_string()),
                country: Some("Testland".to_string()),
                level_asset: Some("/Test/Map/TestMap.TestMap".to_string()),
                route_map_widget: Some("/Test/UI/TestWidget.TestWidget_C".to_string()),
            }),
            frontend_spawn_points: vec![FrontendSpawnPoint {
                tag: "TestHub".to_string(),
                display_name: "Test Hub".to_string(),
                available_in_frontend: true,
                available_in_fast_travel: true,
                source: "test".to_string(),
            }],
            route_titles: vec!["Test Route".to_string()],
            route_definitions: vec!["TestRouteDefinition".to_string()],
            locations: vec![DiscoveredLocation {
                id: "test_platform_1".to_string(),
                display_name: "Test Platform 1".to_string(),
                kind: "platform".to_string(),
                internal_ref: Some("ABC".to_string()),
                source: "test".to_string(),
            }],
            stock: vec![DiscoveredStock {
                id: "RVD_TEST".to_string(),
                source: "test".to_string(),
                evidence: "test".to_string(),
            }],
            sources: vec!["test".to_string()],
            summary: RouteDiscoverySummary {
                source_count: 1,
                route_title_count: 1,
                route_definition_count: 1,
                frontend_spawn_point_count: 1,
                location_count: 1,
                stock_count: 1,
            },
        };

        save_route_discovery(&discovery, &output).expect("save should succeed");
        let loaded = load_route_discovery(&output).expect("load should succeed");
        assert_eq!(loaded, discovery);
    }

    #[test]
    fn builds_location_catalog_grouped_by_station() {
        let discovery = RouteDiscovery {
            schema_version: 2,
            route_id: "FrankfurtFulda".to_string(),
            route_overview: Some(RouteOverview {
                display_name: Some("Frankfurt - Fulda".to_string()),
                stat_tracking_name: None,
                start_point: Some("Frankfurt".to_string()),
                end_point: Some("Fulda".to_string()),
                country: None,
                level_asset: None,
                route_map_widget: None,
            }),
            frontend_spawn_points: vec![
                FrontendSpawnPoint {
                    tag: "HanauHbf".to_string(),
                    display_name: "Hanau Hbf".to_string(),
                    available_in_frontend: true,
                    available_in_fast_travel: true,
                    source: "route_definition".to_string(),
                },
                FrontendSpawnPoint {
                    tag: "FrankfurtSud".to_string(),
                    display_name: "Frankfurt Sud".to_string(),
                    available_in_frontend: true,
                    available_in_fast_travel: false,
                    source: "route_definition".to_string(),
                },
            ],
            route_titles: vec!["Frankfurt - Fulda".to_string()],
            route_definitions: vec!["FrankfurtFuldaRouteDefinition".to_string()],
            locations: vec![
                DiscoveredLocation {
                    id: "hanau_hbf_pl_6".to_string(),
                    display_name: "Hanau Hbf Pl 6".to_string(),
                    kind: "platform".to_string(),
                    internal_ref: Some("HANAU_PL_6".to_string()),
                    source: "timetable".to_string(),
                },
                DiscoveredLocation {
                    id: "hanau_hbf_siding_8".to_string(),
                    display_name: "Hanau Hbf Siding 8".to_string(),
                    kind: "siding".to_string(),
                    internal_ref: Some("HANAU_SIDING_8".to_string()),
                    source: "timetable".to_string(),
                },
                DiscoveredLocation {
                    id: "ffm_sud_pl_4".to_string(),
                    display_name: "Frankfurt (Main) Sud Pl 4".to_string(),
                    kind: "platform".to_string(),
                    internal_ref: Some("FFM_SUD_PL_4".to_string()),
                    source: "timetable".to_string(),
                },
                DiscoveredLocation {
                    id: "west_portal".to_string(),
                    display_name: "West Portal".to_string(),
                    kind: "portal".to_string(),
                    internal_ref: Some("WEST_PORTAL".to_string()),
                    source: "timetable".to_string(),
                },
            ],
            stock: Vec::new(),
            sources: vec!["route_definition".to_string(), "timetable".to_string()],
            summary: RouteDiscoverySummary {
                source_count: 2,
                route_title_count: 1,
                route_definition_count: 1,
                frontend_spawn_point_count: 2,
                location_count: 4,
                stock_count: 0,
            },
        };
        let catalog = build_location_catalog(&discovery);
        assert_eq!(catalog.summary.station_count, 2);
        assert_eq!(catalog.summary.grouped_location_count, 3);
        assert_eq!(catalog.summary.ungrouped_location_count, 1);
        let hanau = catalog
            .stations
            .iter()
            .find(|station| station.display_name == "Hanau Hbf")
            .expect("Hanau Hbf station should exist");
        assert_eq!(hanau.frontend_spawn_points.len(), 1);
        assert_eq!(hanau.confidence, LocationConfidence::High);
        assert!(hanau.allowed_uses.contains(&LocationAllowedUse::PlayerSpawn));
        assert!(hanau
            .locations
            .iter()
            .any(|location| location.display_name == "Hanau Hbf Pl 6"));
        assert!(hanau
            .locations
            .iter()
            .any(|location| location.display_name == "Hanau Hbf Siding 8"));
        let hanau_platform = hanau
            .locations
            .iter()
            .find(|location| location.display_name == "Hanau Hbf Pl 6")
            .expect("Hanau Hbf platform should exist");
        assert_eq!(hanau_platform.source_kind, LocationSourceKind::Timetable);
        assert_eq!(hanau_platform.confidence, LocationConfidence::High);
        assert!(hanau_platform
            .allowed_uses
            .contains(&LocationAllowedUse::ServiceStart));
        assert!(hanau_platform
            .allowed_uses
            .contains(&LocationAllowedUse::ObjectiveLocation));
        let frankfurt_sud = catalog
            .stations
            .iter()
            .find(|station| station.display_name == "Frankfurt Sud")
            .expect("Frankfurt Sud station should exist");
        assert!(frankfurt_sud
            .locations
            .iter()
            .any(|location| location.display_name == "Frankfurt (Main) Sud Pl 4"));
        assert_eq!(catalog.ungrouped_locations.len(), 1);
        assert_eq!(catalog.ungrouped_locations[0].display_name, "West Portal");
        assert_eq!(catalog.ungrouped_locations[0].source_kind, LocationSourceKind::Timetable);
        assert_eq!(catalog.ungrouped_locations[0].confidence, LocationConfidence::High);
        assert_eq!(
            catalog.ungrouped_locations[0].allowed_uses,
            vec![LocationAllowedUse::ServiceStart, LocationAllowedUse::ServiceEnd]
        );
    }
    #[test]
    fn resolves_locations_using_spawn_tags_and_internal_refs() {
        let catalog = LocationCatalog {
            schema_version: 2,
            route_id: "TestRoute".to_string(),
            route_overview: None,
            stations: vec![LocationCatalogStation {
                id: "hanau_hbf".to_string(),
                display_name: "Hanau Hbf".to_string(),
                confidence: LocationConfidence::High,
                allowed_uses: vec![LocationAllowedUse::PlayerSpawn],
                tags: vec!["HanauHbf".to_string()],
                frontend_spawn_points: vec![FrontendSpawnPoint {
                    tag: "HanauHbf".to_string(),
                    display_name: "Hanau Hbf".to_string(),
                    available_in_frontend: true,
                    available_in_fast_travel: true,
                    source: "route_definition".to_string(),
                }],
                locations: vec![LocationCatalogEntry {
                    id: "hanau_hbf_pl_6".to_string(),
                    display_name: "Hanau Hbf Pl 6".to_string(),
                    kind: "platform".to_string(),
                    source_kind: LocationSourceKind::Timetable,
                    confidence: LocationConfidence::High,
                    allowed_uses: vec![LocationAllowedUse::ServiceStart, LocationAllowedUse::ServiceEnd],
                    internal_ref: Some("HANAU_PL_6".to_string()),
                    source: "timetable".to_string(),
                }],
            }],
            ungrouped_locations: Vec::new(),
            summary: LocationCatalogSummary {
                station_count: 1,
                frontend_spawn_point_count: 1,
                grouped_location_count: 1,
                ungrouped_location_count: 0,
            },
        };

        let player_matches = resolve_location_matches(
            &catalog,
            "Hanau Hbf",
            LocationLookupUsage::PlayerSpawn,
        );
        assert_eq!(player_matches.len(), 1);
        assert_eq!(player_matches[0].match_kind, LocationMatchKind::FrontendSpawnPoint);
        assert_eq!(player_matches[0].spawn_tag.as_deref(), Some("HanauHbf"));

        let service_matches = resolve_location_matches(
            &catalog,
            "Hanau Hbf Pl 6",
            LocationLookupUsage::Service,
        );
        assert_eq!(service_matches.len(), 1);
        assert_eq!(service_matches[0].match_kind, LocationMatchKind::CatalogEntry);
        assert_eq!(service_matches[0].internal_ref.as_deref(), Some("HANAU_PL_6"));
    }

    #[test]
    fn resolves_locations_preferring_high_confidence_internal_refs() {
        let catalog = LocationCatalog {
            schema_version: 2,
            route_id: "TestRoute".to_string(),
            route_overview: None,
            stations: Vec::new(),
            ungrouped_locations: vec![
                LocationCatalogEntry {
                    id: "shared_track".to_string(),
                    display_name: "Shared Track".to_string(),
                    kind: "track".to_string(),
                    source_kind: LocationSourceKind::Localization,
                    confidence: LocationConfidence::Low,
                    allowed_uses: vec![LocationAllowedUse::ServiceStart, LocationAllowedUse::ServiceEnd],
                    internal_ref: None,
                    source: "locres".to_string(),
                },
                LocationCatalogEntry {
                    id: "shared_track".to_string(),
                    display_name: "Shared Track".to_string(),
                    kind: "track".to_string(),
                    source_kind: LocationSourceKind::Timetable,
                    confidence: LocationConfidence::High,
                    allowed_uses: vec![LocationAllowedUse::ServiceStart, LocationAllowedUse::ServiceEnd],
                    internal_ref: Some("RIBBON_SHARED".to_string()),
                    source: "timetable".to_string(),
                },
            ],
            summary: LocationCatalogSummary {
                station_count: 0,
                frontend_spawn_point_count: 0,
                grouped_location_count: 0,
                ungrouped_location_count: 2,
            },
        };

        let matches = resolve_location_matches(
            &catalog,
            "Shared Track",
            LocationLookupUsage::Service,
        );

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].confidence, LocationConfidence::High);
        assert_eq!(matches[0].internal_ref.as_deref(), Some("RIBBON_SHARED"));
    }

    #[test]
    fn resolves_locations_with_umlaut_normalization() {
        let catalog = LocationCatalog {
            schema_version: 2,
            route_id: "TestRoute".to_string(),
            route_overview: None,
            stations: Vec::new(),
            ungrouped_locations: vec![LocationCatalogEntry {
                id: "frankfurt_sued_pl_1".to_string(),
                display_name: "Frankfurt SÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¼d Pl 1".to_string(),
                kind: "platform".to_string(),
                source_kind: LocationSourceKind::Timetable,
                confidence: LocationConfidence::High,
                allowed_uses: vec![LocationAllowedUse::ServiceStart, LocationAllowedUse::ServiceEnd],
                internal_ref: Some("FFM_SUED_PL_1".to_string()),
                source: "timetable".to_string(),
            }],
            summary: LocationCatalogSummary {
                station_count: 0,
                frontend_spawn_point_count: 0,
                grouped_location_count: 0,
                ungrouped_location_count: 1,
            },
        };

        let matches = resolve_location_matches(
            &catalog,
            "Frankfurt Sued Pl 1",
            LocationLookupUsage::Service,
        );

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].display_name, "Frankfurt SÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¼d Pl 1");
        assert_eq!(matches[0].internal_ref.as_deref(), Some("FFM_SUED_PL_1"));
    }
}
