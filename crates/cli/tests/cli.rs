use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_workspace(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("tsw_scenario_tool_cli_{name}_{unique}"));
    fs::create_dir_all(&root).expect("failed to create temp workspace");
    root
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create parent directory");
    }
    fs::write(path, content).expect("failed to write test file");
}

fn create_fixture_workspace(pak_names: &[&str]) -> PathBuf {
    let workspace = temp_workspace("fixture");

    write_file(
        &workspace.join("examples").join("scenario.yaml"),
        "meta:\n  id: cli_test_001\n  title: CLI Test\n  author: Dario\n\nscenario:\n  route: RRO\n  template: commuter_simple\n  start_time: \"08:15\"\n  weather: cloudy\n\nformations:\n  player_train:\n    entries:\n      - vehicle: DB_BR422\n\nplayer_service:\n  formation: player_train\n  start_location: Essen_Hbf_P5\n  destination: Bochum_Hbf_P3\n\nai_services:\n  - id: ai_regional_01\n    consist: DB_BR422\n    start_location: Bochum_Hbf_P3\n    destination: Essen_Hbf_P5\n    departure_time: \"08:05\"\n\nobjectives:\n  - id: stop_bochum\n    description: Reach Bochum Hbf\n    kind: stop_at\n    location: Bochum_Hbf_P3\n\ncompletion:\n  success:\n    - kind: all_objectives_completed\n  failure:\n    - kind: time_reached\n      time: \"09:00\"\n",
    );
    write_file(
        &workspace.join("profiles").join("tsw5").join("routes").join("rro.yaml"),
        "id: RRO\nname: Ruhr-Sieg Nord\nsupported_stock:\n  - DB_BR422\nspawn_points:\n  - Essen_Hbf_P5\n  - Bochum_Hbf_P3\ntemplates:\n  - commuter_simple\nsupported_weather:\n  - cloudy\ninstall_ids:\n  - RuhrSiegNord\ninstall_hints:\n  - RRO\n  - Ruhr-Sieg Nord\n",
    );
    write_file(
        &workspace
            .join("templates")
            .join("tsw5")
            .join("commuter_simple")
            .join("template.yaml"),
        "id: commuter_simple\nname: Commuter Simple\noutput_subdir: template\nrender_extensions:\n  - yaml\n",
    );
    write_file(
        &workspace
            .join("templates")
            .join("tsw5")
            .join("commuter_simple")
            .join("seed.yaml"),
        "scenario_id: {{meta.id}}\nroute: {{scenario.route}}\nplayer_ref: {{player_service.consist}}\nformations: {{counts.formations}}\nai_services: {{counts.ai_services}}\n",
    );

    let game_dir = workspace.join("fake_game").join("TS2Prototype").join("Content").join("DLC");
    fs::create_dir_all(&game_dir).expect("failed to create fake game directory");
    for pak_name in pak_names {
        write_file(&game_dir.join(pak_name), "");
    }

    workspace
}

#[test]
fn scan_command_writes_install_scan_cache() {
    let workspace = create_fixture_workspace(&["TS2Prototype-WindowsNoEditor-RuhrSiegNord.pak"]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    let output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("--game-dir")
        .arg(workspace.join("fake_game"))
        .arg("scan")
        .output()
        .expect("failed to run scan command");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let install_scan = workspace.join("install_scan.json");
    let scan_json = fs::read_to_string(install_scan).expect("failed to read install scan cache");
    assert!(scan_json.contains("TS2Prototype-WindowsNoEditor-RuhrSiegNord.pak"));
    assert!(scan_json.contains("\"canonical_dlc_id\": \"RuhrSiegNord\""));
}

#[test]
fn scan_command_json_reports_supported_and_unknown_dlcs() {
    let workspace = create_fixture_workspace(&[
        "TS2Prototype-WindowsNoEditor-RuhrSiegNord.pak",
        "TS2Prototype-WindowsNoEditor-SoutheasternHighSpeed.pak",
    ]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    let output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("--game-dir")
        .arg(workspace.join("fake_game"))
        .arg("scan")
        .arg("--json")
        .output()
        .expect("failed to run scan command");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"canonical_dlc_id\": \"RuhrSiegNord\""));
    assert!(stdout.contains("\"route_id\": \"RRO\""));
    assert!(stdout.contains("\"matched_by\": \"install_id\""));
    assert!(stdout.contains("\"support_level\": \"curated\""));
    assert!(stdout.contains("SoutheasternHighSpeed"));
}

#[test]
fn validate_command_uses_default_install_scan_cache() {
    let workspace = create_fixture_workspace(&["TS2Prototype-WindowsNoEditor-RuhrSiegNord.pak"]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    let scan_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("--game-dir")
        .arg(workspace.join("fake_game"))
        .arg("scan")
        .output()
        .expect("failed to run scan command");
    assert!(scan_output.status.success());

    let output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("validate")
        .arg(workspace.join("examples").join("scenario.yaml"))
        .output()
        .expect("failed to run validate command");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Validation succeeded"));
    assert!(stdout.contains("Install scan in use"));
}

#[test]
fn validate_command_fails_when_route_is_not_installed() {
    let workspace = create_fixture_workspace(&["TS2Prototype-WindowsNoEditor-SoutheasternHighSpeed.pak"]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    let output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("--game-dir")
        .arg(workspace.join("fake_game"))
        .arg("validate")
        .arg(workspace.join("examples").join("scenario.yaml"))
        .output()
        .expect("failed to run validate command");

    assert!(!output.status.success(), "validate should fail when route is not installed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ROUTE_NOT_INSTALLED"));
    assert!(stdout.contains("RuhrSiegNord"));
}

#[test]
fn build_command_generates_template_output_compiled_json_package_plan_and_staging() {
    let workspace = create_fixture_workspace(&["TS2Prototype-WindowsNoEditor-RuhrSiegNord.pak"]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    let output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("--game-dir")
        .arg(workspace.join("fake_game"))
        .arg("build")
        .arg(workspace.join("examples").join("scenario.yaml"))
        .arg("--clean")
        .arg("--package-namespace")
        .arg("ScenarioMods/CLI")
        .output()
        .expect("failed to run build command");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let generated_template = workspace
        .join("build")
        .join("cli_test_001")
        .join("template")
        .join("seed.yaml");
    let compiled_json = workspace
        .join("build")
        .join("cli_test_001")
        .join("compiled_scenario.json");
    let package_plan = workspace
        .join("build")
        .join("cli_test_001")
        .join("package_plan.json");
    let staged_template = workspace
        .join("build")
        .join("cli_test_001")
        .join("staging")
        .join("Content")
        .join("ScenarioMods")
        .join("CLI")
        .join("cli_test_001")
        .join("template")
        .join("seed.yaml");
    let rendered =
        fs::read_to_string(generated_template).expect("failed to read generated template file");
    let compiled =
        fs::read_to_string(compiled_json).expect("failed to read compiled scenario json");
    let plan = fs::read_to_string(package_plan).expect("failed to read package plan json");

    assert!(rendered.contains("scenario_id: cli_test_001"));
    assert!(rendered.contains("route: RRO"));
    assert!(rendered.contains("player_ref: formation:player_train"));
    assert!(rendered.contains("formations: 1"));
    assert!(rendered.contains("ai_services: 1"));
    assert!(compiled.contains("\"objective_count\": 1"));
    assert!(compiled.contains("\"ai_service_count\": 1"));
    assert!(compiled.contains("\"formation_count\": 1"));
    assert!(compiled.contains("\"id\": \"player_train\""));
    assert!(plan.contains("\"package_namespace\": \"ScenarioMods/CLI\""));
    assert!(plan.contains("/Content/ScenarioMods/CLI/cli_test_001/template/seed.yaml"));
    assert!(staged_template.exists());
}

#[test]
fn discover_route_command_writes_route_discovery_artifact() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    let route_dir = workspace
        .join("external_route")
        .join("Content")
        .join("Localization")
        .join("FrankfurtFulda")
        .join("en");
    fs::create_dir_all(&route_dir).expect("failed to create route locres dir");
    fs::write(
        route_dir.join("FrankfurtFulda.locres"),
        b"Frankfurt - Fulda\0Hanau Hbf Pl 6\0Fulda Platform 1\0",
    )
    .expect("failed to write route locres");

    let output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("discover-route")
        .arg("FrankfurtFulda")
        .arg("--route-dir")
        .arg(workspace.join("external_route"))
        .output()
        .expect("failed to run discover-route command");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let artifact = workspace
        .join("artifacts")
        .join("discovery")
        .join("frankfurtfulda.route_discovery.json");
    let location_catalog = workspace
        .join("artifacts")
        .join("discovery")
        .join("frankfurtfulda.location_catalog.json");
    let stock_catalog = workspace
        .join("artifacts")
        .join("discovery")
        .join("frankfurtfulda.stock_catalog.json");
    let formation_catalog = workspace
        .join("artifacts")
        .join("discovery")
        .join("frankfurtfulda.formation_catalog.json");
    let artifact_json = fs::read_to_string(&artifact).expect("failed to read route discovery artifact");
    let location_catalog_json =
        fs::read_to_string(&location_catalog).expect("failed to read location catalog artifact");
    let stock_catalog_json =
        fs::read_to_string(&stock_catalog).expect("failed to read stock catalog artifact");
    let formation_catalog_json =
        fs::read_to_string(&formation_catalog).expect("failed to read formation catalog artifact");

    assert!(artifact_json.contains("\"route_id\": \"FrankfurtFulda\""));
    assert!(artifact_json.contains("Hanau Hbf Pl 6"));
    assert!(artifact_json.contains("Fulda Platform 1"));
    assert!(location_catalog_json.contains("\"route_id\": \"FrankfurtFulda\""));
    assert!(location_catalog_json.contains("\"ungrouped_locations\""));
    assert!(location_catalog_json.contains("Hanau Hbf Pl 6"));
    assert!(location_catalog_json.contains("\"source_kind\""));
    assert!(location_catalog_json.contains("\"allowed_uses\""));
    assert!(stock_catalog_json.contains("\"route_id\": \"FrankfurtFulda\""));
    assert!(stock_catalog_json.contains("\"stock_count\": 0"));
    assert!(formation_catalog_json.contains("\"route_id\": \"FrankfurtFulda\""));
    assert!(formation_catalog_json.contains("\"formation_count\": 0"));
}

#[test]
fn list_locations_command_filters_by_usage_from_location_catalog() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.location_catalog.json"),
        "{\n  \"schema_version\": 2,\n  \"route_id\": \"RRO\",\n  \"stations\": [\n    {\n      \"id\": \"essen_hbf\",\n      \"display_name\": \"Essen Hbf\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"player_spawn\"],\n      \"tags\": [\"Essen_Hbf_P5\"],\n      \"frontend_spawn_points\": [\n        {\n          \"tag\": \"Essen_Hbf_P5\",\n          \"display_name\": \"Essen Hbf\",\n          \"available_in_frontend\": true,\n          \"available_in_fast_travel\": true,\n          \"source\": \"test\"\n        }\n      ],\n      \"locations\": [\n        {\n          \"id\": \"essen_hbf_pl_5\",\n          \"display_name\": \"Essen Hbf Pl 5\",\n          \"kind\": \"platform\",\n          \"source_kind\": \"timetable\",\n          \"confidence\": \"high\",\n          \"allowed_uses\": [\"service_start\", \"service_end\", \"objective_location\"],\n          \"source\": \"test\"\n        }\n      ]\n    }\n  ],\n  \"ungrouped_locations\": [\n    {\n      \"id\": \"depot_track_1\",\n      \"display_name\": \"Depot Track 1\",\n      \"kind\": \"track\",\n      \"source_kind\": \"timetable\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"service_start\", \"service_end\"],\n      \"source\": \"test\"\n    },\n    {\n      \"id\": \"objective_only_marker\",\n      \"display_name\": \"Objective Only Marker\",\n      \"kind\": \"location\",\n      \"source_kind\": \"scenario\",\n      \"confidence\": \"medium\",\n      \"allowed_uses\": [\"objective_location\"],\n      \"source\": \"test\"\n    }\n  ],\n  \"summary\": {\n    \"station_count\": 1,\n    \"frontend_spawn_point_count\": 1,\n    \"grouped_location_count\": 1,\n    \"ungrouped_location_count\": 2\n  }\n}\n",
    );

    let player_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("list-locations")
        .arg("--usage")
        .arg("player_spawn")
        .output()
        .expect("failed to run list-locations player_spawn");
    assert!(player_output.status.success());
    let player_stdout = String::from_utf8_lossy(&player_output.stdout);
    assert!(player_stdout.contains("Locations for route 'RRO' and usage 'player_spawn'"));
    assert!(player_stdout.contains("Essen Hbf"));
    assert!(player_stdout.contains("Essen_Hbf_P5"));
    assert!(!player_stdout.contains("Depot Track 1"));
    assert!(!player_stdout.contains("Objective Only Marker"));

    let service_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("list-locations")
        .arg("--usage")
        .arg("service")
        .output()
        .expect("failed to run list-locations service");
    assert!(service_output.status.success());
    let service_stdout = String::from_utf8_lossy(&service_output.stdout);
    assert!(service_stdout.contains("Essen Hbf Pl 5"));
    assert!(service_stdout.contains("Depot Track 1"));
    assert!(!service_stdout.contains("Objective Only Marker"));

    let objective_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("list-locations")
        .arg("--usage")
        .arg("objective")
        .output()
        .expect("failed to run list-locations objective");
    assert!(objective_output.status.success());
    let objective_stdout = String::from_utf8_lossy(&objective_output.stdout);
    assert!(objective_stdout.contains("Essen Hbf Pl 5"));
    assert!(objective_stdout.contains("Depot Track 1"));
    assert!(objective_stdout.contains("Objective Only Marker"));
}
#[test]
fn show_location_command_lists_candidates_from_location_catalog() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.location_catalog.json"),
        "{\n  \"schema_version\": 2,\n  \"route_id\": \"RRO\",\n  \"stations\": [\n    {\n      \"id\": \"essen_hbf\",\n      \"display_name\": \"Essen Hbf\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"player_spawn\"],\n      \"tags\": [\"Essen_Hbf_P5\"],\n      \"frontend_spawn_points\": [\n        {\n          \"tag\": \"Essen_Hbf_P5\",\n          \"display_name\": \"Essen Hbf\",\n          \"available_in_frontend\": true,\n          \"available_in_fast_travel\": true,\n          \"source\": \"test\"\n        }\n      ],\n      \"locations\": []\n    }\n  ],\n  \"ungrouped_locations\": [\n    {\n      \"id\": \"shared_track_a\",\n      \"display_name\": \"Shared Track\",\n      \"kind\": \"track\",\n      \"source_kind\": \"timetable\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"service_start\", \"service_end\", \"objective_location\"],\n      \"internal_ref\": \"RIBBON_A\",\n      \"source\": \"test\"\n    },\n    {\n      \"id\": \"shared_track_b\",\n      \"display_name\": \"Shared Track\",\n      \"kind\": \"track\",\n      \"source_kind\": \"scenario\",\n      \"confidence\": \"medium\",\n      \"allowed_uses\": [\"service_start\", \"service_end\", \"objective_location\"],\n      \"internal_ref\": \"RIBBON_B\",\n      \"source\": \"test\"\n    }\n  ],\n  \"summary\": {\n    \"station_count\": 1,\n    \"frontend_spawn_point_count\": 1,\n    \"grouped_location_count\": 0,\n    \"ungrouped_location_count\": 2\n  }\n}\n",
    );

    let output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("show-location")
        .arg("Shared Track")
        .arg("--usage")
        .arg("service")
        .output()
        .expect("failed to run show-location command");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Location matches for route 'RRO' and usage 'service': 2"));
    assert!(stdout.contains("Selected candidate:"));
    assert!(stdout.contains("shared_track_a"));
    assert!(stdout.contains("internal_ref=RIBBON_A"));
    assert!(stdout.contains("shared_track_b"));
    assert!(stdout.contains("internal_ref=RIBBON_B"));
}

#[test]
fn validate_command_uses_route_discovery_artifact_to_extend_profile() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace.join("examples").join("scenario.yaml"),
        "meta:\n  id: cli_discovery_001\n  title: CLI Discovery Test\n  author: Dario\n\nscenario:\n  route: RRO\n  template: commuter_simple\n  start_time: \"08:15\"\n  weather: cloudy\n\nplayer_service:\n  consist: DISCOVERED_DB_BR420\n  start_location: Dortmund Hbf Pl 16\n  destination: Hagen Hbf Pl 3\n\nobjectives:\n  - id: stop_hagen\n    description: Reach Hagen Hbf\n    kind: stop_at\n    location: Hagen Hbf Pl 3\n\ncompletion:\n  success:\n    - kind: all_objectives_completed\n",
    );

    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.route_discovery.json"),
        "{\n  \"schema_version\": 1,\n  \"route_id\": \"RRO\",\n  \"route_titles\": [\"Ruhr-Sieg Nord\"],\n  \"route_definitions\": [],\n  \"locations\": [\n    {\n      \"id\": \"dortmund_hbf_pl_16\",\n      \"display_name\": \"Dortmund Hbf Pl 16\",\n      \"kind\": \"platform\",\n      \"source\": \"test\"\n    },\n    {\n      \"id\": \"hagen_hbf_pl_3\",\n      \"display_name\": \"Hagen Hbf Pl 3\",\n      \"kind\": \"platform\",\n      \"source\": \"test\"\n    }\n  ],\n  \"stock\": [\n    {\n      \"id\": \"DISCOVERED_DB_BR420\",\n      \"source\": \"test\",\n      \"evidence\": \"test\"\n    }\n  ],\n  \"sources\": [\"test\"],\n  \"summary\": {\n    \"source_count\": 1,\n    \"route_title_count\": 1,\n    \"route_definition_count\": 0,\n    \"location_count\": 2,\n    \"stock_count\": 1\n  }\n}\n",
    );

    let output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("validate")
        .arg(workspace.join("examples").join("scenario.yaml"))
        .output()
        .expect("failed to run validate command with route discovery");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Validation succeeded"));
    assert!(stdout.contains("Route discovery in use"));
}
#[test]
fn validate_command_warns_when_location_catalog_match_is_ambiguous() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace.join("examples").join("scenario.yaml"),
        "meta:\n  id: cli_ambiguous_001\n  title: CLI Ambiguous Test\n  author: Dario\n\nscenario:\n  route: RRO\n  template: commuter_simple\n  start_time: \"08:15\"\n  weather: cloudy\n\nplayer_service:\n  consist: DB_BR422\n  start_location: Essen_Hbf_P5\n  destination: Shared Track\n\nobjectives:\n  - id: reach_shared_track\n    description: Reach the shared track\n    kind: stop_at\n    location: Shared Track\n\ncompletion:\n  success:\n    - kind: all_objectives_completed\n",
    );

    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.location_catalog.json"),
        "{\n  \"schema_version\": 2,\n  \"route_id\": \"RRO\",\n  \"stations\": [\n    {\n      \"id\": \"essen_hbf\",\n      \"display_name\": \"Essen Hbf\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"player_spawn\"],\n      \"tags\": [\"Essen_Hbf_P5\"],\n      \"frontend_spawn_points\": [\n        {\n          \"tag\": \"Essen_Hbf_P5\",\n          \"display_name\": \"Essen Hbf\",\n          \"available_in_frontend\": true,\n          \"available_in_fast_travel\": true,\n          \"source\": \"test\"\n        }\n      ],\n      \"locations\": []\n    }\n  ],\n  \"ungrouped_locations\": [\n    {\n      \"id\": \"shared_track_a\",\n      \"display_name\": \"Shared Track\",\n      \"kind\": \"track\",\n      \"source_kind\": \"timetable\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"service_start\", \"service_end\", \"objective_location\"],\n      \"internal_ref\": \"RIBBON_A\",\n      \"source\": \"test\"\n    },\n    {\n      \"id\": \"shared_track_b\",\n      \"display_name\": \"Shared Track\",\n      \"kind\": \"track\",\n      \"source_kind\": \"scenario\",\n      \"confidence\": \"medium\",\n      \"allowed_uses\": [\"service_start\", \"service_end\", \"objective_location\"],\n      \"internal_ref\": \"RIBBON_B\",\n      \"source\": \"test\"\n    }\n  ],\n  \"summary\": {\n    \"station_count\": 1,\n    \"frontend_spawn_point_count\": 1,\n    \"grouped_location_count\": 0,\n    \"ungrouped_location_count\": 2\n  }\n}\n",
    );

    let output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("validate")
        .arg(workspace.join("examples").join("scenario.yaml"))
        .output()
        .expect("failed to run validate command with ambiguous location catalog");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("AMBIGUOUS_LOCATION"));
    assert!(stdout.contains("player_service.destination"));
    assert!(stdout.contains("Shared Track"));
    assert!(stdout.contains("shared_track_a"));
    assert!(stdout.contains("RIBBON_A"));
}

#[test]
fn validate_and_build_commands_accept_structured_location_selectors() {
    let workspace = create_fixture_workspace(&["TS2Prototype-WindowsNoEditor-RuhrSiegNord.pak"]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace.join("examples").join("scenario.yaml"),
        "meta:\n  id: cli_structured_001\n  title: CLI Structured Selector Test\n  author: Dario\n\nscenario:\n  route: RRO\n  template: commuter_simple\n  start_time: \"08:15\"\n  weather: cloudy\n\nplayer_service:\n  consist: DB_BR422\n  start_location:\n    name: Essen Hbf\n    spawn_tag: Essen_Hbf_P5\n  destination:\n    name: Shared Track\n    internal_ref: RIBBON_B\n\nobjectives:\n  - id: reach_shared_track\n    description: Reach the shared track\n    kind: stop_at\n    location:\n      name: Shared Track\n      internal_ref: RIBBON_B\n\ncompletion:\n  success:\n    - kind: all_objectives_completed\n",
    );

    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.location_catalog.json"),
        "{\n  \"schema_version\": 2,\n  \"route_id\": \"RRO\",\n  \"stations\": [\n    {\n      \"id\": \"essen_hbf\",\n      \"display_name\": \"Essen Hbf\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"player_spawn\"],\n      \"tags\": [\"Essen_Hbf_P5\"],\n      \"frontend_spawn_points\": [\n        {\n          \"tag\": \"Essen_Hbf_P5\",\n          \"display_name\": \"Essen Hbf\",\n          \"available_in_frontend\": true,\n          \"available_in_fast_travel\": true,\n          \"source\": \"test\"\n        }\n      ],\n      \"locations\": []\n    }\n  ],\n  \"ungrouped_locations\": [\n    {\n      \"id\": \"shared_track_a\",\n      \"display_name\": \"Shared Track\",\n      \"kind\": \"track\",\n      \"source_kind\": \"timetable\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"service_start\", \"service_end\", \"objective_location\"],\n      \"internal_ref\": \"RIBBON_A\",\n      \"source\": \"test\"\n    },\n    {\n      \"id\": \"shared_track_b\",\n      \"display_name\": \"Shared Track\",\n      \"kind\": \"track\",\n      \"source_kind\": \"scenario\",\n      \"confidence\": \"medium\",\n      \"allowed_uses\": [\"service_start\", \"service_end\", \"objective_location\"],\n      \"internal_ref\": \"RIBBON_B\",\n      \"source\": \"test\"\n    }\n  ],\n  \"summary\": {\n    \"station_count\": 1,\n    \"frontend_spawn_point_count\": 1,\n    \"grouped_location_count\": 0,\n    \"ungrouped_location_count\": 2\n  }\n}\n",
    );

    let validate_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("validate")
        .arg(workspace.join("examples").join("scenario.yaml"))
        .output()
        .expect("failed to run validate command with structured selectors");

    assert!(
        validate_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&validate_output.stdout),
        String::from_utf8_lossy(&validate_output.stderr)
    );

    let validate_stdout = String::from_utf8_lossy(&validate_output.stdout);
    assert!(validate_stdout.contains("Validation succeeded"));
    assert!(!validate_stdout.contains("AMBIGUOUS_LOCATION"));
    assert!(!validate_stdout.contains("INVALID_LOCATION_SELECTOR"));

    let build_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("build")
        .arg(workspace.join("examples").join("scenario.yaml"))
        .arg("--clean")
        .output()
        .expect("failed to run build command with structured selectors");

    assert!(
        build_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&build_output.stdout),
        String::from_utf8_lossy(&build_output.stderr)
    );

    let compiled_json = fs::read_to_string(
        workspace
            .join("build")
            .join("cli_structured_001")
            .join("compiled_scenario.json"),
    )
    .expect("failed to read compiled scenario json");
    assert!(compiled_json.contains("\"catalog_id\": \"shared_track_b\""));
    assert!(compiled_json.contains("\"internal_ref\": \"RIBBON_B\""));
    assert!(!compiled_json.contains("\"ambiguous\": true"));
}

#[test]
fn validate_command_fails_when_structured_location_selector_is_invalid() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace.join("examples").join("scenario.yaml"),
        "meta:\n  id: cli_invalid_selector_001\n  title: CLI Invalid Selector Test\n  author: Dario\n\nscenario:\n  route: RRO\n  template: commuter_simple\n  start_time: \"08:15\"\n  weather: cloudy\n\nplayer_service:\n  consist: DB_BR422\n  start_location: Essen_Hbf_P5\n  destination:\n    name: Shared Track\n    internal_ref: DOES_NOT_EXIST\n\ncompletion:\n  success:\n    - kind: all_objectives_completed\n",
    );

    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.location_catalog.json"),
        "{\n  \"schema_version\": 2,\n  \"route_id\": \"RRO\",\n  \"stations\": [\n    {\n      \"id\": \"essen_hbf\",\n      \"display_name\": \"Essen Hbf\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"player_spawn\"],\n      \"tags\": [\"Essen_Hbf_P5\"],\n      \"frontend_spawn_points\": [\n        {\n          \"tag\": \"Essen_Hbf_P5\",\n          \"display_name\": \"Essen Hbf\",\n          \"available_in_frontend\": true,\n          \"available_in_fast_travel\": true,\n          \"source\": \"test\"\n        }\n      ],\n      \"locations\": []\n    }\n  ],\n  \"ungrouped_locations\": [\n    {\n      \"id\": \"shared_track_a\",\n      \"display_name\": \"Shared Track\",\n      \"kind\": \"track\",\n      \"source_kind\": \"timetable\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"service_start\", \"service_end\", \"objective_location\"],\n      \"internal_ref\": \"RIBBON_A\",\n      \"source\": \"test\"\n    }\n  ],\n  \"summary\": {\n    \"station_count\": 1,\n    \"frontend_spawn_point_count\": 1,\n    \"grouped_location_count\": 0,\n    \"ungrouped_location_count\": 1\n  }\n}\n",
    );

    let output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("validate")
        .arg(workspace.join("examples").join("scenario.yaml"))
        .output()
        .expect("failed to run validate command with invalid structured selector");

    assert!(!output.status.success(), "validate should fail with an invalid explicit selector");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("INVALID_LOCATION_SELECTOR"));
    assert!(stdout.contains("player_service.destination"));
    assert!(stdout.contains("Shared Track"));
}


#[test]
fn list_stock_and_show_stock_commands_use_stock_catalog() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.stock_catalog.json"),
        "{\n  \"schema_version\": 1,\n  \"route_id\": \"RRO\",\n  \"stock\": [\n    {\n      \"id\": \"RVD_DB_BR422\",\n      \"plugin\": \"RRO_Route\",\n      \"source\": \"TS2Prototype/Plugins/DLC/RRO_Route/Content/Timetable/RRO_Timetable.json\",\n      \"evidence\": \"RailVehicleDefinition'RVD_DB_BR422'\"\n    },\n    {\n      \"id\": \"RVD_DB_BR185\",\n      \"plugin\": \"RRO_Freight\",\n      \"source\": \"TS2Prototype/Plugins/DLC/RRO_Freight/Content/Data/RVD/RVD_DB_BR185.json\",\n      \"evidence\": \"RailVehicleDefinition'RVD_DB_BR185'\"\n    }\n  ],\n  \"summary\": {\n    \"stock_count\": 2,\n    \"plugin_count\": 2,\n    \"source_count\": 2\n  }\n}\n",
    );

    let list_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("list-stock")
        .output()
        .expect("failed to run list-stock command");

    assert!(
        list_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&list_output.stdout),
        String::from_utf8_lossy(&list_output.stderr)
    );

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(list_stdout.contains("Stock for route 'RRO': 2"));
    assert!(list_stdout.contains("RVD_DB_BR422"));
    assert!(list_stdout.contains("RVD_DB_BR185"));

    let show_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("show-stock")
        .arg("BR422")
        .output()
        .expect("failed to run show-stock command");

    assert!(
        show_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&show_output.stdout),
        String::from_utf8_lossy(&show_output.stderr)
    );

    let show_stdout = String::from_utf8_lossy(&show_output.stdout);
    assert!(show_stdout.contains("Stock matches for route 'RRO': 1"));
    assert!(show_stdout.contains("RVD_DB_BR422"));
    assert!(show_stdout.contains("plugin=RRO_Route"));
    assert!(show_stdout.contains("RRO_Timetable.json"));
}
#[test]
fn list_formations_and_show_formation_commands_use_formation_catalog() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.formation_catalog.json"),
        "{\n  \"schema_version\": 1,\n  \"route_id\": \"RRO\",\n  \"formations\": [\n    {\n      \"id\": \"FRM_RRO_DB_BR422_Pair\",\n      \"plugin\": \"RRO_Route\",\n      \"source\": \"TS2Prototype/Plugins/DLC/RRO_Route/Content/Data/Formations/FRM_RRO_DB_BR422_Pair.json\",\n      \"entries\": [\n        {\n          \"index\": 0,\n          \"vehicle_id\": \"RVD_DB_BR422_A\",\n          \"flipped\": false,\n          \"cargo_loaded\": false\n        },\n        {\n          \"index\": 1,\n          \"vehicle_id\": \"RVD_DB_BR422_B\",\n          \"flipped\": true,\n          \"cargo_loaded\": false\n        }\n      ],\n      \"drivable\": true\n    },\n    {\n      \"id\": \"RRO_PlayerFormation\",\n      \"plugin\": \"RRO_Gameplay\",\n      \"source\": \"TS2Prototype/Plugins/DLC/RRO_Gameplay/Content/Scenarios/ScA/Formations/RRO_PlayerFormation.json\",\n      \"entries\": [\n        {\n          \"index\": 0,\n          \"formation_id\": \"FRM_RRO_DB_BR422_Pair\",\n          \"cargo_asset\": \"/Game/Core/Assets/Cargo/Passenger.Passenger\",\n          \"cargo_units\": 120,\n          \"flipped\": false,\n          \"cargo_loaded\": true\n        }\n      ],\n      \"drivable\": true\n    }\n  ],\n  \"summary\": {\n    \"formation_count\": 2,\n    \"plugin_count\": 2,\n    \"source_count\": 2\n  }\n}\n",
    );

    let list_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("list-formations")
        .output()
        .expect("failed to run list-formations command");

    assert!(
        list_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&list_output.stdout),
        String::from_utf8_lossy(&list_output.stderr)
    );

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(list_stdout.contains("Formations for route 'RRO': 2"));
    assert!(list_stdout.contains("FRM_RRO_DB_BR422_Pair"));
    assert!(list_stdout.contains("RRO_PlayerFormation"));

    let show_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("show-formation")
        .arg("PlayerFormation")
        .output()
        .expect("failed to run show-formation command");

    assert!(
        show_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&show_output.stdout),
        String::from_utf8_lossy(&show_output.stderr)
    );

    let show_stdout = String::from_utf8_lossy(&show_output.stdout);
    assert!(show_stdout.contains("Formation matches for route 'RRO': 1"));
    assert!(show_stdout.contains("RRO_PlayerFormation"));
    assert!(show_stdout.contains("plugin=RRO_Gameplay"));
    assert!(show_stdout.contains("Flattened vehicles:"));
    assert!(show_stdout.contains("RVD_DB_BR422_A"));
    assert!(show_stdout.contains("RVD_DB_BR422_B, flipped=true"));
    assert!(show_stdout.contains("cargo=/Game/Core/Assets/Cargo/Passenger.Passenger"));
}

#[test]
fn show_service_ref_command_prints_yaml_snippets_for_stock_and_formations() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.stock_catalog.json"),
        "{\n  \"schema_version\": 1,\n  \"route_id\": \"RRO\",\n  \"stock\": [\n    {\n      \"id\": \"RVD_DB_BR422\",\n      \"plugin\": \"RRO_Route\",\n      \"source\": \"TS2Prototype/Plugins/DLC/RRO_Route/Content/Timetable/RRO_Timetable.json\",\n      \"evidence\": \"RailVehicleDefinition'RVD_DB_BR422'\"\n    }\n  ],\n  \"summary\": {\n    \"stock_count\": 1,\n    \"plugin_count\": 1,\n    \"source_count\": 1\n  }\n}\n",
    );
    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.formation_catalog.json"),
        "{\n  \"schema_version\": 1,\n  \"route_id\": \"RRO\",\n  \"formations\": [\n    {\n      \"id\": \"RRO_PlayerFormation\",\n      \"plugin\": \"RRO_Gameplay\",\n      \"source\": \"TS2Prototype/Plugins/DLC/RRO_Gameplay/Content/Scenarios/ScA/Formations/RRO_PlayerFormation.json\",\n      \"entries\": [\n        {\n          \"index\": 0,\n          \"vehicle_id\": \"RVD_DB_BR422_A\",\n          \"flipped\": false,\n          \"cargo_loaded\": false\n        }\n      ],\n      \"drivable\": true\n    }\n  ],\n  \"summary\": {\n    \"formation_count\": 1,\n    \"plugin_count\": 1,\n    \"source_count\": 1\n  }\n}\n",
    );

    let stock_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("show-service-ref")
        .arg("BR422")
        .arg("--kind")
        .arg("stock")
        .output()
        .expect("failed to run show-service-ref stock command");

    assert!(
        stock_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&stock_output.stdout),
        String::from_utf8_lossy(&stock_output.stderr)
    );

    let stock_stdout = String::from_utf8_lossy(&stock_output.stdout);
    assert!(stock_stdout.contains("Service reference matches for route 'RRO': 1"));
    assert!(stock_stdout.contains("Kind: stock"));
    assert!(stock_stdout.contains("YAML snippet:"));
    assert!(stock_stdout.contains("consist:"));
    assert!(stock_stdout.contains("id: RVD_DB_BR422"));
    assert!(stock_stdout.contains("plugin: RRO_Route"));

    let formation_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("show-service-ref")
        .arg("PlayerFormation")
        .arg("--kind")
        .arg("formation")
        .output()
        .expect("failed to run show-service-ref formation command");

    assert!(
        formation_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&formation_output.stdout),
        String::from_utf8_lossy(&formation_output.stderr)
    );

    let formation_stdout = String::from_utf8_lossy(&formation_output.stdout);
    assert!(formation_stdout.contains("Service reference matches for route 'RRO': 1"));
    assert!(formation_stdout.contains("Kind: formation"));
    assert!(formation_stdout.contains("YAML snippet:"));
    assert!(formation_stdout.contains("formation_ref:"));
    assert!(formation_stdout.contains("id: RRO_PlayerFormation"));
    assert!(formation_stdout.contains("plugin: RRO_Gameplay"));
}

#[test]
fn init_scenario_command_generates_valid_yaml_from_catalog_defaults() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.location_catalog.json"),
        "{\n  \"schema_version\": 2,\n  \"route_id\": \"RRO\",\n  \"route_overview\": {\n    \"display_name\": \"Ruhr-Sieg Nord\",\n    \"stat_tracking_name\": \"RuhrSiegNord\",\n    \"start_point\": \"Essen\",\n    \"end_point\": \"Bochum\",\n    \"country\": \"Deutschland\",\n    \"level_asset\": \"/RRO/Map/RROMap.RROMap\",\n    \"route_map_widget\": \"/RRO/UI/RRORouteMapWidget.RRORouteMapWidget_C\"\n  },\n  \"stations\": [\n    {\n      \"id\": \"essen_hbf\",\n      \"display_name\": \"Essen Hbf\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"player_spawn\"],\n      \"tags\": [\"Essen_Hbf_P5\"],\n      \"frontend_spawn_points\": [\n        {\n          \"tag\": \"Essen_Hbf_P5\",\n          \"display_name\": \"Essen Hbf P5\",\n          \"available_in_frontend\": true,\n          \"available_in_fast_travel\": true,\n          \"source\": \"TS2Prototype/Plugins/DLC/RRO/Content/RouteDefinition/RRORouteDefinition.json\"\n        }\n      ],\n      \"locations\": [\n        {\n          \"id\": \"bochum_hbf_p3\",\n          \"display_name\": \"Bochum Hbf P3\",\n          \"kind\": \"platform\",\n          \"source_kind\": \"timetable\",\n          \"confidence\": \"high\",\n          \"allowed_uses\": [\"service_start\", \"service_end\", \"objective_location\"],\n          \"internal_ref\": \"REF-BOCHUM-P3\",\n          \"source\": \"TS2Prototype/Plugins/DLC/RRO_Gameplay/Content/Timetable/RRO_Timetable.json\"\n        }\n      ]\n    }\n  ],\n  \"ungrouped_locations\": [],\n  \"summary\": {\n    \"station_count\": 1,\n    \"frontend_spawn_point_count\": 1,\n    \"grouped_location_count\": 1,\n    \"ungrouped_location_count\": 0\n  }\n}\n",
    );
    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.formation_catalog.json"),
        "{\n  \"schema_version\": 1,\n  \"route_id\": \"RRO\",\n  \"formations\": [\n    {\n      \"id\": \"RRO_PlayerFormation\",\n      \"plugin\": \"RRO_Gameplay\",\n      \"source\": \"TS2Prototype/Plugins/DLC/RRO_Gameplay/Content/Scenarios/ScA/Formations/RRO_PlayerFormation.json\",\n      \"entries\": [\n        {\n          \"index\": 0,\n          \"vehicle_id\": \"RVD_DB_BR422_A\",\n          \"flipped\": false,\n          \"cargo_loaded\": false\n        }\n      ],\n      \"drivable\": true\n    }\n  ],\n  \"summary\": {\n    \"formation_count\": 1,\n    \"plugin_count\": 1,\n    \"source_count\": 1\n  }\n}\n",
    );

    let output_path = workspace.join("examples").join("generated_init.yaml");
    let init_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("init-scenario")
        .arg("--output")
        .arg(&output_path)
        .arg("--scenario-id")
        .arg("generated_init")
        .arg("--title")
        .arg("Generated Init")
        .arg("--author")
        .arg("Codex")
        .output()
        .expect("failed to run init-scenario command");

    assert!(
        init_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&init_output.stdout),
        String::from_utf8_lossy(&init_output.stderr)
    );

    let generated = fs::read_to_string(&output_path).expect("failed to read generated scenario");
    assert!(generated.contains("id: generated_init"));
    assert!(generated.contains("route: RRO"));
    assert!(generated.contains("formation_ref:"));
    assert!(generated.contains("id: RRO_PlayerFormation"));
    assert!(generated.contains("start_location:"));
    assert!(generated.contains("catalog_id:") || generated.contains("spawn_tag:"));
    assert!(generated.contains("internal_ref: REF-BOCHUM-P3"));

    let validate_output = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("validate")
        .arg(&output_path)
        .output()
        .expect("failed to validate generated scenario");

    assert!(
        validate_output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&validate_output.stdout),
        String::from_utf8_lossy(&validate_output.stderr)
    );
}

#[test]
fn init_scenario_command_supports_guided_prompt_flow() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.location_catalog.json"),
        "{\n  \"schema_version\": 2,\n  \"route_id\": \"RRO\",\n  \"route_overview\": {\n    \"display_name\": \"Ruhr-Sieg Nord\",\n    \"stat_tracking_name\": \"RuhrSiegNord\",\n    \"start_point\": \"Essen\",\n    \"end_point\": \"Bochum\",\n    \"country\": \"Deutschland\",\n    \"level_asset\": \"/RRO/Map/RROMap.RROMap\",\n    \"route_map_widget\": \"/RRO/UI/RRORouteMapWidget.RRORouteMapWidget_C\"\n  },\n  \"stations\": [\n    {\n      \"id\": \"essen_hbf\",\n      \"display_name\": \"Essen Hbf\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"player_spawn\"],\n      \"tags\": [\"Essen_Hbf_P5\"],\n      \"frontend_spawn_points\": [\n        {\n          \"tag\": \"Essen_Hbf_P5\",\n          \"display_name\": \"Essen Hbf P5\",\n          \"available_in_frontend\": true,\n          \"available_in_fast_travel\": true,\n          \"source\": \"TS2Prototype/Plugins/DLC/RRO/Content/RouteDefinition/RRORouteDefinition.json\"\n        }\n      ],\n      \"locations\": [\n        {\n          \"id\": \"bochum_hbf_p3\",\n          \"display_name\": \"Bochum Hbf P3\",\n          \"kind\": \"platform\",\n          \"source_kind\": \"timetable\",\n          \"confidence\": \"high\",\n          \"allowed_uses\": [\"service_start\", \"service_end\", \"objective_location\"],\n          \"internal_ref\": \"REF-BOCHUM-P3\",\n          \"source\": \"TS2Prototype/Plugins/DLC/RRO_Gameplay/Content/Timetable/RRO_Timetable.json\"\n        }\n      ]\n    }\n  ],\n  \"ungrouped_locations\": [],\n  \"summary\": {\n    \"station_count\": 1,\n    \"frontend_spawn_point_count\": 1,\n    \"grouped_location_count\": 1,\n    \"ungrouped_location_count\": 0\n  }\n}\n",
    );
    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.formation_catalog.json"),
        "{\n  \"schema_version\": 1,\n  \"route_id\": \"RRO\",\n  \"formations\": [\n    {\n      \"id\": \"RRO_PlayerFormation\",\n      \"plugin\": \"RRO_Gameplay\",\n      \"source\": \"TS2Prototype/Plugins/DLC/RRO_Gameplay/Content/Scenarios/ScA/Formations/RRO_PlayerFormation.json\",\n      \"entries\": [\n        {\n          \"index\": 0,\n          \"vehicle_id\": \"RVD_DB_BR422_A\",\n          \"flipped\": false,\n          \"cargo_loaded\": false\n        }\n      ],\n      \"drivable\": true\n    }\n  ],\n  \"summary\": {\n    \"formation_count\": 1,\n    \"plugin_count\": 1,\n    \"source_count\": 1\n  }\n}\n",
    );

    let output_path = workspace.join("examples").join("guided_init.yaml");
    let mut child = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("init-scenario")
        .arg("--output")
        .arg(&output_path)
        .arg("--force")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn guided init-scenario command");

    {
        let stdin = child.stdin.as_mut().expect("stdin not available");
        stdin
            .write_all(b"guided_init\nGuided Init\nCodex\nEssen Hbf P5\nBochum Hbf P3\n1\nn\ny\n")
            .expect("failed to write guided answers");
    }

    let output = child
        .wait_with_output()
        .expect("failed to wait for guided init-scenario");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Scenario summary:"));
    assert!(stdout.contains("Write scenario file?"));
    assert!(stdout.contains("spawn_tag=Essen_Hbf_P5"));
    assert!(stdout.contains("internal_ref=REF-BOCHUM-P3"));
    assert!(stdout.contains("plugin=RRO_Gameplay"));
    assert!(stdout.contains("source=TS2Prototype/Plugins/DLC/RRO_Gameplay/Content/Scenarios/ScA/Formations/RRO_PlayerFormation.json"));

    let generated = fs::read_to_string(&output_path).expect("failed to read guided scenario");
    assert!(generated.contains("id: guided_init"));
    assert!(generated.contains("title: Guided Init"));
    assert!(generated.contains("author: Codex"));
    assert!(generated.contains("formation_ref:"));
    assert!(generated.contains("name: Bochum Hbf P3"));
    assert!(!generated.contains("ai_services:"));
}

#[test]
fn init_scenario_command_can_add_guided_ai_service() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace.join("artifacts").join("discovery").join("rro.location_catalog.json"),
        "{\n  \"schema_version\": 2,\n  \"route_id\": \"RRO\",\n  \"route_overview\": {\n    \"display_name\": \"Ruhr-Sieg Nord\",\n    \"stat_tracking_name\": \"RuhrSiegNord\",\n    \"start_point\": \"Essen\",\n    \"end_point\": \"Bochum\",\n    \"country\": \"Deutschland\",\n    \"level_asset\": \"/RRO/Map/RROMap.RROMap\",\n    \"route_map_widget\": \"/RRO/UI/RRORouteMapWidget.RRORouteMapWidget_C\"\n  },\n  \"stations\": [\n    {\n      \"id\": \"essen_hbf\",\n      \"display_name\": \"Essen Hbf\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"player_spawn\"],\n      \"tags\": [\"Essen_Hbf_P5\"],\n      \"frontend_spawn_points\": [\n        {\n          \"tag\": \"Essen_Hbf_P5\",\n          \"display_name\": \"Essen Hbf P5\",\n          \"available_in_frontend\": true,\n          \"available_in_fast_travel\": true,\n          \"source\": \"TS2Prototype/Plugins/DLC/RRO/Content/RouteDefinition/RRORouteDefinition.json\"\n        }\n      ],\n      \"locations\": [\n        {\n          \"id\": \"bochum_hbf_p3\",\n          \"display_name\": \"Bochum Hbf P3\",\n          \"kind\": \"platform\",\n          \"source_kind\": \"timetable\",\n          \"confidence\": \"high\",\n          \"allowed_uses\": [\"service_start\", \"service_end\", \"objective_location\"],\n          \"internal_ref\": \"REF-BOCHUM-P3\",\n          \"source\": \"TS2Prototype/Plugins/DLC/RRO_Gameplay/Content/Timetable/RRO_Timetable.json\"\n        }\n      ]\n    }\n  ],\n  \"ungrouped_locations\": [],\n  \"summary\": {\n    \"station_count\": 1,\n    \"frontend_spawn_point_count\": 1,\n    \"grouped_location_count\": 1,\n    \"ungrouped_location_count\": 0\n  }\n}\n",
    );
    write_file(
        &workspace.join("artifacts").join("discovery").join("rro.formation_catalog.json"),
        "{\n  \"schema_version\": 1,\n  \"route_id\": \"RRO\",\n  \"formations\": [\n    {\n      \"id\": \"RRO_PlayerFormation\",\n      \"plugin\": \"RRO_Gameplay\",\n      \"source\": \"TS2Prototype/Plugins/DLC/RRO_Gameplay/Content/Scenarios/ScA/Formations/RRO_PlayerFormation.json\",\n      \"entries\": [{\"index\": 0, \"vehicle_id\": \"RVD_DB_BR422_A\", \"flipped\": false, \"cargo_loaded\": false}],\n      \"drivable\": true\n    }\n  ],\n  \"summary\": {\n    \"formation_count\": 1,\n    \"plugin_count\": 1,\n    \"source_count\": 1\n  }\n}\n",
    );

    let output_path = workspace.join("examples").join("guided_ai_init.yaml");
    let mut child = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("init-scenario")
        .arg("--output")
        .arg(&output_path)
        .arg("--force")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn guided ai init-scenario command");

    {
        let stdin = child.stdin.as_mut().expect("stdin not available");
        stdin
            .write_all(b"guided_ai_init\nGuided AI Init\nCodex\nEssen Hbf P5\nBochum Hbf P3\n1\ny\nBochum Hbf P3\nEssen Hbf P5\n1\n08:05\ny\n")
            .expect("failed to write guided ai answers");
    }

    let output = child.wait_with_output().expect("failed to wait for guided ai init-scenario");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(&output_path).expect("failed to read guided ai scenario");
    assert!(generated.contains("ai_services:"));
    assert!(generated.contains("id: ai_service_01"));
    assert!(generated.contains("departure_time: \"08:05\"") || generated.contains("departure_time: 08:05"));
}

#[test]
fn init_scenario_summary_allows_editing_before_write() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.location_catalog.json"),
        "{\n  \"schema_version\": 2,\n  \"route_id\": \"RRO\",\n  \"route_overview\": {\n    \"display_name\": \"Ruhr-Sieg Nord\",\n    \"stat_tracking_name\": \"RuhrSiegNord\",\n    \"start_point\": \"Essen\",\n    \"end_point\": \"Bochum\",\n    \"country\": \"Deutschland\",\n    \"level_asset\": \"/RRO/Map/RROMap.RROMap\",\n    \"route_map_widget\": \"/RRO/UI/RRORouteMapWidget.RRORouteMapWidget_C\"\n  },\n  \"stations\": [\n    {\n      \"id\": \"essen_hbf\",\n      \"display_name\": \"Essen Hbf\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"player_spawn\"],\n      \"tags\": [\"Essen_Hbf_P5\"],\n      \"frontend_spawn_points\": [\n        {\n          \"tag\": \"Essen_Hbf_P5\",\n          \"display_name\": \"Essen Hbf P5\",\n          \"available_in_frontend\": true,\n          \"available_in_fast_travel\": true,\n          \"source\": \"TS2Prototype/Plugins/DLC/RRO/Content/RouteDefinition/RRORouteDefinition.json\"\n        }\n      ],\n      \"locations\": [\n        {\n          \"id\": \"bochum_hbf_p3\",\n          \"display_name\": \"Bochum Hbf P3\",\n          \"kind\": \"platform\",\n          \"source_kind\": \"timetable\",\n          \"confidence\": \"high\",\n          \"allowed_uses\": [\"service_start\", \"service_end\", \"objective_location\"],\n          \"internal_ref\": \"REF-BOCHUM-P3\",\n          \"source\": \"TS2Prototype/Plugins/DLC/RRO_Gameplay/Content/Timetable/RRO_Timetable.json\"\n        }\n      ]\n    }\n  ],\n  \"ungrouped_locations\": [],\n  \"summary\": {\n    \"station_count\": 1,\n    \"frontend_spawn_point_count\": 1,\n    \"grouped_location_count\": 1,\n    \"ungrouped_location_count\": 0\n  }\n}\n",
    );
    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.formation_catalog.json"),
        "{\n  \"schema_version\": 1,\n  \"route_id\": \"RRO\",\n  \"formations\": [\n    {\n      \"id\": \"RRO_PlayerFormation\",\n      \"plugin\": \"RRO_Gameplay\",\n      \"source\": \"TS2Prototype/Plugins/DLC/RRO_Gameplay/Content/Scenarios/ScA/Formations/RRO_PlayerFormation.json\",\n      \"entries\": [{\"index\": 0, \"vehicle_id\": \"RVD_DB_BR422_A\", \"flipped\": false, \"cargo_loaded\": false}],\n      \"drivable\": true\n    }\n  ],\n  \"summary\": {\n    \"formation_count\": 1,\n    \"plugin_count\": 1,\n    \"source_count\": 1\n  }\n}\n",
    );

    let output_path = workspace.join("examples").join("guided_edit_init.yaml");
    let mut child = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("init-scenario")
        .arg("--output")
        .arg(&output_path)
        .arg("--force")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn guided edit init-scenario command");

    {
        let stdin = child.stdin.as_mut().expect("stdin not available");
        stdin
            .write_all(b"guided_edit_init\nOriginal Title\nCodex\nEssen Hbf P5\nBochum Hbf P3\n1\nn\nn\n1\nRetitled Scenario\n8\ny\n")
            .expect("failed to write guided edit answers");
    }

    let output = child
        .wait_with_output()
        .expect("failed to wait for guided edit init-scenario");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("What do you want to edit?"));
    assert!(stdout.contains("Scenario summary:"));
    assert!(stdout.contains("Back to summary"));

    let generated = fs::read_to_string(&output_path).expect("failed to read guided edited scenario");
    assert!(generated.contains("title: Retitled Scenario"));
}

#[test]
fn init_scenario_summary_allows_multiple_edits_before_returning() {
    let workspace = create_fixture_workspace(&[]);
    let binary = env!("CARGO_BIN_EXE_tswtool");

    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.location_catalog.json"),
        "{\n  \"schema_version\": 2,\n  \"route_id\": \"RRO\",\n  \"route_overview\": {\n    \"display_name\": \"Ruhr-Sieg Nord\",\n    \"stat_tracking_name\": \"RuhrSiegNord\",\n    \"start_point\": \"Essen\",\n    \"end_point\": \"Bochum\",\n    \"country\": \"Deutschland\",\n    \"level_asset\": \"/RRO/Map/RROMap.RROMap\",\n    \"route_map_widget\": \"/RRO/UI/RRORouteMapWidget.RRORouteMapWidget_C\"\n  },\n  \"stations\": [\n    {\n      \"id\": \"essen_hbf\",\n      \"display_name\": \"Essen Hbf\",\n      \"confidence\": \"high\",\n      \"allowed_uses\": [\"player_spawn\"],\n      \"tags\": [\"Essen_Hbf_P5\"],\n      \"frontend_spawn_points\": [\n        {\n          \"tag\": \"Essen_Hbf_P5\",\n          \"display_name\": \"Essen Hbf P5\",\n          \"available_in_frontend\": true,\n          \"available_in_fast_travel\": true,\n          \"source\": \"TS2Prototype/Plugins/DLC/RRO/Content/RouteDefinition/RRORouteDefinition.json\"\n        }\n      ],\n      \"locations\": [\n        {\n          \"id\": \"bochum_hbf_p3\",\n          \"display_name\": \"Bochum Hbf P3\",\n          \"kind\": \"platform\",\n          \"source_kind\": \"timetable\",\n          \"confidence\": \"high\",\n          \"allowed_uses\": [\"service_start\", \"service_end\", \"objective_location\"],\n          \"internal_ref\": \"REF-BOCHUM-P3\",\n          \"source\": \"TS2Prototype/Plugins/DLC/RRO_Gameplay/Content/Timetable/RRO_Timetable.json\"\n        }\n      ]\n    }\n  ],\n  \"ungrouped_locations\": [],\n  \"summary\": {\n    \"station_count\": 1,\n    \"frontend_spawn_point_count\": 1,\n    \"grouped_location_count\": 1,\n    \"ungrouped_location_count\": 0\n  }\n}\n",
    );
    write_file(
        &workspace
            .join("artifacts")
            .join("discovery")
            .join("rro.formation_catalog.json"),
        "{\n  \"schema_version\": 1,\n  \"route_id\": \"RRO\",\n  \"formations\": [\n    {\n      \"id\": \"RRO_PlayerFormation\",\n      \"plugin\": \"RRO_Gameplay\",\n      \"source\": \"TS2Prototype/Plugins/DLC/RRO_Gameplay/Content/Scenarios/ScA/Formations/RRO_PlayerFormation.json\",\n      \"entries\": [{\"index\": 0, \"vehicle_id\": \"RVD_DB_BR422_A\", \"flipped\": false, \"cargo_loaded\": false}],\n      \"drivable\": true\n    }\n  ],\n  \"summary\": {\n    \"formation_count\": 1,\n    \"plugin_count\": 1,\n    \"source_count\": 1\n  }\n}\n",
    );

    let output_path = workspace.join("examples").join("guided_multi_edit_init.yaml");
    let mut child = Command::new(binary)
        .current_dir(&workspace)
        .arg("--project-root")
        .arg(&workspace)
        .arg("init-scenario")
        .arg("--output")
        .arg(&output_path)
        .arg("--force")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn guided multi-edit init-scenario command");

    {
        let stdin = child.stdin.as_mut().expect("stdin not available");
        stdin
            .write_all(b"guided_multi_edit\nOriginal Title\nOriginal Author\nEssen Hbf P5\nBochum Hbf P3\n1\nn\nn\n1\nRetitled Scenario\n2\nRetitled Author\n8\ny\n")
            .expect("failed to write guided multi-edit answers");
    }

    let output = child
        .wait_with_output()
        .expect("failed to wait for guided multi-edit init-scenario");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let generated =
        fs::read_to_string(&output_path).expect("failed to read guided multi-edit scenario");
    assert!(generated.contains("title: Retitled Scenario"));
    assert!(generated.contains("author: Retitled Author"));
}
