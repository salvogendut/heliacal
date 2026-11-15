# Headless PTY implementation and acceptance

2026-11-15, branch `feat/headless-pty-console`, Heliacal issue #1.
Helios remains unchanged at `ac4dcdf`. This supersedes the implementation plan
in [headless-pty-console.md](headless-pty-console.md).

## Implemented

- Per-machine `headless` and `serial_pty_link` settings, CLI overrides,
  allocation-free topology validation, and Tinker-gated Advanced controls.
- No framebuffer/VSIMM or TurboGX ROM loading in headless construction. SS-20
  reaches 512 MiB; returning to SX adjusts only the draft to 448 MiB. Emulated
  keyboard presence is retained for abbreviated original-OBP startup.
- Raw nonblocking SCC A/PTY transport with bounded 64 KiB queues, partial-write
  retry, output-overflow reporting, pause buffering and receiver/reset flushing.
- Owned, locked aliases with no foreign-file/symlink replacement; unchanged
  endpoints survive F5, failed replacements preserve the guest and old PTY.
- A control-window endpoint/command panel, explicit terminal launcher using
  argument vectors, disabled headless capture, and an explicit SCC BREAK action.
- Console selection for initialized NVRAM from the pinned original PROMs.
  Independent OBP setenv diffs verified the counted strings and checksum delta.
  This preserves boot policy/identity and changes no ROM instructions.

## Validation

- 61 frontend tests pass, including actual PTY raw-byte/reconnect/backpressure,
  lock/alias ownership, worker pause/reset/failure, settings/named machine/RAM,
  and SCC BREAK hold/FIFO retry/receiver epoch regressions.
- `cargo fmt -p heliacal --check`, Clippy `--locked --offline --all-targets
  --no-deps -- -D warnings`, `cargo test --locked --offline`, release build and
  `make smoke` pass. The final smoke uses SDL dummy/software.
- Original SS-5 256 MiB graphical smoke passes at 98,027,520 outcomes,
  PC `ffd0c57c`; this smoke alone is not an OS-boot claim.
- Original-firmware SS-5/256 MiB/one CPU, Axil/512 MiB/two CPUs and
  SS-20/512 MiB/two CPUs boot installed Solaris 2.6 through actual minicom.
  Guest `psrinfo` reports CPU 0 on SS-5 and CPUs 0/2 on the MBus machines.
- SS-5, SS-20 and Axil write `/heliacal-pty-test`, cleanly sync/halt, pause/resume,
  confirm F5 while minicom stays attached, reboot and reread checksum
  `2128675962 21 /heliacal-pty-test`. Quit returns 0 and removes each alias.
- SS-20 preserves its original `auto-boot?` policy and reaches login without
  forcing an OBP prompt. Its initial serial command/write/halt phase passes.
  Its automatic reboot also passes the persistence and alias-cleanup checks.
- Actual SDL F9 Save/F5 headless → SX → headless on SS-20 passes. The draft
  visibly changes 512 to 448 MiB; the graphical capture shows the original
  448 MiB SX OBP `ok` prompt. Returning to headless restores TTYA and evaluates
  `8 9 + .` to hexadecimal `11`.
- Actual Advanced BREAK interrupts `begin again` in original SS-20 OBP. The
  initial one-batch pulse was too short; holding at least 1M outcomes matches
  the earlier accepted core firmware probe. Exact UART bit timing is not added.

## Evidence and limitations

Compact local evidence is under
`../helios/local/headless-pty-2026-11-15/`: `frontend-{ss5,ss20,axil}/` logs,
raw/minicom transcripts and PNG captures; `mode-transition/` captures and
acceptance log; the harness sources and independent OBP/NVRAM probes.
Build/test/smoke logs are copied there before cleanup.

The optional terminal launcher requires minicom and a supported terminal on
PATH. This environment has minicom in `my-distrobox`, but no supported graphical
terminal; actual minicom transport was tested using a controlling PTY, while
launcher argument handling has a tracked regression. No terminal package or
user desktop setting was installed/changed.

Pinned firmware with blank/malformed console fields needs initialized NVRAM:
boot graphically first. Unknown firmware is untouched and emits an explicit
manual-console-configuration notice. Full POST, OpenBIOS and arbitrary PROM
NVRAM formats are outside this acceptance. See the human
[serial-console guide](../docs/serial-console.md).

One early SS-20 harness incorrectly waited for `ok` despite `auto-boot?` being
set. The corrected harness respects automatic boot. Its forced termination
left a recorded alias whose PTY number was reused by another test; recovery
correctly refused to unlink that live target. The owned test alias was then
removed explicitly. No production safety check was relaxed.

Source disks, ROMs and the live catalog were not modified. Completed disposable
disk clones, temporary probe build output, downloaded OpenBoot reference clone
and redundant PPM captures were removed. The tiny `.heliacal-lock` files are
intentional persistent lock identities, not leaked processes or guest media.
