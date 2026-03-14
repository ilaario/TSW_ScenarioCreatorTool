# TSW Scenario Tool

`tswtool` is a CLI-first Rust workspace for authoring Train Sim World scenarios from a simple YAML file.

The current project focus is intentionally narrow:

- define scenarios in YAML
- validate them against curated route profiles
- compile them into a predictable build folder
- prepare structured artifacts for future packaging work

## Core Pipeline

```text
Scenario YAML
  -> Validator
  -> Template compiler
  -> Build folder
  -> (optional) package staging / .pak packaging later
```

The real MVP is still the configuration pipeline, not `.pak` generation.

## Current Capabilities

- scenario YAML parsing
- route profile loading
- template loading
- validation with structured `errors` and `warnings`
- template-based build output
- compiled intermediate JSON output
- optional install scan support
- optional route discovery artifacts derived from unpacked route data or FModel JSON exports
- package staging preparation for future packaging work

## Out Of Scope For Now

These remain intentionally deferred:

- Unreal asset generation
- `.uasset` writing
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
|  `- frankfurt_fulda_example.yaml
|- profiles/
|  `- tsw5/
|     `- routes/
|- templates/
|  `- tsw5/
|     `- commuter_simple/
`- artifacts/
   `- discovery/
```

## Scenario Example

```yaml
meta:
  id: ftf_br111_morning_001
  title: 111 Hanau to Frankfurt Morning Peak
  author: Dario

scenario:
  route: FrankfurtFulda
  template: commuter_simple
  start_time: "06:05"
  weather: cloudy

formations:
  br111_push_pull:
    entries:
      - vehicle: FTF_DB_BR111
      - vehicle: FTF_DB_NWagen
        count: 3
      - vehicle: FTF_DB_Bnrdzf_463
        flipped: true

player_service:
  formation: br111_push_pull
  start_location: Hanau Hbf Pl 6
  destination: Frankfurt (Main) Hbf Pl 2
```

## Commands

Validate the included example:

```bash
cargo run -- validate examples/frankfurt_fulda_example.yaml
```

Build the included example:

```bash
cargo run -- build examples/frankfurt_fulda_example.yaml
```

Build and clean the previous output first:

```bash
cargo run -- build examples/frankfurt_fulda_example.yaml --clean
```

Validate the catalog-backed Frankfurt Fulda example:

```bash
- `examples/frankfurt_fulda_example.yaml` for a realistic Frankfurt Fulda gameplay-style example
- `examples/frankfurt_fulda_catalog_formation_example.yaml` for a discovery-backed example that uses `formation_ref`
cargo run -- build examples/frankfurt_fulda_catalog_formation_example.yaml --clean
```

Emit the validation report as JSON:

```bash
cargo run -- validate examples/frankfurt_fulda_example.yaml --json
```

List locations for the only discovered route catalog in the project:

```bash
cargo run -- list-locations --usage player_spawn
cargo run -- list-locations --usage service
cargo run -- list-locations --usage objective
```

List locations for a specific route explicitly:

```bash
cargo run -- list-locations FrankfurtFulda --usage service
```

Inspect a single location query and see the ranked catalog candidates:

```bash
cargo run -- show-location "Frankfurt (Main) Hbf Pl 2" --usage service --route FrankfurtFulda
cargo run -- show-location "Hanau Hbf" --usage player_spawn
```

List discovered stock ids for the only discovered route catalog in the project:

```bash
cargo run -- list-stock
```

Inspect a stock id and see the discovered plugin/source evidence:

```bash
cargo run -- show-stock BR411 --route FrankfurtFulda
cargo run -- show-stock RVD_FTF_DB_BR411_TW_0 --route FrankfurtFulda --json
```

List discovered formation ids for a route:

```bash
cargo run -- list-formations FrankfurtFulda
```

Inspect a formation and see both its direct entries and the flattened vehicle list:

```bash
cargo run -- show-formation BR411 --route FrankfurtFulda
cargo run -- show-formation FTF_ScA_PlayerICET --route FrankfurtFulda --json
```

Generate an explicit YAML selector for authoring `consist` or `formation_ref`:

```bash
cargo run -- show-service-ref BR411 --kind stock --route FrankfurtFulda
cargo run -- show-service-ref FTF_ScA_PlayerICET --kind formation --route FrankfurtFulda
```

Generate a starter scenario YAML from the current route profile and discovery catalogs:

```bash
cargo run -- init-scenario --route FrankfurtFulda --scenario-id ftf_new_001 --author Dario
```

If you omit key fields, `init-scenario` switches to a guided CLI flow and prompts for start location, destination, service reference, and basic metadata.
The guided flow now also lets you choose between discovered `formation_ref` and `consist` references, and can optionally add a first AI service.
Before writing the file, the guided flow prints a summary and asks for final confirmation.
If you answer `no`, the wizard now lets you edit a section and then returns to the summary instead of aborting immediately.
You can keep editing multiple sections in that loop before choosing `Back to summary` and confirming the write.
The summary now includes technical selectors such as `spawn_tag`, `internal_ref`, `plugin`, and `source` when they are available.

When a name is ambiguous, the scenario YAML can pin the exact catalog entry explicitly:

```yaml
player_service:
  start_location:
    name: Hanau Hbf
    spawn_tag: Hanau Hbf
  destination:
    name: Frankfurt (Main) Hbf Pl 2
    internal_ref: 2FE008C5-4F8121B6-AD8F088A-695B7B33
```

Supported selector keys are:

- `name`: human-readable label used for matching and template rendering
- `catalog_id`: exact catalog entry id from `show-location --json`
- `spawn_tag`: frontend/player spawn tag from route discovery
- `internal_ref`: timetable/scenario internal reference such as a ribbon ref

Discovery-backed service references work the same way for rolling stock and formations:

```yaml
player_service:
  formation_ref:
    id: FRM_FTF_DB_BR411
    plugin: FTF_DB_BR411
  start_location:
    name: Hanau Hbf
    spawn_tag: Hanau Hbf
  destination:
    name: Fulda Platform 1
    internal_ref: 7AEE42F0-4C8F92AE-E4267099-250DBBF9
```

Service reference rules are:

- `consist`: direct rolling-stock id, optionally with `plugin` / `source` selectors
- `formation`: custom formation defined inside the same scenario YAML
- `formation_ref`: discovered formation from `formation_catalog.json`, optionally pinned with `plugin` / `source`
- exactly one of `consist`, `formation`, or `formation_ref` may be set for a service

Scan an installed TSW directory and save `install_scan.json`:

```bash
cargo run -- scan --game-dir "C:/Games/Train Sim World 6"
```

Discover route metadata from an unpacked route directory:

```bash
cargo run -- discover-route FrankfurtFulda --route-dir external_dlc/FrankfurtFulda/TS2Prototype/Plugins/DLC/FrankfurtFulda
```

Discover route metadata from FModel JSON exports:

```bash
cargo run -- discover-route BremenOldenburg --exports-dir "C:/Users/dadob/Desktop/Output/Exports/TS2Prototype/Plugins/DLC/BremenOldenburg_Route_Gameplay"
```

Emit route discovery as JSON to stdout while also saving the artifact:

```bash
cargo run -- discover-route FrankfurtFulda --route-dir external_dlc/FrankfurtFulda/TS2Prototype/Plugins/DLC/FrankfurtFulda --json
```

## Route Discovery Artifacts

`discover-route` writes a route discovery artifact by default to:

```text
artifacts/discovery/<route-id>.route_discovery.json
```

For example:

```text
artifacts/discovery/frankfurtfulda.route_discovery.json
artifacts/discovery/frankfurtfulda.location_catalog.json
artifacts/discovery/frankfurtfulda.stock_catalog.json
artifacts/discovery/frankfurtfulda.formation_catalog.json
```

The matching `location_catalog.json` groups candidate operational locations under frontend stations when possible, leaves the remaining items under `ungrouped_locations`, and annotates each grouped location with `source_kind`, `confidence`, and `allowed_uses` so the app can distinguish between player spawn points, timetable/service locations, scenario references, and low-confidence localization-only candidates.

The matching `stock_catalog.json` captures discovered rolling stock ids together with their source plugin/path evidence, so the CLI can list and inspect the real stock seen in exported DLC JSON without relying only on curated route profiles.

The matching `formation_catalog.json` captures discovered `TrainFormation` assets together with their direct entries, plugin/source evidence, and enough data to flatten nested formations into their final vehicle list for inspection.

A route discovery artifact can contain:

- route overview metadata (display name, endpoints, level asset, route map widget)
- frontend spawn points from `RouteDefinition`
- route definition ids
- candidate operational locations
- stock ids discovered from exports
- stock catalog entries with plugin/source evidence
- formation catalog entries with nested composition data
- source file evidence

`validate` and `build` auto-load the matching discovery artifact for the scenario route if it exists. When a matching `location_catalog.json` is present, the compiler also resolves user-facing location labels into real route references such as frontend `spawn_tag` values and operational `internal_ref` / `RibbonReference` identifiers. `validate` emits `AMBIGUOUS_LOCATION` warnings if the same label matches multiple catalog entries, and explicit selectors such as `spawn_tag`, `catalog_id`, or `internal_ref` fail with `INVALID_LOCATION_SELECTOR` if they do not resolve.

The artifact extends the curated route profile with additional:

- spawn point candidates
- frontend spawn points and station tags
- supported stock ids
- install hints from discovered route titles

Curated profiles remain the source of truth. Route discovery is an enrichment layer, not a replacement.

## Install Scan Notes

Install scanning answers "what seems to be installed on this machine?" but does not decide what the tool supports.

Route discovery answers "what metadata can we derive from unpacked content or exports?" but does not replace curated validation rules.

The intended layering is:

1. curated route profile
2. optional route discovery artifact
3. optional install scan

## Examples

The repository includes:

- `examples/frankfurt_fulda_example.yaml` for a realistic Frankfurt Fulda gameplay-style example
- `examples/frankfurt_fulda_catalog_formation_example.yaml` for a discovery-backed example that uses `formation_ref`

## Additional Docs

See:

- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)
- [Dovetail Questions](docs/DOVETAIL_QUESTIONS.md)


