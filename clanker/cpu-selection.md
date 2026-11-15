# Experimental multi-CPU selection

2026-11-14. Branch `feat/cpu-selection`, from `95b6e10`. Built and tested with
sibling Helios core `4e6ab81` (merged slice 3) for the original frontend
implementation. The subsequent slice-4 CLI and desktop acceptance is recorded
in the follow-up below; the library scheduler/device implementation is unchanged.

## Behavior

- Flat and named configuration stanzas accept `cpus = 1|2`, defaulting to one
  for old files. Session override: `--cpus`. Hardware validation uses Helios's
  topology API; SS-5 permits only one, SS-20/Axil one or two. Inactive named
  stanzas are also validated. Counts 0/3/4 and invalid types fail explicitly.
- F9 General exposes CPUs for SS-20/Axil and a fixed one on SS-5. Changing the
  draft model to SS-5 normalizes its count to one. Save/Discard and named
  recall preserve their existing transactional behavior. Changing count
  requires confirmed cold replacement and does not hot-add a CPU.
- Construction uses `Machine::with_profile_processors`. One worker owns the
  complete machine, using legacy `step` for UP and `step_scheduled` for SMP.
  It never advances devices separately for each processor. Runtime selection
  comes from the machine topology, not settings that may describe a pending
  replacement. Errors retain the hardware processor ID and fault PC/nPC;
  board DMA errors remain device errors.
- Published hardware descriptions show the actual machine count. Tinker
  shows CPU0/CPU2 PCs and per-module outcome totals. Totals include trap,
  interrupt and reset entry; they are not retired-instruction counters.
  Successful cold replacement clears them and failed replacement preserves
  both processors and their run/pause state.

Two CPUs remain marked experimental. Original Sun SS-20 2.25R and Axil 2.19
ROMs reached responsive serial prompts with banners identifying both CPUs
in independent disk-free core probes. Each performed one normal startup
software reset. The guest, not the harness, enabled and parked CPU 2 through
AER. Two-CPU Solaris and desktop acceptance remain the next milestone.

The SS-20 hardware supports two MBus modules, including dual-CPU models
listed in Sun's service manual. The owner confirmed Axil accepts the same
modules: both can reach four CPUs via two dual modules. That topology is not
implemented in Helios. Do not enable a four-CPU UI value until supported
module models, ownership, IDs and startup have been implemented and tested.
See the linked primary manuals in README and the Helios slice-4 work order.

## Validation

- 34 frontend tests pass, including named/legacy counts, model limits,
  independent drafts, two real CPU owners executing synthetic guest firmware
  on the worker, pause/resume, rejected replacement, return to UP, NVRAM
  save and CPU-2 error attribution. The test firmware releases CPU 2 using a
  guest STA to AER; it does not force the worker's arbitration from the host.
- Formatting, Clippy `--all-targets --no-deps -- -D warnings`, release build,
  and `make smoke` pass. The SDL smoke saves a two-CPU named machine and
  recalls the original one-CPU machine.
- Actual Xvfb/SDL window: F9 count changed to two, saved and reloaded; a
  one-CPU draft was discarded; F12/Left/Return Quit exited 0. Layout capture
  confirms the new row and hint fit without covering other controls.
- Eight bad CLI combinations were rejected before worker launch, including
  SS-5/two CPUs and four CPUs on SS-20.
- Documented disk-free display smoke with temporary config/NVRAM passes for
  SS-5/one CPU (96,174,080 outcomes), SS-20/two/native SX (53,637,120), and
  Axil/two/CGsix (156,243,968). Display smoke means rendered firmware output,
  not OS acceptance or a verified graphical prompt.
- Unchanged core checks pass: 601 unique normal and 603 feature tests, both
  Clippy configurations, formatting and four demos. Network tests require
  loopback socket permission in this environment.

Evidence is under sibling Helios's ignored
`local/smp-slice4-2026-11-14/heliacal/` (test/lint/build logs, CLI failures,
real-firmware smoke logs/captures, actual SDL panel capture). The headless
firmware traces and source-input hashes are in its parent directory. No source
ROM/NVRAM/disk was changed, and no writable disk was attached during this work.
The release binary was rebuilt against the core above. Existing installed
Solaris UP checkpoints remain historical; they were not rerun in this frontend
slice. Two-CPU guest work, storage persistence and desktop input are still open.


## 2026-11-15 acceptance follow-up

Helios slice 4 on `feat/sun4m-smp-startup` at `249d785` plus reviewed CLI
changes now boots Solaris 2.6 with both processors online on SS-20 and Axil.
The frontend remains `5c97099`; no frontend code change was needed for the
new desktop validation. The earlier open milestones above are historical.

SS-20/native-SX accepts graphical-console login, Xterm input, capture/release,
pause/resume, window dragging and clean halt. Axil/CGsix CDE accepts terminal
input, capture, CPU-bound work during filesystem I/O, clean halt, confirmed
F5 reset/reboot, checksum reread and host Quit. Independent Axil follow-up
also passes move, maximize/restore, expose, iconify/restore and held-Shift
focus-loss release, with guest geometry and exact pixel assertions.

The corrected core CLI/test tree has 605/607 passing tests and unchanged
validated source hashes. Keep two CPUs experimental: four CPUs, arbitrary
SMP operating systems and dedicated malloc/mmap stress remain unverified.
The user approved incremental integration of both reviewed branches on
2026-11-15; issue-state changes are separate from this acceptance record. See the [Helios review](../../helios/clanker/smp-slice4-review.md)
for exact scopes and evidence. Both private review disks and duplicate captures
were removed after clean shutdown; compact captures, assertions and logs remain.
