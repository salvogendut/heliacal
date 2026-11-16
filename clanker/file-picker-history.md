# File-picker history — 2026-11-16

`src/file_picker.rs` keeps the last selected file in three categories:
firmware (PROM/FCode), hard disk and CD-ROM. The current field takes
precedence; empty fields use history. Missing files/directories fall back
to a surviving directory, with a trailing separator for native pickers.
New-disk dialogs use the disk directory with a new SCSI-specific filename.

History is atomically written to `<config>.picker.toml` after a successful
selection. It is shared across named machines in that catalog and independent
of the hardware settings draft. Discard can retain this navigation preference
without saving attachments or changing the guest. Config locking already
serializes writers. Missing history starts empty; malformed/unreadable history
logs a warning and does not prevent launch. Save errors retain the choice
in the draft and report a notification.

The SDL Zenity handoff returned an empty filename when the controlled chooser
cancelled. `App::picked` now ignores that result before changing the draft or
history. The smoke test covers this alongside persistent disk/CD history and
Discard after choosing a new-disk destination.

Validation:

- `cargo fmt -p heliacal --check`: passed.
- `cargo clippy --locked --all-targets --no-deps -- -D warnings`: passed.
- `cargo test --locked`: initially aborted with stack overflow in the
  `helios-machine` worker. `RUST_MIN_STACK=16777216 cargo test --locked`
  passed all 100 tests, including four new history tests. The sibling Helios
  checkout had in-progress `src/mmu.rs` and `tests/mmu.rs` changes; no core
  files or worker-stack settings were changed here. This run does not
  establish a passing full suite with the default Rust thread stack.
- `make smoke`: passed without the stack override.
- Seven isolated X11 checks through SDL's actual Zenity backend and a
  controlled chooser responder: PROM/FCode sharing, current-file precedence,
  cancellation, independent disk/CD history, new-disk filename, Discard,
  restart and removed-file fallback. Both application runs quit cleanly.
  No guest was booted; no user firmware, media or configuration was used.
  This checks dialog arguments and callbacks, not native GTK/portal visuals.
- Documentation links and `git diff --check`: passed.

The temporary driver was `/tmp/heliacal-picker-acceptance.py`; successful
evidence is `/tmp/heliacal-picker-acceptance-zhk8hwiy/`, including
`calls.jsonl`, `result.json`, the history file and application logs.

Used the toolchain/cache from `build-environment.md`. The host also needed
`target/native/libstdc++.so -> /lib64/libstdc++.so.6` alongside its existing
SDL linker symlink; both remain ignored build artifacts. Xvfb/xdotool ran
inside the existing `my-distrobox` container with its own display and a
temporary responder on PATH. The initial cancellation failure was reproduced
there and passed after the empty-filename guard.
