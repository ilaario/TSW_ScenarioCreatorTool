# TSW Scenario Tool

`tswtool` is a CLI-first Rust workspace for authoring Train Sim World scenarios from a simple YAML file.

The project direction is intentionally narrow:

- build a scenario builder + validator first
- treat `.pak` packaging as an optional final step
- keep route profiles as the source of truth for what is valid
- avoid centering the architecture around reverse engineering or packaging

## Core Pipeline

```text
Scenario YAML
  -> Validator
  -> Template compiler
  -> Build folder
  -> (optional) package staging / .pak packaging later
```

The real MVP is not a packaging tool. The real MVP is a reliable workflow for:

1. defining a scenario in YAML
2. validating it against curated route profiles
3. compiling it into a predictable build folder

## Current MVP Focus

The core product scope is:

1. Scenario schema
2. Route profiles
3. Validator
4. Builder

Minimal example:

```yaml
meta:
  id: rro_test_001
  title: Test Scenario
  author: Dario

scenario:
  route: RRO
  template: commuter_simple
  start_time: "08:15"
  weather: cloudy

player_service:
  consist: DB_BR422
  start_location: Essen_Hbf_P5
  destination: Bochum_Hbf_P3
```

Matching route profile example:

```yaml
id: RRO
name: Ruhr-Sieg Nord

supported_stock:
  - DB_BR422

spawn_points:
  - Essen_Hbf_P5
  - Bochum_Hbf_P3

templates:
  - commuter_simple
```

See:

- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)
- [Dovetail Questions](docs/DOVETAIL_QUESTIONS.md)

## Repository Status

The repository already contains code and data beyond the narrow MVP path. Today, the important distinction is:

- core path: schema, profiles, validation, build output
- optional support track: install scanning and package staging

Existing capabilities in the workspace include:

- scenario YAML parsing
- profile loading
- validation
- build directory generation from templates
- compiled JSON output
- optional install scan support
- package staging preparation for future packaging work

Those extra pieces should support the core workflow, not define it.

## Out Of Scope For Now

These are explicitly deferred until the validator/builder workflow is solid:

- Unreal asset generation
- `.uasset` parsing
- `.pak` packaging
- mod installer
- GUI

## Workspace Layout

```text
tsw-scenario-tool/
|- Cargo.toml
|- crates/
|  |- cli/
|  |- compiler/
|  |- core/
|  |- scanner/
|  |- schema/
|  `- validator/
|- docs/
|  |- ARCHITECTURE.md
|  |- DOVETAIL_QUESTIONS.md
|  `- ROADMAP.md
|- examples/
|  `- rro_example.yaml
|- profiles/
|  `- tsw5/
|     `- routes/
|        `- rro.yaml
`- templates/
   `- tsw5/
      `- commuter_simple/
         |- README.md
         |- scenario_blueprint.yaml
         `- template.yaml
```

## Commands

Validate a scenario:

```bash
cargo run -- validate examples/rro_example.yaml
```

Build a scenario:

```bash
cargo run -- build examples/rro_example.yaml
```

Build and clean the previous output first:

```bash
cargo run -- build examples/rro_example.yaml --clean
```

Emit the validation report as JSON:

```bash
cargo run -- validate examples/rro_example.yaml --json
```

Optional install scan support:

```bash
cargo run -- scan --game-dir "C:/Games/Train Sim World 6"
```

Optional validation using a live scan:

```bash
cargo run -- validate examples/rro_example.yaml --game-dir "C:/Games/Train Sim World 6"
```

Optional validation using a saved install scan cache:

```bash
cargo run -- validate examples/rro_example.yaml --install-scan install_scan.json
```

## Design Notes

- Profiles remain the source of truth for supported stock, spawn points, templates, and game compatibility.
- Install scanning answers "what seems to be installed on this machine?" but does not decide what the tool supports.
- Packaging is a downstream concern and should not become the primary architecture driver.
