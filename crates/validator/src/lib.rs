use std::collections::HashSet;

use serde::Serialize;
use tsw_scenario_schema::{
    AiService, Condition, ConditionKind, Objective, ObjectiveKind, RouteProfile, ScenarioProject,
    TemplateDefinition,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ValidationIssue {
    pub code: String,
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct ValidationReport {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }
}

pub fn validate_project(
    project: &ScenarioProject,
    route_profile: &RouteProfile,
    template_definition: &TemplateDefinition,
) -> ValidationReport {
    let mut report = ValidationReport::default();

    require_non_empty(&mut report, "meta.id", &project.meta.id);
    require_non_empty(&mut report, "meta.title", &project.meta.title);
    require_non_empty(&mut report, "meta.author", &project.meta.author);
    require_non_empty(&mut report, "scenario.route", &project.scenario.route);
    require_non_empty(&mut report, "scenario.template", &project.scenario.template);
    require_non_empty(&mut report, "scenario.start_time", &project.scenario.start_time);
    require_non_empty(&mut report, "scenario.weather", &project.scenario.weather);
    require_non_empty(
        &mut report,
        "player_service.consist",
        &project.player_service.consist,
    );
    require_non_empty(
        &mut report,
        "player_service.start_location",
        &project.player_service.start_location,
    );
    require_non_empty(
        &mut report,
        "player_service.destination",
        &project.player_service.destination,
    );

    validate_service(
        &mut report,
        route_profile,
        "player_service",
        None,
        &project.player_service.consist,
        &project.player_service.start_location,
        &project.player_service.destination,
        None,
    );

    if !project.scenario.route.is_empty() && project.scenario.route != route_profile.id {
        report.errors.push(issue(
            "ROUTE_MISMATCH",
            "scenario.route",
            format!(
                "scenario route '{}' does not match loaded route profile '{}'",
                project.scenario.route, route_profile.id
            ),
        ));
    }

    if !project.scenario.template.is_empty()
        && !route_profile.supports_template(&project.scenario.template)
    {
        report.errors.push(issue(
            "INVALID_TEMPLATE",
            "scenario.template",
            format!(
                "template '{}' is not supported by route '{}'",
                project.scenario.template, route_profile.id
            ),
        ));
    }

    if !project.scenario.template.is_empty() && project.scenario.template != template_definition.id {
        report.errors.push(issue(
            "TEMPLATE_MISMATCH",
            "scenario.template",
            format!(
                "scenario template '{}' does not match loaded template definition '{}'",
                project.scenario.template, template_definition.id
            ),
        ));
    }

    if !project.scenario.weather.is_empty() && !route_profile.supports_weather(&project.scenario.weather)
    {
        report.errors.push(issue(
            "INVALID_WEATHER",
            "scenario.weather",
            format!(
                "weather '{}' is not supported by route '{}'",
                project.scenario.weather, route_profile.id
            ),
        ));
    }

    if !project.scenario.start_time.is_empty() && !is_valid_time_format(&project.scenario.start_time)
    {
        report.errors.push(issue(
            "INVALID_START_TIME",
            "scenario.start_time",
            format!(
                "start_time '{}' must use HH:MM 24-hour format",
                project.scenario.start_time
            ),
        ));
    }

    if project.player_service.start_location == project.player_service.destination {
        report.warnings.push(issue(
            "START_EQUALS_DESTINATION",
            "player_service.destination",
            "start_location and destination are identical".to_string(),
        ));
    }

    let mut ai_service_ids = HashSet::new();
    for (index, ai_service) in project.ai_services.iter().enumerate() {
        validate_ai_service(&mut report, route_profile, ai_service, index, &mut ai_service_ids);
    }

    let mut objective_ids = HashSet::new();
    for (index, objective) in project.objectives.iter().enumerate() {
        validate_objective(&mut report, route_profile, objective, index, &mut objective_ids);
    }

    validate_completion_rules(&mut report, project, &objective_ids, &ai_service_ids);

    report
}

fn validate_ai_service(
    report: &mut ValidationReport,
    route_profile: &RouteProfile,
    ai_service: &AiService,
    index: usize,
    ai_service_ids: &mut HashSet<String>,
) {
    let field_prefix = format!("ai_services[{index}]");
    require_non_empty(report, &format!("{field_prefix}.id"), &ai_service.id);
    require_non_empty(
        report,
        &format!("{field_prefix}.consist"),
        &ai_service.consist,
    );
    require_non_empty(
        report,
        &format!("{field_prefix}.start_location"),
        &ai_service.start_location,
    );
    require_non_empty(
        report,
        &format!("{field_prefix}.destination"),
        &ai_service.destination,
    );

    if !ai_service.id.is_empty() && !ai_service_ids.insert(ai_service.id.clone()) {
        report.errors.push(issue(
            "DUPLICATE_AI_SERVICE_ID",
            &format!("{field_prefix}.id"),
            format!("AI service id '{}' is duplicated", ai_service.id),
        ));
    }

    validate_service(
        report,
        route_profile,
        &field_prefix,
        Some(&ai_service.id),
        &ai_service.consist,
        &ai_service.start_location,
        &ai_service.destination,
        ai_service.departure_time.as_deref(),
    );
}

fn validate_objective(
    report: &mut ValidationReport,
    route_profile: &RouteProfile,
    objective: &Objective,
    index: usize,
    objective_ids: &mut HashSet<String>,
) {
    let field_prefix = format!("objectives[{index}]");
    require_non_empty(report, &format!("{field_prefix}.id"), &objective.id);
    require_non_empty(
        report,
        &format!("{field_prefix}.description"),
        &objective.description,
    );

    if !objective.id.is_empty() && !objective_ids.insert(objective.id.clone()) {
        report.errors.push(issue(
            "DUPLICATE_OBJECTIVE_ID",
            &format!("{field_prefix}.id"),
            format!("objective id '{}' is duplicated", objective.id),
        ));
    }

    match objective.kind {
        ObjectiveKind::ReachDestination => {}
        ObjectiveKind::StopAt => {
            if objective.location.as_deref().unwrap_or_default().trim().is_empty() {
                report.errors.push(issue(
                    "MISSING_OBJECTIVE_LOCATION",
                    &format!("{field_prefix}.location"),
                    "stop_at objectives require a location".to_string(),
                ));
            }
        }
        ObjectiveKind::ArriveBy => {
            if objective.time.as_deref().unwrap_or_default().trim().is_empty() {
                report.errors.push(issue(
                    "MISSING_OBJECTIVE_TIME",
                    &format!("{field_prefix}.time"),
                    "arrive_by objectives require a time".to_string(),
                ));
            }
        }
    }

    if let Some(location) = objective.location.as_deref() {
        if !location.trim().is_empty() {
            validate_spawn_point(report, route_profile, &format!("{field_prefix}.location"), location);
        }
    }

    if let Some(time) = objective.time.as_deref() {
        if !time.trim().is_empty() {
            validate_time(report, &format!("{field_prefix}.time"), time);
        }
    }
}

fn validate_completion_rules(
    report: &mut ValidationReport,
    project: &ScenarioProject,
    objective_ids: &HashSet<String>,
    ai_service_ids: &HashSet<String>,
) {
    if project.objectives.is_empty() {
        report.warnings.push(issue(
            "NO_OBJECTIVES_DEFINED",
            "objectives",
            "no objectives defined; scenario success will rely only on completion rules".to_string(),
        ));
    }

    if project.completion.success.is_empty() {
        report.warnings.push(issue(
            "NO_SUCCESS_CONDITIONS",
            "completion.success",
            "no explicit success conditions defined".to_string(),
        ));
    }

    for (index, condition) in project.completion.success.iter().enumerate() {
        validate_condition(
            report,
            condition,
            &format!("completion.success[{index}]"),
            objective_ids,
            ai_service_ids,
        );
    }

    for (index, condition) in project.completion.failure.iter().enumerate() {
        validate_condition(
            report,
            condition,
            &format!("completion.failure[{index}]"),
            objective_ids,
            ai_service_ids,
        );
    }
}

fn validate_condition(
    report: &mut ValidationReport,
    condition: &Condition,
    field_prefix: &str,
    objective_ids: &HashSet<String>,
    ai_service_ids: &HashSet<String>,
) {
    match condition.kind {
        ConditionKind::AllObjectivesCompleted => {}
        ConditionKind::ObjectiveCompleted | ConditionKind::ObjectiveFailed => {
            let objective_id = condition.objective_id.as_deref().unwrap_or_default();
            if objective_id.trim().is_empty() {
                report.errors.push(issue(
                    "MISSING_CONDITION_OBJECTIVE_ID",
                    &format!("{field_prefix}.objective_id"),
                    "this condition requires an objective_id".to_string(),
                ));
            } else if !objective_ids.contains(objective_id) {
                report.errors.push(issue(
                    "UNKNOWN_OBJECTIVE_REFERENCE",
                    &format!("{field_prefix}.objective_id"),
                    format!("objective '{}' is not defined in objectives", objective_id),
                ));
            }
        }
        ConditionKind::ServiceArrived => {
            let service_id = condition.service_id.as_deref().unwrap_or_default();
            if service_id.trim().is_empty() {
                report.errors.push(issue(
                    "MISSING_CONDITION_SERVICE_ID",
                    &format!("{field_prefix}.service_id"),
                    "this condition requires a service_id".to_string(),
                ));
            } else if service_id != "player" && !ai_service_ids.contains(service_id) {
                report.errors.push(issue(
                    "UNKNOWN_SERVICE_REFERENCE",
                    &format!("{field_prefix}.service_id"),
                    format!(
                        "service '{}' is not defined in ai_services or reserved as 'player'",
                        service_id
                    ),
                ));
            }
        }
        ConditionKind::TimeReached => {
            let time = condition.time.as_deref().unwrap_or_default();
            if time.trim().is_empty() {
                report.errors.push(issue(
                    "MISSING_CONDITION_TIME",
                    &format!("{field_prefix}.time"),
                    "time_reached conditions require a time".to_string(),
                ));
            } else {
                validate_time(report, &format!("{field_prefix}.time"), time);
            }
        }
    }
}

fn validate_service(
    report: &mut ValidationReport,
    route_profile: &RouteProfile,
    field_prefix: &str,
    service_id: Option<&str>,
    consist: &str,
    start_location: &str,
    destination: &str,
    departure_time: Option<&str>,
) {
    if !consist.trim().is_empty() && !route_profile.supports_stock(consist) {
        report.errors.push(issue(
            "INVALID_ROLLING_STOCK",
            &format!("{field_prefix}.consist"),
            format!(
                "rolling stock '{}' is not supported by route '{}'",
                consist, route_profile.id
            ),
        ));
    }

    validate_spawn_point(
        report,
        route_profile,
        &format!("{field_prefix}.start_location"),
        start_location,
    );
    validate_spawn_point(
        report,
        route_profile,
        &format!("{field_prefix}.destination"),
        destination,
    );

    if let Some(time) = departure_time {
        if !time.trim().is_empty() {
            validate_time(report, &format!("{field_prefix}.departure_time"), time);
        }
    }

    if let Some(service_id) = service_id {
        if !service_id.trim().is_empty() && start_location == destination {
            report.warnings.push(issue(
                "AI_SERVICE_LOOP",
                &format!("{field_prefix}.destination"),
                format!(
                    "AI service '{}' starts and ends at the same spawn point",
                    service_id
                ),
            ));
        }
    }
}

fn validate_spawn_point(
    report: &mut ValidationReport,
    route_profile: &RouteProfile,
    field: &str,
    spawn_point: &str,
) {
    if !spawn_point.trim().is_empty() && !route_profile.has_spawn_point(spawn_point) {
        report.errors.push(issue(
            "INVALID_SPAWN_POINT",
            field,
            format!(
                "spawn point '{}' is not available on route '{}'",
                spawn_point, route_profile.id
            ),
        ));
    }
}

fn validate_time(report: &mut ValidationReport, field: &str, value: &str) {
    if !is_valid_time_format(value) {
        report.errors.push(issue(
            "INVALID_TIME",
            field,
            format!("time '{}' must use HH:MM 24-hour format", value),
        ));
    }
}

fn require_non_empty(report: &mut ValidationReport, field: &str, value: &str) {
    if value.trim().is_empty() {
        report.errors.push(issue(
            "MISSING_REQUIRED_FIELD",
            field,
            format!("required field '{field}' cannot be empty"),
        ));
    }
}

fn issue(code: &str, field: &str, message: String) -> ValidationIssue {
    ValidationIssue {
        code: code.to_string(),
        field: field.to_string(),
        message,
    }
}

fn is_valid_time_format(value: &str) -> bool {
    let mut parts = value.split(':');
    let Some(hours) = parts.next() else {
        return false;
    };
    let Some(minutes) = parts.next() else {
        return false;
    };

    if parts.next().is_some() || hours.len() != 2 || minutes.len() != 2 {
        return false;
    }

    match (hours.parse::<u8>(), minutes.parse::<u8>()) {
        (Ok(hours), Ok(minutes)) => hours < 24 && minutes < 60,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tsw_scenario_schema::{
        AiService, CompletionRules, Condition, ConditionKind, Meta, Objective, ObjectiveKind,
        PlayerService, RouteProfile, Scenario, ScenarioProject, TemplateDefinition,
        TemplateReference,
    };

    fn sample_project() -> ScenarioProject {
        ScenarioProject {
            meta: Meta {
                id: "rro_test_001".to_string(),
                title: "Test Scenario".to_string(),
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
            objectives: vec![
                Objective {
                    id: "stop_bochum".to_string(),
                    description: "Reach Bochum Hbf".to_string(),
                    kind: ObjectiveKind::StopAt,
                    location: Some("Bochum_Hbf_P3".to_string()),
                    time: None,
                },
                Objective {
                    id: "arrive_on_time".to_string(),
                    description: "Arrive before 08:45".to_string(),
                    kind: ObjectiveKind::ArriveBy,
                    location: None,
                    time: Some("08:45".to_string()),
                },
            ],
            completion: CompletionRules {
                success: vec![Condition {
                    kind: ConditionKind::AllObjectivesCompleted,
                    objective_id: None,
                    service_id: None,
                    time: None,
                    description: Some("Complete all scenario objectives".to_string()),
                }],
                failure: vec![Condition {
                    kind: ConditionKind::TimeReached,
                    objective_id: None,
                    service_id: None,
                    time: Some("09:00".to_string()),
                    description: Some("Scenario timeout".to_string()),
                }],
            },
        }
    }

    fn sample_route_profile() -> RouteProfile {
        RouteProfile {
            id: "RRO".to_string(),
            name: "Ruhr-Sieg Nord".to_string(),
            description: None,
            game_version: Some("tsw5".to_string()),
            supported_stock: vec!["DB_BR422".to_string()],
            spawn_points: vec!["Essen_Hbf_P5".to_string(), "Bochum_Hbf_P3".to_string()],
            templates: vec![TemplateReference::Id("commuter_simple".to_string())],
            supported_weather: vec!["clear".to_string(), "cloudy".to_string()],
            install_ids: vec!["RuhrSiegNord".to_string()],
            install_hints: vec!["RRO".to_string(), "Ruhr-Sieg Nord".to_string()],
        }
    }

    fn sample_template_definition() -> TemplateDefinition {
        TemplateDefinition {
            id: "commuter_simple".to_string(),
            name: "Commuter Simple".to_string(),
            description: None,
            output_subdir: Some("template".to_string()),
            render_extensions: vec!["yaml".to_string(), "txt".to_string()],
        }
    }

    #[test]
    fn accepts_valid_project_with_ai_objectives_and_completion_rules() {
        let report = validate_project(
            &sample_project(),
            &sample_route_profile(),
            &sample_template_definition(),
        );

        assert!(report.is_valid());
        assert_eq!(report.error_count(), 0);
    }

    #[test]
    fn rejects_invalid_ai_service_and_unknown_completion_references() {
        let mut project = sample_project();
        project.ai_services.push(AiService {
            id: "ai_regional_01".to_string(),
            consist: "DB_BR422".to_string(),
            start_location: "Unknown".to_string(),
            destination: "Bochum_Hbf_P3".to_string(),
            departure_time: Some("8:05".to_string()),
        });
        project.completion.failure.push(Condition {
            kind: ConditionKind::ObjectiveFailed,
            objective_id: Some("missing_objective".to_string()),
            service_id: None,
            time: None,
            description: None,
        });

        let report = validate_project(
            &project,
            &sample_route_profile(),
            &sample_template_definition(),
        );

        assert!(report.has_errors());
        assert!(report
            .errors
            .iter()
            .any(|issue| issue.code == "DUPLICATE_AI_SERVICE_ID"));
        assert!(report
            .errors
            .iter()
            .any(|issue| issue.code == "INVALID_SPAWN_POINT"));
        assert!(report
            .errors
            .iter()
            .any(|issue| issue.code == "INVALID_TIME"));
        assert!(report
            .errors
            .iter()
            .any(|issue| issue.code == "UNKNOWN_OBJECTIVE_REFERENCE"));
    }
}