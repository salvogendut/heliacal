# Frontend architecture

Heliacal links the current sibling `../helios` library through a Cargo path
dependency. CPU, firmware execution, bus mapping, graphics, keyboard protocol,
SCSI behavior, and guest time belong to Helios. Heliacal adds host presentation,
input routing, settings, and lifecycle management. It does not patch firmware
or substitute a synthetic boot display.

`src/main.rs` owns the SDL event/render loop, host shortcuts, file-picker
completion, confirmations, and application lifetime. `view.rs` draws the guest
texture, paused screen, F9 overlay, notifications, status hints, and RUN/CPU/
SCSI activity LEDs. SDL's built-in debug font supplies the compact Happy Years
text. `overlay.rs` holds the editable settings draft independently from live
preferences; F9 saves it, and closing a dirty draft prompts Save/Discard/Cancel.

`emulator.rs` owns `Machine` on one worker thread. Construction and image
loading happen there. A bounded command channel controls boot/pause/shutdown.
A failed replacement leaves the existing machine and its run/pause state
intact. Successful cold replacement reuses NVRAM only when the resolved state
path is unchanged; switching labels of the same model saves the outgoing
state and loads the incoming label's file. The published machine name and
hardware summary change only after successful construction, so saving a
different selection does not relabel a guest that is still running.

RTC startup is a frontend policy. `rtc_host_time` defaults true in flat and
named records. The worker samples host UTC once at construction, then calls
`Nvram::initialize_sun_clock` on the prospective machine's private NVRAM copy.
Only RTC registers change; guest configuration and IDPROM survive. Failed
replacement cannot change the old clock. Saved-time mode does not read the host
clock. The setting participates in cold-replacement detection, but not NVRAM
identity. Core advancement, guest reset and pause/resume never resynchronize
from host time; UI refresh and the CPU-rate option remain independent.


The worker uses `Machine::step` for one CPU and `Machine::step_scheduled` for
an explicitly selected two-CPU machine. The topology fixes that choice until
cold replacement; it never mixes the two execution APIs on a running guest.
One worker owns both CPUs and shared devices. The core owns all scheduling
and shared virtual time; the frontend does not advance devices per CPU.
Snapshots retain hardware processor IDs 0/2 in Tinker diagnostics and errors;
step totals count execution outcomes, including trap/interrupt entry and
software reset. They are not a retired-instruction count. Old files select
one CPU; model/count validation uses the core's supported topology before
construction. CPU-count changes participate in the cold-reset comparison.

It publishes one
latest immutable RGBA framebuffer and a small status snapshot, at most about
30 times per host second. The main thread renders at about 60 Hz and never
holds a machine lock. Old frames are replaced rather than queued. The
1152x900 guest display is fitted within the logical 960x760 desktop, above the
16-pixel hint strip and 22-pixel LED strip; window resizing and high-DPI
rendering do not affect guest time. LED pulses are presentation-only and never
feed back into guest scheduling.

`input.rs` maps SDL physical scancodes to Sun Type 5 US stations, following
Helios's existing native keyboard mapping. Shared stations (left/right Ctrl,
Pause/F13) remain pressed until all corresponding host keys are released.
Host repeat is ignored. A bounded 256-transition queue retains events when the
core keyboard FIFO applies backpressure. On overflow, focus loss, pause,
opening the overlay, or releasing capture, pending input is discarded and
held guest stations are released. Releases themselves retry core backpressure.
Host mouse events navigate the frontend until capture. While captured, relative
motion and left/middle/right buttons are queued as Sun Mouse Systems packets on
SCC channel B; focus loss and capture release send button releases.

`settings.rs` serializes a strict TOML schema and selects SS-5, SS-20 or Axil
320 with an exact valid RAM total (128 MiB default). The model/display choice
delegates RAM validation and choices to the Helios profile API; RAM is part of
cold-replacement detection but not machine identity or NVRAM paths. SS-20
exposes a display choice: native SX/CG14 (no TurboGX ROM) or a TurboGX SBus
card. Graphical SS-5 and Axil 320 require TurboGX. CGsix construction selects the
no-VSIMM core path directly rather than removing SX after allocation.
Configured image sizes/readability and read/write disk access are
validated on save. Empty firmware paths are valid for an unconfigured desktop;
Boot reports what's missing. Paths are resolved relative to the config
directory. Saving uses a synced temporary file in the destination directory and
an atomic rename. An OS file lock prevents two instances sharing a config/NVRAM
file. NVRAM is separated per model (`<config>.nvram`, `.nvram.ss20`,
`.nvram.axil320`) so changing machines starts with the correct IDPROM identity.

`profiles.rs` captures machine settings in the `machines` TOML catalog and
uses `machine_label` to choose an entry. Model/display, ROMs, SCSI targets,
disk access and NAT belong to each entry; desktop preferences remain shared.
The selected stanza is authoritative on load. Save synchronizes the current
fields into that stanza before validating and atomically publishing the file.
Cycling labels retains edits within the draft; Save As preserves the source
entry. The config lock still limits a catalog to one open window.

Unnamed and first-named (`legacy_state`) workstations retain the original
NVRAM layout. Other labels use `<config>.nvram.machines/<hex-label>/nvram`
with the existing model suffix. Full byte encoding prevents labels containing
path separators or differing only in case from colliding. Files are created
only when firmware state is saved; opening/discarding an options draft has no
state-file side effects. Separate labels retain their image paths, so copying
a configuration does not duplicate a disk.

Normal shutdown saves NVRAM on the worker. If saving fails, the window stays
open, the machine remains available, and a running guest is paused for retry.
The worker is joined on application exit; an abnormal UI exit also attempts
to save NVRAM. This is firmware-state persistence, not a guest RAM snapshot.

The frontend does not create or truncate disk images. Populated hard-disk
targets use `ScsiBus::attach_disk_rw` by default and `ScsiBus::attach` when the
Advanced read-only option is selected. CD-ROM targets always use `attach`.
A fixed seven-element target array maps index to SCSI ID 0–6; ID 7 stays
reserved. Each target independently selects disconnected/disk/CD-ROM and an
image. Empty paths attach nothing; disconnected targets retain their saved
paths without opening them. Settings validation uses the selected device's
block size, and machine construction attaches every populated target.
Firmware compatibility and OS support remain properties of the evolving
Helios core. Unsupported core operations keep the last framebuffer visible,
with a persistent error and a full fault on stderr.

RAM choices come from the core's profile/video layout validation, including
Axil OBP 2.19's 16/64 MiB bank restriction. The frontend does not maintain a
second module-size table. Tracked RAM acceptance tests independently enumerate
socket populations and exercise the actual F5 worker replacement: a guest
write/read at 192 MiB distinguishes 128 MiB from 256 MiB installation. Cancel,
Discard, failed Save and failed construction preserve the previous machine,
catalog and NVRAM identity.

## Serial PTY ownership

`serial_pty.rs` owns a nonblocking Unix PTY and an optional locked, owned symlink
on the machine worker. It preserves raw SCC A bytes; printable diagnostics are
derived separately. Input is bounded and retained on SCC FIFO backpressure;
receiver epochs and cold resets discard stale input. Paused workers service
host transport at the existing bounded command wait without stepping the guest.
No terminal process owns emulation, and slow readers cannot block the worker.

Headless construction selects `InstalledVideo::None` before allocating RAM,
skips CGsix loading, and retains keyboard presence for supported OBP startup.
`obp_console.rs` adjusts only the two console variables in initialized NVRAM
for pinned original PROMs. See [firmware details and limits](serial-console.md).
The control panel displays the running endpoint, not an unsaved draft. Mode
and alias changes use the existing confirmed cold-replacement transaction;
failed replacement retains the old machine and PTY. `terminal.rs` runs only
on an explicit Advanced action and uses argument vectors, never a shell.

`serial_screen.rs` feeds the same raw SCC bytes into the pinned `vt100` parser
on the worker, independent of visibility. Its 80x24 screen has no scrollback;
the parser's default features are disabled to bound VTE OSC storage to 1024
bytes. Changed immutable screen snapshots are published with normal status
updates, at most roughly 30 times per second. SDL redraws the embedded view
when its generation changes and groups adjacent cells with matching attributes.
The renderer supports ANSI colours, underline and a static cursor using the
existing ASCII glyph font. Terminal callbacks remain inert: the view sends no
replies, keyboard input, clipboard operations or resize requests to the guest.

The global `show_serial_console` preference is Tinker-gated in Advanced and
applies on Save; it is excluded from hardware replacement and named-machine
state. Disabling it leaves both the parser and PTY running, so the next visible
snapshot is current. A cold replacement resets parser state together with the
frame cache. General's Display row is fixed/grey while headless, showing
Headless without changing the remembered graphical display choice.

Parser reference: [vt100 0.18.1](https://docs.rs/crate/vt100/0.18.1), pinned in
Cargo.lock. The no-std feature selection here bounds parser buffers; the
frontend itself remains a normal std application.

`accurate_cpu_clocks` is a per-machine hardware preference, default false.
False selects the core's default Fast 100 ns timebase. True constructs a fresh
machine then calls `reset_with_clock_mode(CpuClockMode::Hardware)` before
attaching keyboard, storage or host endpoints. This selects nominal 85/75 MHz
for both processors, while the core remains the sole owner of elapsed time.
Clock changes participate in hardware_eq and the existing confirmed cold
replacement; there is no live timebase switch or NVRAM identity change.

## Host media and explicit creation

Settings validation delegates to `helios::media::validate_attachment`, including
inactive named-machine records. It checks logical media geometry; CHD host
container size is never guest capacity. Validation does not create a companion.
The emulator worker opens actual media while building a replacement, preserving
the old guest if construction fails. Native CHD handles stay on that worker.

`disk_creation.rs` runs sparse CHD creation on a short-lived worker. SDL keeps
polling; the creation form is held until completion so a result cannot attach
to a different draft. Path selection/Cancel are separate from explicit Create.
The worker publishes through the core's private-file/no-overwrite helper.
Successful creation changes only a draft target, leaving Save/F5 and named
NVRAM identity unchanged. Discard never deletes the newly created user file.
