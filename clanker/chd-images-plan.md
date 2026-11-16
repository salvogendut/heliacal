# CHD frontend work: issue #6 handoff

Prepared 2026-11-15 on local `feat/chd-images`, based on Heliacal `8651356`.
The paired Helios branch starts at `f62911f`. This is the original plan.
Implementation and release rebuild now exist; see
[implementation progress](chd-images-progress.md). The slice-A-only directions
below are historical.
The full specification and ordered review stops are in
[Helios's CHD plan](../../helios/clanker/chd-images-plan.md).

Start with core slice A, then writable media and creation APIs. Frontend work
begins at slice D, after their contracts have been reviewed. Baseline: 69
Heliacal tests, release and smoke/guest acceptance described in the clock note.

## Concrete integration points

- `src/main.rs` already lists chd in MEDIA_FILTERS. Listing an extension does
  not establish decoding support. Update CLI help when the backend is ready.
- `src/settings.rs::validate_files` currently requires host file length to be
  a sector multiple, and opens writable disk paths with OpenOptions. Use the
  core's read-only inspection and logical capacity instead. A compressed
  parent's read-only host permission does not determine whether its child is
  writable. Save/inspection must never create a child or modify guest data.
- `src/emulator.rs::build` constructs media on the worker before publishing a
  replacement machine. Continue using core attachment APIs for raw/CHD and
  retain the running machine/endpoint if construction fails. Do not invent a
  frontend-only decoder or move a non-Send CHD handle between threads.
- `src/overlay.rs` owns the independent SCSI draft. Add an explicit New disk
  action for a selected target: destination, format, whole-MiB capacity. Start
  with the core's new uncompressed CHD helper and a 2048 MiB suggested size.
- `src/profiles.rs` and the existing target paths preserve named-machine
  recall. Format detection should not require a new configuration field if
  the same path already identifies the medium unambiguously.

## User-visible behavior

A new disk has no partitions, Sun label or filesystem; Solaris format/install
prepares it. Creation refuses an existing destination or conflicting sidecar.
Report exact logical capacity, not compressed file size. Explain that writes
to a compressed parent live in its persistent `.diff.chd` companion. Moving
or backing up that disk requires keeping both files. No automatic merge or
sidecar deletion on F5, shutdown or Quit is part of this milestone.

Keep Advanced Disk access semantics: read-only guests see any existing valid
child without modifying it; writable guests use the core's documented policy.
Bad parents/children must produce useful errors and leave prior media intact.
Do not turn a read-only request into a new writable overlay.

Creation runs outside the SDL event loop with bounded progress/results and a
responsive cancel path. Prismatic's `src/disk_image.rs` documents why some
native compression cancellation needs a helper process; inspect the actual
library behavior before choosing a thread or subprocess. The initial
uncompressed blank path may avoid lengthy compression entirely. A helper,
if needed, must start before SDL/config locking and must not launch chdman
through a shell or download software at runtime.

Before successful publication, cancellation/error cleans only owned temporary
files. After publication, the new disk is a user artifact: Discard cancels the
configuration draft, not the completed file. Successful creation fills only
that draft SCSI slot. Save/Discard/Cancel, confirmed F5 and failed replacement
retain their established behavior. Labels and NVRAM identity do not change.

## Regression and acceptance checklist

- Shared inspection rejects corrupt/wrong-type CHDs and handles renamed CHDs,
  unsupported layouts and feature-off core builds explicitly.
- Save is read-only; read-only mode, relative paths, named-machine roundtrips
  and preservation of unrelated targets work. A compressed parent is not
  rejected by raw file-length checks or a writable-parent open check.
- New disk succeeds at exact requested capacity, reads zeros, attaches at the
  selected target and survives guest write/reopen. Invalid size, overflow,
  existing paths, dangling symlinks, sidecar collisions, cancellation and host
  I/O errors preserve all preexisting files and the running machine.
- Actual SDL acceptance exercises Create, Save, Discard, Cancel, file-picker
  completion/cancellation, F5 replacement, failed construction and exit while
  creation is active. Displayed progress/errors trigger redraws in the existing
  event-driven presentation loop.
- Rebuild against the reviewed core. Run package formatting, locked Clippy
  with warnings denied, tests, release and make smoke. Complete original OBP
  and Solaris checks from the core handoff, including raw/ISO controls and a
  disposable new-disk format/install/reboot path. Use private catalogs/NVRAM
  and media, never the user's active disk.

## Separate headless clarification

The user's CDE banner report was clarified as text on the serial console.
Solaris's configured dtlogin service can start at boot even when the machine
has no framebuffer. For a guest intended to stay serial-only, Solaris provides
`/usr/dt/bin/dtconfig -d` to disable future desktop startup and `-kill` to stop
its current managed sessions; `-e` enables startup again. These are explicit
root actions inside the guest, not actions the frontend should inject when a
hardware toggle changes. Source: [Solaris 2.6 release notes](https://docs.oracle.com/cd/E19504-01/805-2988/805-2988.pdf)
and [Oracle's dtconfig reference](https://docs.oracle.com/cd/E19253-01/806-7492/appadtconfig-28152/index.html).

Current headless construction deliberately retains the Sun keyboard for the
abbreviated original-PROM path. Disk-free no-keyboard probes on the merged
core reach ok for all three machines, but SS-5 prints a synchronous-trap
error and SS-20 reports a failed TLB/POST diagnostic. Axil reached ok without
those messages. Logs are under Helios `local/headless-keyboard-2026-11-15/`.
This is a separate full-POST limitation, not evidence that a graphical CDE
console exists. No keyboard/POST behavior was changed during this review;
do not mix a workaround removal or guest-service changes into CHD work.
