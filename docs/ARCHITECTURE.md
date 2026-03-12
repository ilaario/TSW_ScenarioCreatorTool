# Architecture

## Product Positioning

The project should be treated as a scenario builder + validator for Train Sim World, not as a `.pak` tool.

That distinction matters because the first useful product is not "something that packages files." The first useful product is "something that helps authors define valid scenarios and produces a clean build artifact."

## Target Pipeline

```text
Scenario YAML
  -> Validator
  -> Template compiler
  -> Build folder
  -> Optional package staging / packaging later
```

The pipeline above should remain the main architectural spine for the project.

## Source Of Truth

The source of truth is split between:

- scenario YAML files authored by users
- curated route profiles under `profiles/`
- template definitions under `templates/`

Packaging, install scanning, and future Unreal-specific work are downstream concerns and should not redefine the data model.

## MVP Components

### 1. Scenario Schema

The minimum schema should stay small and easy to author:

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

ai_services:
  - id: ai_br111_return
    formation: br111_push_pull
    start_location: Frankfurt (Main) Hbf Pl 1
    destination: Hanau Hbf Pl 5
    departure_time: "06:12"

objectives:
  - id: stop_offenbach_hbf
    description: Call at Offenbach Hbf platform 2
    kind: stop_at
    location: Offenbach Hbf Pl 2
  - id: stop_frankfurt_sud
    description: Call at Frankfurt (Main) Sud platform 4
    kind: stop_at
    location: Frankfurt (Main) Sud Pl 4
  - id: arrive_hbf
    description: Arrive in Frankfurt before 06:42
    kind: arrive_by
    time: "06:42"

completion:
  success:
    - kind: all_objectives_completed
      description: Complete the BR111 stopping service on time
  failure:
    - kind: time_reached
      time: "06:50"
      description: Service delay exceeded the scenario window
```

Important principle:

- the MVP schema should optimize for clarity and validation, not exhaustiveness

Fields like AI services, objectives, and completion rules can exist as extensions, but they should not distract from the minimum valid authoring flow.

### 2. Route Profiles

Profiles tell the tool what is valid for a route.

Example:

```yaml
id: FFU
name: Frankfurt Fulda

supported_stock:
  - DB_BR111

spawn_points:
  - Frankfurt_(Main)_Hbf_Pl_2
  - Frankfurt_(Main)_Sud_Pl_4

templates:
  - commuter_simple
```

Profiles should answer:

- which route ids are known
- which rolling stock is allowed
- which spawn points are allowed
- which templates are available
- which weather or route-specific constraints apply

## Validator Responsibilities

The validator is the heart of the product.

At minimum, it should verify:

- route exists
- template is valid for that route
- consist is supported
- spawn points are valid

Expected output style:

```text
Validation OK
```

or:

```text
Error: unsupported consist
Error: unknown spawn point
```

The validator should be the place where the project earns trust. Clear error messages are more valuable than early packaging support.

## Builder Responsibilities

The builder should take a validated scenario and create a predictable build directory.

Minimum expected output:

```text
build/frankfurt_fulda_test_001/
  manifest.json
  scenario.yaml
```

At this stage the builder should not generate Unreal assets.

Its job is to:

- freeze validated scenario data into a build artifact
- render template files
- create a stable handoff point for later packaging

## Role Of Templates

Templates are not the product by themselves. They are reusable scaffolds applied after validation.

That means:

- templates should depend on validated scenario data
- validation should happen before template rendering
- template complexity can grow later without changing the main architecture

## Role Of Install Scanning

Install scanning is useful, but secondary.

It can help answer:

- does this machine appear to have the requested DLC installed?

It should not answer:

- is this route supported by the tool?
- is this scenario valid?

Those remain profile- and validator-level concerns.

## Non-Goals For This Phase

The following should stay explicitly out of scope until the core authoring loop is stable:

- real scenario asset generation
- `.pak` packaging as a primary feature
- mod installation workflow
- GUI work

## Practical Rule

If a proposed feature does not make the YAML -> validate -> build loop stronger, it is probably not part of the current priority path.
