# Roadmap

## Guiding Rule

Do not build a `.pak` tool first.

Build a trustworthy scenario builder + validator first, and only then extend it toward packaging and distribution.

## Now

Current priority:

1. keep the scenario schema small and stable
2. treat route profiles as curated validity data
3. make validator output clear and dependable
4. make builder output deterministic and easy to inspect
5. document the product as an authoring workflow, not as a packaging workflow

Practical meaning for this repository:

- README and docs should emphasize schema, profiles, validation, and build output
- install scanning should remain optional support
- packaging-related work should stay clearly secondary

## Next

Once the core loop is stable, the next useful layer is:

1. richer scenario templates
2. clearer manifest/build contracts
3. support for additional scenario sections such as AI services and objectives
4. stronger validation coverage for route-specific rules
5. better examples and authoring documentation

## Later

Only after the builder/validator path feels solid:

1. `.pak` packaging
2. packaging best practices and compatibility checks
3. mod installer workflow
4. GUI

## Decision Filter

Before adding new work, ask:

1. does this improve authoring a scenario YAML?
2. does this improve validation confidence?
3. does this improve the build folder handoff?

If the answer is no, it likely belongs in a later phase.

## Notes On Existing Repo Features

This repository already includes install scanning and package staging concepts.

That is acceptable as long as they are treated as:

- optional supporting capabilities
- not the architectural center of the project
- not the definition of the MVP
