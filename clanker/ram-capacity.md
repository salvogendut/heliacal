# Configurable RAM frontend handoff

Status update: implementation and acceptance are complete; see the
[completion review](ram-capacity-completion.md). The original handoff below
records the earlier checkpoint.

Prepared 2026-11-15 for [Helios issue #3](https://github.com/salvogendut/helios/issues/3).

The authoritative implementation plan is
[Helios: configurable RAM plan](../../helios/clanker/ram-capacity-plan.md).
Both repositories are on the pushed `feat/configurable-ram` branch. Heliacal's
implementation base is `ddafd879ce4301f030b5f78e7d3ca48963ac996f`; the accepted
baseline has 34 tests. This handoff does not claim new RAM acceptance.

Implement the core topology/DMA work before exposing larger sizes here. In
particular, an SS-20 with its native SX/VSIMM and an SS-20 with only CGsix have
different supported RAM configurations. Use the core's shared validation and
choice list instead of duplicating its hardware rules in the UI.

Frontend work is slice C of the shared plan:

- Reuse `Settings.ram_mib` and `NamedMachine.ram_mib`, retaining the 128 MiB
  default. Validate active and inactive named machines without allocating RAM.
- Add RAM to `Settings::hardware_eq`. Pass the actual selected size and display
  topology to core construction in `emulator::build`; it currently allocates
  constant `RAM_MIB` regardless of the settings field.
- Make F9 General's existing Memory row selectable. Preserve draft/save/discard
  behavior and the existing confirmed cold-replacement flow. Keep running-machine
  labels truthful while replacement is pending, and preserve the old machine on
  construction failure.
- Add `--ram-mib` with checked conversion and consistent named-machine/CLI override
  precedence. Invalid configuration/CLI values must fail without silent rounding.
- Keep each named machine's NVRAM identity stable across RAM changes; update help,
  About, examples and README's fixed-size claims.
- Test actual worker allocation, named recall, draft transitions, serialization,
  replacement detection and error behavior. Rebuild against the final core, run
  the checks in `AGENTS.md`, SDL smoke and the shared firmware/OS acceptance matrix.

Use temporary configs, NVRAM and disk clones for acceptance. The shared plan lists
source media and cleanup requirements. Leave the user's live catalog intact.
Report results and remaining limits in this repository's `clanker/` notes and
index; do not equate a larger status label or display smoke with working RAM.
