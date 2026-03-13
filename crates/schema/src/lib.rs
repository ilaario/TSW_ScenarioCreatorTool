use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub id: String,
    pub title: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub route: String,
    pub template: String,
    pub start_time: String,
    pub weather: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ScenarioLocation {
    Simple(String),
    Detailed(ScenarioLocationReference),
}

impl ScenarioLocation {
    pub fn reference(&self) -> Option<&ScenarioLocationReference> {
        match self {
            Self::Simple(_) => None,
            Self::Detailed(reference) => Some(reference),
        }
    }

    pub fn primary_value(&self) -> Option<&str> {
        match self {
            Self::Simple(value) => non_empty_option(Some(value.as_str())),
            Self::Detailed(reference) => reference.primary_value(),
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Simple(value) => non_empty_option(Some(value.as_str())),
            Self::Detailed(reference) => non_empty_option(reference.name.as_deref()),
        }
    }

    pub fn catalog_id(&self) -> Option<&str> {
        match self {
            Self::Simple(_) => None,
            Self::Detailed(reference) => non_empty_option(reference.catalog_id.as_deref()),
        }
    }

    pub fn spawn_tag(&self) -> Option<&str> {
        match self {
            Self::Simple(_) => None,
            Self::Detailed(reference) => non_empty_option(reference.spawn_tag.as_deref()),
        }
    }

    pub fn internal_ref(&self) -> Option<&str> {
        match self {
            Self::Simple(_) => None,
            Self::Detailed(reference) => non_empty_option(reference.internal_ref.as_deref()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.primary_value().is_none()
    }

    pub fn display_label(&self) -> String {
        self.primary_value().unwrap_or_default().to_string()
    }

    pub fn has_explicit_selector(&self) -> bool {
        self.catalog_id().is_some() || self.spawn_tag().is_some() || self.internal_ref().is_some()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioLocationReference {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawn_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_ref: Option<String>,
}

impl ScenarioLocationReference {
    pub fn primary_value(&self) -> Option<&str> {
        non_empty_option(self.name.as_deref())
            .or_else(|| non_empty_option(self.spawn_tag.as_deref()))
            .or_else(|| non_empty_option(self.catalog_id.as_deref()))
            .or_else(|| non_empty_option(self.internal_ref.as_deref()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ScenarioAssetReference {
    Simple(String),
    Detailed(ScenarioAssetSelector),
}

impl ScenarioAssetReference {
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Simple(value) => non_empty_option(Some(value.as_str())),
            Self::Detailed(reference) => non_empty_option(Some(reference.id.as_str())),
        }
    }

    pub fn plugin(&self) -> Option<&str> {
        match self {
            Self::Simple(_) => None,
            Self::Detailed(reference) => non_empty_option(reference.plugin.as_deref()),
        }
    }

    pub fn source(&self) -> Option<&str> {
        match self {
            Self::Simple(_) => None,
            Self::Detailed(reference) => non_empty_option(reference.source.as_deref()),
        }
    }

    pub fn display_label(&self) -> String {
        self.id().unwrap_or_default().to_string()
    }

    pub fn has_explicit_selector(&self) -> bool {
        self.plugin().is_some() || self.source().is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioAssetSelector {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlayerService {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consist: Option<ScenarioAssetReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formation_ref: Option<ScenarioAssetReference>,
    pub start_location: ScenarioLocation,
    pub destination: ScenarioLocation,
}

impl PlayerService {
    pub fn consist_id(&self) -> Option<&str> {
        self.consist.as_ref().and_then(ScenarioAssetReference::id)
    }

    pub fn formation_id(&self) -> Option<&str> {
        non_empty_option(self.formation.as_deref())
    }

    pub fn formation_ref_id(&self) -> Option<&str> {
        self.formation_ref.as_ref().and_then(ScenarioAssetReference::id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AiService {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consist: Option<ScenarioAssetReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formation_ref: Option<ScenarioAssetReference>,
    pub start_location: ScenarioLocation,
    pub destination: ScenarioLocation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub departure_time: Option<String>,
}

impl AiService {
    pub fn consist_id(&self) -> Option<&str> {
        self.consist.as_ref().and_then(ScenarioAssetReference::id)
    }

    pub fn formation_id(&self) -> Option<&str> {
        non_empty_option(self.formation.as_deref())
    }

    pub fn formation_ref_id(&self) -> Option<&str> {
        self.formation_ref.as_ref().and_then(ScenarioAssetReference::id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FormationCargo {
    pub asset: String,
    #[serde(default = "default_cargo_units", skip_serializing_if = "is_default_cargo_units")]
    pub units: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FormationEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vehicle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formation: Option<String>,
    #[serde(default = "default_entry_count", skip_serializing_if = "is_default_entry_count")]
    pub count: usize,
    #[serde(default)]
    pub flipped: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flipped_indices: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo: Option<FormationCargo>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FormationDefinition {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<FormationEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveKind {
    ReachDestination,
    StopAt,
    ArriveBy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Objective {
    pub id: String,
    pub description: String,
    pub kind: ObjectiveKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<ScenarioLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionKind {
    AllObjectivesCompleted,
    ObjectiveCompleted,
    ObjectiveFailed,
    ServiceArrived,
    TimeReached,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub kind: ConditionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompletionRules {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub success: Vec<Condition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failure: Vec<Condition>,
}

impl CompletionRules {
    pub fn is_empty(&self) -> bool {
        self.success.is_empty() && self.failure.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioProject {
    pub meta: Meta,
    pub scenario: Scenario,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub formations: BTreeMap<String, FormationDefinition>,
    pub player_service: PlayerService,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ai_services: Vec<AiService>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objectives: Vec<Objective>,
    #[serde(default, skip_serializing_if = "CompletionRules::is_empty")]
    pub completion: CompletionRules,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum TemplateReference {
    Id(String),
    Detailed(TemplateProfileRef),
}

impl TemplateReference {
    pub fn id(&self) -> &str {
        match self {
            Self::Id(id) => id,
            Self::Detailed(reference) => &reference.id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TemplateProfileRef {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RouteProfile {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_version: Option<String>,
    #[serde(default)]
    pub supported_stock: Vec<String>,
    #[serde(default)]
    pub spawn_points: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub player_spawn_points: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub service_locations: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objective_locations: Vec<String>,
    #[serde(default)]
    pub templates: Vec<TemplateReference>,
    #[serde(default)]
    pub supported_weather: Vec<String>,
    #[serde(default)]
    pub install_ids: Vec<String>,
    #[serde(default)]
    pub install_hints: Vec<String>,
}

impl RouteProfile {
    pub fn supports_stock(&self, consist_id: &str) -> bool {
        self.supported_stock.iter().any(|stock| stock == consist_id)
    }

    pub fn has_spawn_point(&self, spawn_point_id: &str) -> bool {
        self.spawn_points.iter().any(|spawn_point| spawn_point == spawn_point_id)
    }

    pub fn has_player_spawn_point(&self, location_id: &str) -> bool {
        self.has_spawn_point(location_id)
            || self
                .player_spawn_points
                .iter()
                .any(|spawn_point| spawn_point == location_id)
    }

    pub fn has_service_location(&self, location_id: &str) -> bool {
        self.has_player_spawn_point(location_id)
            || self
                .service_locations
                .iter()
                .any(|location| location == location_id)
    }

    pub fn has_objective_location(&self, location_id: &str) -> bool {
        self.has_service_location(location_id)
            || self
                .objective_locations
                .iter()
                .any(|location| location == location_id)
    }

    pub fn supports_template(&self, template_id: &str) -> bool {
        self.templates
            .iter()
            .any(|template| template.id() == template_id)
    }

    pub fn supports_weather(&self, weather: &str) -> bool {
        self.supported_weather.is_empty()
            || self
                .supported_weather
                .iter()
                .any(|candidate| candidate == weather)
    }

    pub fn canonical_install_ids(&self) -> Vec<String> {
        let mut identifiers = Vec::new();

        for install_id in &self.install_ids {
            push_unique_identifier(&mut identifiers, install_id);
        }

        identifiers
    }

    pub fn install_aliases(&self) -> Vec<String> {
        let mut identifiers = Vec::new();
        push_unique_identifier(&mut identifiers, &self.id);
        push_unique_identifier(&mut identifiers, &self.name);

        for hint in &self.install_hints {
            push_unique_identifier(&mut identifiers, hint);
        }

        identifiers
    }

    pub fn install_identifiers(&self) -> Vec<String> {
        let mut identifiers = self.canonical_install_ids();

        for alias in self.install_aliases() {
            push_unique_identifier(&mut identifiers, &alias);
        }

        identifiers
    }
}

fn push_unique_identifier(identifiers: &mut Vec<String>, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }

    if identifiers
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(trimmed))
    {
        return;
    }

    identifiers.push(trimmed.to_string());
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TemplateDefinition {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_subdir: Option<String>,
    #[serde(default)]
    pub render_extensions: Vec<String>,
}

impl TemplateDefinition {
    pub fn output_subdir(&self) -> &str {
        self.output_subdir.as_deref().unwrap_or("template")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledTemplateInfo {
    pub id: String,
    pub name: String,
    pub output_subdir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledScenarioSummary {
    pub formation_count: usize,
    pub ai_service_count: usize,
    pub objective_count: usize,
    pub success_condition_count: usize,
    pub failure_condition_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResolvedFormationVehicle {
    pub vehicle: String,
    pub flipped: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo: Option<FormationCargo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledFormation {
    pub id: String,
    pub definition: FormationDefinition,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved_vehicles: Vec<ResolvedFormationVehicle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedServiceReferenceKind {
    Stock,
    ProjectFormation,
    CatalogFormation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResolvedServiceReference {
    pub query: String,
    pub kind: ResolvedServiceReferenceKind,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drivable: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved_vehicles: Vec<ResolvedFormationVehicle>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub ambiguous: bool,
    #[serde(default, skip_serializing_if = "is_zero_or_one")]
    pub candidate_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledAiServiceReference {
    pub service_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<ResolvedServiceReference>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledResolvedServiceReferences {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_service: Option<ResolvedServiceReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ai_services: Vec<CompiledAiServiceReference>,
}

impl CompiledResolvedServiceReferences {
    pub fn is_empty(&self) -> bool {
        self.player_service.is_none() && self.ai_services.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedLocationMatchKind {
    FrontendSpawnPoint,
    Station,
    CatalogEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResolvedLocationReference {
    pub query: String,
    pub match_kind: ResolvedLocationMatchKind,
    pub catalog_id: String,
    pub display_name: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawn_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub ambiguous: bool,
    #[serde(default, skip_serializing_if = "is_zero_or_one")]
    pub candidate_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledAiServiceLocations {
    pub service_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<ResolvedLocationReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<ResolvedLocationReference>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledObjectiveLocation {
    pub objective_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<ResolvedLocationReference>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledResolvedLocations {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_start: Option<ResolvedLocationReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_destination: Option<ResolvedLocationReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ai_services: Vec<CompiledAiServiceLocations>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objectives: Vec<CompiledObjectiveLocation>,
}

impl CompiledResolvedLocations {
    pub fn is_empty(&self) -> bool {
        self.player_start.is_none()
            && self.player_destination.is_none()
            && self.ai_services.is_empty()
            && self.objectives.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledScenario {
    pub schema_version: u32,
    pub meta: Meta,
    pub scenario: Scenario,
    pub player_service: PlayerService,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ai_services: Vec<AiService>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formations: Vec<CompiledFormation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objectives: Vec<Objective>,
    #[serde(default, skip_serializing_if = "CompletionRules::is_empty")]
    pub completion: CompletionRules,
    #[serde(default, skip_serializing_if = "CompiledResolvedLocations::is_empty")]
    pub resolved_locations: CompiledResolvedLocations,
    #[serde(default, skip_serializing_if = "CompiledResolvedServiceReferences::is_empty")]
    pub resolved_service_references: CompiledResolvedServiceReferences,
    pub template: CompiledTemplateInfo,
    pub summary: CompiledScenarioSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageEntryKind {
    CompiledScenario,
    TemplateFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackageEntry {
    pub kind: PackageEntryKind,
    pub source_path: String,
    pub staged_path: String,
    pub mount_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackagePlan {
    pub schema_version: u32,
    pub scenario_id: String,
    pub game_version: String,
    pub package_namespace: String,
    pub suggested_pak_name: String,
    pub staging_root: String,
    pub mount_root: String,
    pub entries: Vec<PackageEntry>,
}

fn default_entry_count() -> usize {
    1
}

fn is_default_entry_count(value: &usize) -> bool {
    *value == default_entry_count()
}

fn default_cargo_units() -> u32 {
    1
}

fn is_default_cargo_units(value: &u32) -> bool {
    *value == default_cargo_units()
}

fn is_false(value: &bool) -> bool {
    !value
}

fn is_zero_or_one(value: &usize) -> bool {
    *value <= 1
}

fn non_empty_option(value: Option<&str>) -> Option<&str> {
    value.filter(|candidate| !candidate.trim().is_empty())
}



