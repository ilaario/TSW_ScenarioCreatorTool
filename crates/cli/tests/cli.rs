use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
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
        "meta:\n  id: cli_test_001\n  title: CLI Test\n  author: Dario\n\nscenario:\n  route: RRO\n  template: commuter_simple\n  start_time: \"08:15\"\n  weather: cloudy\n\nplayer_service:\n  consist: DB_BR422\n  start_location: Essen_Hbf_P5\n  destination: Bochum_Hbf_P3\n\nai_services:\n  - id: ai_regional_01\n    consist: DB_BR422\n    start_location: Bochum_Hbf_P3\n    destination: Essen_Hbf_P5\n    departure_time: \"08:05\"\n\nobjectives:\n  - id: stop_bochum\n    description: Reach Bochum Hbf\n    kind: stop_at\n    location: Bochum_Hbf_P3\n\ncompletion:\n  success:\n    - kind: all_objectives_completed\n  failure:\n    - kind: time_reached\n      time: \"09:00\"\n",
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
        "scenario_id: {{meta.id}}\nroute: {{scenario.route}}\nai_services: {{counts.ai_services}}\n",
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
    assert!(rendered.contains("ai_services: 1"));
    assert!(compiled.contains("\"objective_count\": 1"));
    assert!(compiled.contains("\"ai_service_count\": 1"));
    assert!(plan.contains("\"package_namespace\": \"ScenarioMods/CLI\""));
    assert!(plan.contains("/Content/ScenarioMods/CLI/cli_test_001/template/seed.yaml"));
    assert!(staged_template.exists());
}