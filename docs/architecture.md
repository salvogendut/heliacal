# Frontend architecture

Heliacal links the current sibling `../helios` library through a Cargo path
dependency. CPU, firmware execution, bus mapping, graphics, keyboard protocol,
SCSI behavior, and guest time belong to Helios. Heliacal adds host presentation,
input routing, settings, and lifecycle management. It does not patch firmware
or substitute a synthetic boot display.

`src/main.rs` owns the SDL event/render loop, host shortcuts, file-picker
completion, confirmations, and application lifetime. `view.rs` draws the guest
texture, paused screen, F9 overlay, notifications, and status strip. SDL's
built-in debug font supplies the compact Happy Years text. `overlay.rs` holds
the editable settings draft independently from live preferences.

`emulator.rs` owns `Machine` on one worker thread. Construction and image
loading happen there. A bounded command channel controls boot/pause/shutdown.
A failed replacement leaves the existing machine and its run/pause state
intact. Successful cold replacement uses the current machine's NVRAM.

The worker advances guest time only through `Machine::step`. It publishes one
latest immutable RGBA framebuffer and a small status snapshot, at most about
30 times per host second. The main thread renders at about 60 Hz and never
holds a machine lock. Old frames are replaced rather than queued. The
1152x900 guest display is fitted within the logical 960x760 desktop; window
resizing and high-DPI rendering do not affect guest time.

`input.rs` maps SDL physical scancodes to Sun Type 5 US stations, following
Helios's existing native keyboard mapping. Shared stations (left/right Ctrl,
Pause/F13) remain pressed until all corresponding host keys are released.
Host repeat is ignored. A bounded 256-transition queue retains events when the
core keyboard FIFO applies backpressure. On overflow, focus loss, pause,
opening the overlay, or releasing capture, pending input is discarded and
held guest stations are released. Releases themselves retry core backpressure.
Host mouse events navigate the frontend and capture the keyboard; no guest
mouse packets are synthesized.

`settings.rs` serializes a strict TOML schema, validates the sole machine and
RAM size, and validates configured image sizes/readability on save. Empty
firmware paths are valid for an unconfigured desktop; Boot reports what's
missing. Paths are resolved relative to the config directory. Saving uses a
synced temporary file in the destination directory and an atomic rename.
An OS file lock prevents two instances sharing a config/NVRAM file.

Normal shutdown saves NVRAM on the worker. If saving fails, the window stays
open, the machine remains available, and a running guest is paused for retry.
The worker is joined on application exit; an abnormal UI exit also attempts
to save NVRAM. This is firmware-state persistence, not a guest RAM snapshot.

No disk image is created or modified. The frontend uses `ScsiBus::attach`,
which opens raw media read-only; it does not call the core's writable attach
API. A fixed seven-element target array maps index to SCSI ID 0–6; ID 7 stays
reserved. Each target independently selects disconnected/disk/CD-ROM and an
image. Empty paths attach nothing; disconnected targets retain their saved
paths without opening them. Settings validation uses the selected device's
block size, and machine construction attaches every populated target.
Firmware compatibility and OS support remain properties of the evolving
Helios core. Unsupported core operations keep the last framebuffer visible,
with a persistent error and a full fault on stderr.
