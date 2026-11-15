# Headless guest and serial PTY handoff

Prepared 2026-11-15 for [Heliacal issue #1](https://github.com/salvogendut/heliacal/issues/1).
Branch: `feat/headless-pty-console`, starting at Heliacal `39f9691` with Helios
`ac4dcdf`. This is the implementation plan; no headless/PTY feature is claimed
implemented by this documentation commit. The reviewed baseline is 48 frontend
tests and 624 normal / 626 perf-counter core tests.

## User decisions and scope

The user explicitly chose to **keep the Heliacal control window**. Headless
means the emulated machine uses its TTYA console and has no framebuffer/VSIMM;
it does not mean a windowless Heliacal process. Keep F9, boot/pause/reset/quit,
status and endpoint information available. Preserve the existing paused-launch
behavior and guest `auto-boot?` policy.

New controls belong under **F9 > Advanced**, gated by **General > Tinker**.
The boot mode and optional stable serial symlink belong to each named machine.
Existing files default to graphical mode and no PTY alias. Turning Tinker off
hides the controls; it must not silently reset the guest or remove its console.
Show the active console/endpoint independently of unsaved settings drafts.

Headless construction must not require a TurboGX ROM or install CGsix/SX.
Use the core's no-VSIMM topology: SS-5 up to 256 MiB, SS-20/Axil up to 512 MiB.
Remember the graphical display choice for switching back. An SS-20 returning
from headless 512 MiB to SX must visibly adjust its draft to a valid capacity
(as the existing model/display RAM adjustment does), then use Save and the
confirmed F5 replacement flow. Raw configuration input must be validated,
not silently clamped. Do not add four-CPU support or alter scheduler timing.

The user also suggested an **Open serial terminal** button. Plan it as a small
separate slice after the PTY works; it must not be required for manual minicom
connections. A true windowless service, TCP serial backend, TTYB, embedded
terminal emulator and remote management API are outside this issue.

## Existing code and references

Read both repositories' `AGENTS.md`. Heliacal implementation points:

- `src/settings.rs`: `Display` currently has CGsix/SX only; topology validation
  and RAM choices use `display.installed_video()`. Introduce one shared
  effective-hardware calculation for headless/graphical mode and use it in
  validation, RAM choices, construction and replacement detection.
- `src/profiles.rs`: add the new per-machine fields to capture/apply/resolve/
  validation, including inactive records. Keep label and NVRAM path identity.
- `src/emulator.rs::build`: currently requires the CGsix ROM for CGsix, installs
  the selected framebuffer and always attaches an emulated keyboard.
- `src/emulator.rs::run`: drains TTYA with `take_output(Channel::A)`, then strips
  CR and nonprintable bytes for diagnostics. Drain once and give the original
  bytes to the transport; derive the existing printable diagnostic view from
  that same buffer. Do not send the filtered string to minicom.
- The keyboard-delivery loop currently unwraps the keyboard device. Do not
  remove it without handling that path. Disable frontend guest keyboard/mouse
  capture in headless mode; keep host control shortcuts working.
- `publish` and `FrameCache`: skip scanout decoding/publication in headless
  mode; clear the previous guest frame on successful mode replacement. Keep
  status/activity updates and shared guest time independent of rendering.
- `src/overlay.rs`, `main.rs`, `view.rs`: transactional drafts, inline text
  editor, confirmed replacement, Tinker diagnostics and cold-reset lifecycle.
  Advanced currently includes Disk access even when Tinker is off; gate the
  new rows rather than hiding existing unrelated controls.

Helios already provides raw `Serial::receive`, `take_output`,
`receiver_enabled`, `receiver_epoch`, and decoded `set_break` on Channel A.
`receive` can refuse a byte; do not pop it from a host queue until accepted.
Check how receiver disable/reset epochs should discard or preserve queued
input, especially across guest reset versus full machine replacement.

The requested reference is 1984's **USIfAC II** interface, at revision
`4f8453f7566d0d7dfec46f1974dabbf3dd6f4ab9`:

- [PTY implementation](https://github.com/salvogendut/1984/blob/4f8453f7566d0d7dfec46f1974dabbf3dd6f4ab9/src/usifac.c)
- [User interface and connection recipes](https://github.com/salvogendut/1984/blob/4f8453f7566d0d7dfec46f1974dabbf3dd6f4ab9/docs/USIFAC.md)
- Local `../1984/src/overlay.c`, `config.c`, `config.h`: inline alias editor
  and persisted `usifac_pty_link` field.

Use the observable behavior as a reference, not copied C implementation.
1984's setup unconditionally unlinks the configured alias path, and its TX
loop can drop a byte after a failed write. Heliacal must preserve unrelated
files and retry partial/EAGAIN writes correctly. Its shutdown target check is
useful, but session ownership and concurrent instances also need handling.

[Linux pty(7)](https://man7.org/linux/man-pages/man7/pty.7.html) documents the
master/slave byte direction and asynchronous I/O. The emulator owns the master;
minicom opens the slave through `/dev/pts/N` or the alias. Use safe Unix APIs
without weakening `unsafe_code = "forbid"`; for example,
[nix's safe openpty API](https://docs.rs/nix/0.31.3/nix/pty/fn.openpty.html)
returns both descriptors and supports slave termios configuration. Confirm the
selected dependency/version/features against the actual toolchain and lockfile.
Do not introduce a child shell or `forkpty` just to create the serial endpoint.

## Slice A — original-firmware console contract

Resolve this before spending long runs on GUI integration:

1. Using disposable NVRAM and media, establish display-free TTYA startup on
   SS-5, SS-20 and Axil, with the original pinned firmware. Test both a valid
   existing graphical NVRAM image and an existing serial image.
2. Read Helios `docs/firmware.md` around the SS-20/Axil startup notes. SS-20
   abbreviated startup currently depends on keyboard presence; keyboard-free
   boot can enter unsupported full POST. Headless presentation does not require
   deleting the modeled keyboard. Preserve its presence if necessary and
   document the exact emulated configuration; do not conceal a POST failure.
3. Decide and document how the selected mode gets OBP onto `input-device ttya`
   and `output-device ttya`, and how graphical restoration works. Prove the
   transition with the same named machine/NVRAM. Do not replace NVRAM wholesale,
   reset identity, overwrite unrelated settings, patch ROM instructions, or
   repeatedly inject configuration commands into an arbitrary running guest.
   Any NVRAM environment editing requires a documented format/checksum and
   focused tests; normal OBP commands are the reference behavior.
4. Keep existing boot-device, auto-boot and diagnostic settings unless a
   documented mode transition requires a change. Record any remaining setup
   requirement explicitly before presenting a one-click headless boot as done.
5. Use a small headless harness and existing core serial APIs first. If core
   behavior needs a fix, isolate it on a corresponding Helios branch with
   sources/tests; keep the core usable without the frontend.

## Slice B — PTY transport and lifetime

Create a frontend-owned Unix transport, separate from the SPARC/SCC device.
Set the slave to raw 8-bit operation without echo, canonical buffering or
CR/LF translation. Apply nonblocking I/O and close-on-exec appropriately.
Define whether a slave keepalive descriptor is retained; do not report a
connected external client merely because the app holds that descriptor.

Use bounded host RX/TX queues. Retain unwritten suffixes and refused SCC input.
Handle EAGAIN/EINTR and disconnected-slave EIO/EOF without killing the machine
or busy-spinning. Make overflow policy explicit and observable; an absent or
slow reader must not cause unbounded memory growth or stop guest clocks.
Service bytes in bounded batches. Keep pause/control responsiveness, and avoid
per-instruction syscalls. Retain the PTY over pause and preferably same-machine
F5 reset so a connected terminal survives; fully replace it when identity or
endpoint configuration requires it. Never deliver old-machine RX to a new one.

A blank alias means use the displayed slave path directly. Resolve a custom
relative alias against the config directory; it need not already exist. Save
must not create a PTY or symlink. Publish the link only when the endpoint is
ready, and preserve the old guest/link if replacement construction fails.

Use no-follow metadata and an ownership/locking scheme to reject regular files,
directories, unrelated symlinks and a live competing session. Define stale-link
recovery without blindly unlinking anything. At shutdown remove only the link
still owned by this session; if another process replaced it, leave it alone.
Do not alter device-node permissions or rename a `/dev/pts` node. Unsupported
platforms must return a clear feature error without breaking graphical mode.

## Slice C — named configuration and Tinker UI

Suggested fields are `headless = false` and `serial_pty_link = ""`; final names
can follow repository conventions. Persist them in Settings and NamedMachine,
including defaults, round trips, inactive-record validation and relative paths.
Keep any host-terminal application preference global rather than copying it
into each workstation's hardware identity.

Under Advanced, when Tinker is enabled, expose Boot mode, Serial PTY link,
read-only active endpoint/status, and a manual Send BREAK action if needed.
Use the existing inline path editor for a new alias, not an existing-file-only
picker. Add changes to replacement detection, with clear saved-versus-running
labels. Failed validation or endpoint creation must retain the current machine.

Determine whether minicom's BREAK ioctl reaches the master with the chosen
platform/API. Verify it rather than interpreting a NUL or Ctrl-C as BREAK.
If unsupported, document that and route the explicit action through the core's
`set_break`, including retryable release. Sending guest control bytes must not
signal or terminate the host process.

## Slice D — optional terminal button

A working manual connection recipe, verified against installed minicom help:

```sh
minicom -o -8 -b 9600 -D /tmp/heliacal-ss20.pty
```

Document 8N1 and disabled hardware/software flow control. Verify exact guest
console behavior; PTY settings are not a new SPARC timing model.

The button should launch an available/configured terminal and minicom using
`std::process::Command` plus individual arguments, with the active alias or
slave as one literal argument. No `sh -c` or interpolated command strings.
Support spaces in paths. Report missing executables or launch failure without
changing the guest or its endpoint. Do not auto-launch on boot. Reap owned
children without blocking SDL; closing the terminal must leave the guest alive.
Disable the action when no live endpoint exists. Unit-test argument construction
with a harmless fake launcher, then perform one real terminal/minicom check.

## Acceptance, documentation and cleanup

First add meaningful transport/lifecycle/config tests: all byte values, CR/ESC,
partial writes, FIFO refusal, pause/resume, reconnect, reset, failed replacement,
foreign files/links, two-instance contention, ownership-safe cleanup and paths
with spaces. Check old configs stay graphical and retain their NVRAM identity.
Check both maximum RAM topology and returning to an SX graphical draft.

Then use real minicom on original-firmware disposable guests:

- SS-5 UP, SS-20 and Axil UP/SMP: OBP `ok`, a Forth arithmetic command, Solaris
  2.6 login and an interactive shell command, then sync/halt.
- Disconnect/reconnect the terminal, pause/resume, confirm F5 reset and verify
  the alias still points at the live endpoint. Check named-machine switching.
- Exercise graphical → headless → graphical with the same named machine,
  preserving disks, identity and unrelated NVRAM settings.
- Check UI controls remain responsive, host input is not accidentally forwarded
  to the headless guest, and detached/paused operation does not busy-spin.
- Retain current graphical tests/smokes. Test the optional launcher separately
  and report it as deferred if not delivered; do not claim it from a unit test.

Run the frontend checks in AGENTS.md, rebuild the release binary and run SDL
and original-firmware smokes. Run the core's full required validation if core
code changes. Documentation-only preparation needs diff/link review, not a
repeat of long guest boots. Update README, architecture, example config and
this handoff/index as slices land. Record actual successes and limitations.

Keep firmware/disks under ignored local/, hash source media and the live catalog,
use disposable clones, and keep compact logs/captures/commands. Remove completed
scratch disks/NVRAM, temporary builds and duplicate captures. Do not change the
user's live machine catalog merely to demonstrate the feature.
