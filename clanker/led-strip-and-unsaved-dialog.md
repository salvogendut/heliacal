# LED strip and unsaved-dialog refactor (2026-11-07)

- Matched Prismatic's overlay flow: no Save/Discard rows in any tab; F9 opens
  or saves; Escape on a dirty draft opens Save/Discard/Cancel with Cancel
  selected by default.
- Added the bottom RUN/CPU/HD/CD LED strip and moved shortcut hints above it.
  The guest area shrinks by 16 (hints) + 22 (LEDs) logical pixels. LED pulses
  are frontend-only and expire after 150 ms.
- `../helios/src/devices/scsi.rs` now exposes per-target completed read/write
  counters through `ScsiTarget::activity()`. Counters are incremented only
  after successful transfers, survive SCSI bus reset, reset when a target is
  reattached, and do not affect guest-visible state. Core test:
  `activity_counts_completed_transfers_without_changing_target_state`.
- `heliacal/src/activity.rs` maps CPU to bit 0 and read/write activity to two
  bits per SCSI ID; `emulator.rs` publishes the counters in `Snapshot`.
- Smoke test now exercises the unsaved dialog: default Cancel keeps the draft,
  Discard closes it, and a subsequent save still persists targets.

Verified with heliacal fmt/clippy/15 tests/`make smoke`, a release boot smoke
(`97062912 steps, PC=ffd0c4d0`), and helios fmt-check/clippy/tests/all demos.
