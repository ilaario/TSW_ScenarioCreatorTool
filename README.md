# TSW Scenario Tool

`tswtool` is a CLI-first Rust workspace for validating and building Train Sim World scenario mods from a simple YAML file.

Current scope:

- parse scenario YAML
- load game, route, and template profiles
- scan a local TSW installation for installed DLC `.pak` files
- map canonical DLC ids to supported route profiles
- validate the configuration
- generate a build directory from a template scaffold
- emit a compiled JSON representation
- generate a package staging layout and package plan for future `.pak` packaging

Still out of scope:

- Unreal asset generation
- `.uasset` parsing
- `.pak` packaging itself
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

Scan a local TSW installation and write `install_scan.json`:

```bash
cargo run -- scan --game-dir "C:/Games/Train Sim World 6"
```

Validate using a live scan of the game installation:

```bash
cargo run -- validate examples/rro_example.yaml --game-dir "C:/Games/Train Sim World 6"
```

Validate using a previously saved install scan cache:

```bash
cargo run -- validate examples/rro_example.yaml --install-scan install_scan.json
```

Build a scenario:

```bash
cargo run -- build examples/rro_example.yaml
```

Build and clean the previous output first:

```bash
cargo run -- build examples/rro_example.yaml --clean
```

Build with a custom package namespace for staging:

```bash
cargo run -- build examples/rro_example.yaml --clean --package-namespace ScenarioMods/RRO
```

Emit the validation report as JSON:

```bash
cargo run -- validate examples/rro_example.yaml --json
```

Emit the enriched install scan report as JSON:

```bash
cargo run -- scan --game-dir "C:/Games/Train Sim World 6" --json
```

## Install Scanning

The scanner is a read-only discovery layer for local DLC content.

It currently:

- recursively scans a Train Sim World installation for `.pak` files
- derives a `canonical_dlc_id` from pack names such as `TS2Prototype-WindowsNoEditor-RuhrSiegNord.pak`
- stores a machine-readable cache in `install_scan.json`
- matches canonical DLC ids against supported route profiles
- lets `validate` and `build` check whether the requested route appears to be installed

Important design note:

- profiles remain the source of truth for supported stock, spawn points, templates, and game compatibility
- install scanning answers "what seems to be installed on this machine?"
- a DLC can be installed but still unsupported by the tool if no profile exists yet

If `install_scan.json` exists in the project root, `validate` and `build` will load it automatically unless `--game-dir` or `--install-scan` is provided explicitly.

## Route Profiles And Canonical DLC Ids

Route profiles now distinguish between canonical game ids and user-facing aliases.

Example:

```yaml
id: RRO
name: Ruhr-Sieg Nord

install_ids:
  - RuhrSiegNord

install_hints:
  - RRO
  - Ruhr-Sieg Nord
```

Recommended usage:

- `install_ids`: exact canonical DLC identifiers derived from `.pak` names
- `install_hints`: human-friendly aliases and abbreviations used as fallback matching terms

Matching order during validation is:

1. canonical `install_ids`
2. alias fallback using `id`, `name`, and `install_hints`

## Enriched Scan Output

`scan --json` now emits an enriched report that includes:

- the raw install scan cache payload
- `matched_routes`, for canonical DLC ids that match known route profiles
- `unknown_canonical_dlcs`, for canonical DLC ids found in the install but not yet mapped to a route profile

Each matched route also reports a `support_level`: `curated` when the profile has stock, spawn point, and template data, or `discovery_only` when the profile currently exists just for DLC recognition.

This makes it easier to see names such as:

- `RuhrSiegNord -> RRO`
- `SoutheasternHighSpeed -> unknown`

## Scenario YAML Example

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

ai_services:
  - id: ai_regional_01
    consist: DB_BR422
    start_location: Bochum_Hbf_P3
    destination: Essen_Hbf_P5
    departure_time: "08:05"

objectives:
  - id: stop_bochum
    description: Reach Bochum Hbf platform 3
    kind: stop_at
    location: Bochum_Hbf_P3
  - id: arrive_on_time
    description: Arrive before 08:45
    kind: arrive_by
    time: "08:45"

completion:
  success:
    - kind: all_objectives_completed
  failure:
    - kind: time_reached
      time: "09:00"
```

## Build Output

A build now produces both tool artifacts and packaging-oriented artifacts:

```text
build/<scenario-id>/
|- compiled_scenario.json
|- manifest.json
|- package_plan.json
|- scenario.yaml
|- staging/
|  `- Content/
|     `- <package-namespace>/
|        `- <scenario-id>/
|           |- compiled_scenario.json
|           `- template/
|              |- README.md
|              `- scenario_blueprint.yaml
`- template/
   |- README.md
   `- scenario_blueprint.yaml
```

## Packaging Staging

`package_plan.json` is the next-level compiler output that bridges the current builder and a future `.pak` packer.

It records:

- the package namespace
- the suggested `.pak` filename
- the staging root
- the logical mount root
- every packable entry with source path, staged path, and mount path

The `staging/` directory mirrors what a future packaging step can consume directly.

## Tests

Run the full test suite with:

```bash
cargo test --workspace
```

## Crates

- `tsw-scenario-schema`: shared data structures, including route profile install ids, compiled scenario, and package plan types
- `tsw-scenario-core`: YAML, route profile, and template loading
- `tsw-scenario-scanner`: local game installation scanning, canonical DLC id derivation, and install scan cache loading/saving
- `tsw-scenario-validator`: semantic validation and structured reports
- `tsw-scenario-compiler`: template-based build generation, compiled JSON emission, and package staging
- `tswtool`: CLI entrypoint and enriched scan reporting