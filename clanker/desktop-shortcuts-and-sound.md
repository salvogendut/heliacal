# Desktop shortcuts and screenshot sound — 2026-11-16

Ctrl+Plus/Equals and Ctrl+Minus resize the host window proportionally by
1.25 or 0.8, preserving the existing 720×570 minimum and fitting its display's
usable area including decorations. Keypad equivalents work. Host keycodes
support non-US layouts; Alt/AltGr/GUI combinations remain available to their
existing handlers. Fullscreen ignores resizing. Maximized windows restore
asynchronously before applying the requested size. Consumed chords suppress
text input until release or focus loss and work before overlay/serial routing.

F12 and window close skip confirmation for both `Stopped` and `Failed`
(displayed as STOPPED / CORE ERROR). Running and paused guests retain the
default-Cancel confirmation. A separate `Snapshot::shutdown_complete` flag
prevents guest power-off from being mistaken for completed worker shutdown:
the UI waits for NVRAM persistence and PTY cleanup. Failed state saving keeps
the window and worker available for retry, including an already stopped guest.

F4 embeds the exact 30,030-byte WAV from `../1984/src/shutter_wav.h`, pinned
and checksummed in [the asset notes](../assets/README.md). An SDL playback
stream is opened lazily after a successful screenshot save, on the main
thread. Captures clear/requeue the short sound without blocking the UI or
worker. Save failures and missing framebuffers remain silent; unavailable
audio does not turn a saved screenshot into a failure. Guest audio support
is unchanged.

Validation:

- `cargo fmt -p heliacal --check`: passed.
- `cargo clippy --locked --all-targets --no-deps -- -D warnings`: passed.
- `RUST_MIN_STACK=16777216 cargo test --locked`: all 107 tests passed,
  including three resize/input tests and four quit/persistence tests. This
  retains the stack override documented in [file-picker history](file-picker-history.md)
  for the concurrently edited sibling Helios MMU. The default-stack full
  suite was not re-established; no core files or worker stack sizes changed.
- `make smoke`: passed without a stack override.
- `cargo build --release --locked`: passed.
- Final real SS-5 PROM plus TurboGX display smoke: passed after 98,402,304
  steps, PC `ffd0c5b4`, with an explicit empty source configuration, temporary
  NVRAM, no disks, and SDL dummy/software drivers. Output:
  `/tmp/heliacal-desktop-firmware-ihqBMy/{boot.log,ss5.bmp}`. This confirms
  firmware display output, not an OS boot.
- Thirteen isolated X11/Fluxbox desktop checks in `my-distrobox` passed:
  standard/keypad chords, Alt exclusion, min/max sizes, fullscreen no-op and
  restore, maximized restore, overlay/captured/serial routing, pre-boot and
  headless F4, successful/repeated captures, write failures, absent audio,
  running confirmation, stopped NVRAM failure/retry and PTY cleanup, and
  core-error shutdown. All used synthetic PROMs and temporary configuration;
  no user firmware or media was modified.
- SDL's disk audio backend recorded stereo S16 at 44.1 kHz. Both channels
  contained the original mono samples exactly, with only device silence
  outside the recording. Two successful F4 captures produced two copies;
  a rejected write produced no additional sound. The WAV bytes were also
  compared directly against 1984's header array.
- Desktop harness and evidence:
  `/tmp/heliacal-desktop-acceptance.py`,
  `/tmp/heliacal-desktop-acceptance-6xaxnzpk/result.json` and per-run logs.
  Xdotool selected an unwanted Ctrl+Alt XKB level for keypad keysyms, so the
  harness used physical XTest keypad events. No frontend workaround was
  needed. Native Wayland and physical speaker playback were not tested.

Build environment: cached nightly toolchain under
`../xiri/.work/rustup/toolchains/nightly-2026-10-28-x86_64-unknown-linux-gnu/bin`,
`CARGO_HOME=/tmp/heliacal-cargo`, and ignored `target/native` SDL3/libstdc++
linker symlinks. Desktop acceptance used the container's Xvfb, Fluxbox,
xdotool and Python, with all runtime files under `/tmp`.
