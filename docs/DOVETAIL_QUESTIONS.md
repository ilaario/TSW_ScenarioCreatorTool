# Dovetail Questions

## Goal

When reaching out to Dovetail, the right ask is not "please provide modding tools."

The better ask is for technical guidance that helps keep the project compatible with the Train Sim World ecosystem.

## Positioning

Suggested framing:

> I am building a scenario authoring and validation workflow that starts from a simple scenario definition, validates it against curated route data, and only later considers packaging. I want to make sure the project respects the expected structure and limitations of Train Sim World scenario mods.

## Questions To Ask

### 1. Scenario Mod Structure

Ask for:

- the expected high-level structure of a scenario mod
- recommended file/folder conventions
- whether there are naming rules or required metadata conventions

### 2. DLC Dependencies

Ask for:

- how scenario dependencies on routes or rolling stock should be represented
- whether there are preferred identifiers for DLC references
- what compatibility assumptions should be avoided

### 3. Packaging Best Practices

Ask for:

- packaging guidelines that help a scenario mod behave correctly in their ecosystem
- best practices for namespace/layout choices
- any known pitfalls around packaging order, conflicts, or distribution

### 4. Technical Limits And Constraints

Ask for:

- limitations a scenario mod should respect
- content boundaries that should not be crossed
- compatibility concerns across game versions or DLC combinations

## What Not To Ask For

Avoid positioning the project as:

- a reverse engineering tool
- a `.pak` extraction utility
- a request for proprietary internals

Avoid asking for:

- private tools
- undocumented asset internals unless they voluntarily point to them
- anything that makes the project sound packaging-first

## Short Draft Message

```text
Hello,

I am building a small Train Sim World scenario authoring workflow focused on three steps:
1. define a scenario in a simple YAML-like format
2. validate it against curated route/profile data
3. compile it into a clean build output before any later packaging step

I would like to keep the project aligned with your ecosystem and avoid making incorrect assumptions.

Could you share any guidance on:
- the expected structure of scenario mods
- how DLC dependencies should be represented
- packaging best practices
- technical limitations or compatibility constraints we should respect

The goal is compatibility and good ecosystem behavior, not reverse engineering.

Thank you.
```
