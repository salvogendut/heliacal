# Configurable RAM: slice C implementation

Status update: implementation and acceptance are complete; see the
[completion review](ram-capacity-completion.md). The original handoff below
records the earlier checkpoint.

Implemented 2026-11-15 on `feat/configurable-ram`, after Helios slices A/B were
accepted. Changes are uncommitted. No live user catalog or source media was
modified.

## Implemented

- `Settings::validate_hardware` now delegates RAM topology to Helios's
  allocation-free `MachineProfile::ram_layout`; `Settings::valid_ram_mib`
  exposes the core's exact totals to F9.
- `NamedMachine` keeps its existing `ram_mib` field; old records default to
  128 MiB, and inactive labels are validated with their own model/display/RAM.
- `Settings::hardware_eq` includes RAM, so a saved RAM change follows the
  existing confirmed cold-replacement flow. RAM does not affect label identity
  or NVRAM paths.
- `emulator::build` reads the TurboGX ROM before allocation, selects the
  explicit `InstalledVideo` mode, allocates `settings.ram_bytes()`, and does not
  construct an SX machine temporarily when CGsix is selected.
- F9 General's Memory row is selectable. Model/display changes keep a still-valid
  RAM total or choose the largest valid total no greater than the old one (the
  minimum when none is lower), with a visible adjustment notice. Drafts remain
  independent and Save/Discard/Cancel retain their existing semantics.
- Added `--ram-mib MIB` with checked integer parsing and final model/display
  validation after named-machine selection. Invalid totals fail before worker
  startup or media attachment.
- Updated help, About, README, architecture notes and the example configuration.

## Validation

| Check | Result |
| --- | --- |
| Frontend unit tests | 42 passed |
| `cargo fmt -p heliacal --check` | Pass |
| Clippy `--locked --all-targets --no-deps -- -D warnings` | Pass |
| Release build `--locked --offline` | Pass |
| SDL dummy/software smoke | Pass |
| Original SS-5 firmware display smoke, 256 MiB | 96,702,464 steps; display output passed; not an OS-boot claim |

The SS-5 smoke used temporary config/NVRAM and the pinned local ROM/TurboGX
images. Full SS-20/SX 448 MiB and SS-20/Axil 512 MiB firmware/Solaris and Heliacal
interactive acceptance remain for slice D; this note does not claim them.
