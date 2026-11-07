# Heliacal

![Heliacal](heliacal_logo.png)

Heliacal is a native **Rust/SDL3 Happy Years frontend** for the sibling
[Helios](../helios) Sun SPARC emulator. Its interface follows Prismatic and the
Happy Years family: the guest display owns the window, **F9** opens options,
and a status strip plus machine activity LEDs keep the essential state visible.

The initial machine is fixed at **Sun SPARCstation 5 / sun4m**, with an
**85 MHz microSPARC-II**, **128 MiB RAM**, **TurboGX (CGsix) graphics**, and a
**Sun Type 5 US keyboard**. Other machines and RAM sizes are not selectable.

## Build and run

Keep the checkouts alongside each other:

```text
Dev/
  helios/
  heliacal/
```

Use a current Rust toolchain (edition 2024), Cargo, a native linker, and SDL3
development libraries. SDL3 is linked dynamically; SDL_ttf and SDL_image are
not required. On a Fedora development system, the native dependencies are
`gcc`, `make`, and `SDL3-devel`.

```sh
make
./target/release/heliacal
# Or:
cargo run --release --locked
```

The application opens **paused**. Select your original Sun SS-5 PROM and
32 KiB TurboGX FCode ROM under **F9 > General**, save with **F9**, and press
**Ctrl+P** to boot. Firmware and guest operating systems are not bundled.
The Helios repository documents firmware acquisition and expected hashes.

You can also supply paths for this session:

```sh
./target/release/heliacal \
  --prom ../helios/local/firmware/sun-ss5.bin \
  --cgsix-rom ../helios/local/firmware/turbogx.bin
```

**F9 > Media** exposes **SCSI IDs 0–6**. Each target can be a hard disk, a
CD-ROM, or disconnected; **ID 7 is reserved for the host adapter**. Press
**Space** on a target, or click its type, to cycle the device type. Press
**Enter** or click its image path to open the system file picker (Ctrl+B does
the same); **Ctrl+E** lets you type a path instead.
Use **Delete** to detach and clear a target. Multiple disks and CDs may be
attached simultaneously. Changes take effect on the next confirmed cold reset.

Targets default to empty, with hard disk/CD-ROM types preselected at IDs 3/6.
Selecting an image for a disconnected target initially chooses hard disk;
use Space to change it to CD-ROM as needed. Cycling a target to disconnected
retains its path for later use. Saving checks the block size and readability
of each connected image.

Optional `--disk disk.raw` selects ID 3; `--cdrom install.iso` selects ID 6.
These shortcuts leave other targets unchanged. Images must contain whole
512-byte disk blocks or 2048-byte CD sectors. All attachments are **read-only**,
including when the linked core supports disk writes.

## Controls

| Key / action | Effect |
| --- | --- |
| Ctrl+P | Boot, pause, or resume |
| Click guest display / Ctrl+Enter | Capture / toggle keyboard capture |
| Pause + A | Sun Stop-A; F13 can also serve as Stop |
| F4 | Save the guest framebuffer as a native-resolution PPM in the working directory |
| F5 | Confirm a cold reset using the saved machine settings |
| F9 | Open settings / save settings |
| F11 | Toggle fullscreen |
| F12 / window close | Confirm quit and save NVRAM |
| Shift + reserved function key | Pass the shifted function key to the guest while captured |

In options, use **Left/Right** or **Tab/Shift+Tab** to switch sections,
**Up/Down** to select a row, and **Enter** to choose a file or toggle it. Tabs
and rows are clickable. For paths, **Enter** or **Ctrl+B** opens the native file picker
with firmware or disk/CD filters; **Ctrl+E** opens text entry, **Ctrl+V**
pastes, **Ctrl+U** clears, and **Delete** detaches the selected image when
outside text entry. **Escape** cancels text entry; with a changed draft it
offers **Save**, **Discard**, or **Cancel**. Confirmation dialogs default to
Cancel; use arrows and Enter, or click a choice.

Display offers smoothing, scanlines, and shortcut hints. The bottom strip shows
RUN, CPU, and configured hard disk/CD activity; shortcut hints sit above it and
can be hidden. **General > Tinker** reveals Advanced, with the core PC, step
count, and recent printable serial A output. Serial output is diagnostic text,
not a terminal emulator.

Settings remain a draft until **F9** validates and saves them successfully.
Display changes then apply immediately. Firmware and media changes apply at
the next **F5 cold reset**; saving them does not interrupt a running guest.
Choosing **Discard** closes the draft without changing the machine. Reset
replaces guest RAM and retains NVRAM; shut down a guest OS before resetting or
quitting.

## Configuration and state

The default config is `heliacal.cfg` under:

- Linux: `$XDG_CONFIG_HOME/heliacal/`, or `~/.config/heliacal/`.
- macOS: `~/Library/Application Support/heliacal/`.
- Windows: `%APPDATA%/heliacal/`.

`--config PATH` selects an independent configuration. Image paths in that
file are relative to its directory; CLI image paths are relative to the
working directory. CLI overrides are saved only if you choose Save in F9.
The `scsi` array contains exactly seven entries, in ID order (0 through 6),
each with `kind = "disconnected"`, `"disk"`, or `"cdrom"` and a `path`.
The early `disk`/`cdrom` config fields migrate to targets 3/6 on load.
See [the example configuration](heliacal.cfg.example).

NVRAM is loaded from and atomically saved to `<config-path>.nvram`; the lock
file is `<config-path>.lock`. Concurrent instances cannot use the same config
and NVRAM. Choose a different config for another workstation instance. Keep
firmware/disks under ignored `local/` if storing them in this repository.

Fresh NVRAM lets OBP initialize its defaults. The initial checksum message is
expected. The firmware performs its 128 MiB memory test and may automatically
attempt network boot. After the memory test, capture the keyboard and use
**Pause+A** to interrupt it. Helios documents its graphical `ok` prompt
checkpoint; startup may take several minutes.

## Status and validation

Implemented: real Helios execution on a worker thread, TurboGX presentation,
Sun keyboard input, pause/resume, confirmed cold reset and quit, NVRAM
persistence, firmware/media selection, settings drafts, display options,
screenshots, and Tinker diagnostics. The included logo is used on the paused
screen and as the window icon.

Validated on Linux with SDL3's software/dummy driver: the original Sun PROM
renders its SPARCstation 5 banner, reports **128 MB installed** and **Keyboard
Present**, and starts its memory test. This frontend check establishes display
integration; it does not establish OS boot or installation compatibility.
Guest mouse, audio, writable disks, interactive serial, host networking,
clipboard-to-guest paste, and additional models are not provided yet.
Native desktop file-picker behavior and non-Linux hosts need interactive
validation. The existing Prismatic and Iris repositories are independent.

```sh
make check
make smoke
# Optional real-firmware display check (temporary NVRAM; 180-second timeout):
SDL_VIDEO_DRIVER=dummy SDL_RENDER_DRIVER=software \
  ./target/release/heliacal --boot-smoke-test \
  --prom ../helios/local/firmware/sun-ss5.bin \
  --cgsix-rom ../helios/local/firmware/turbogx.bin \
  --screenshot /tmp/heliacal-boot.bmp
```

`--smoke-test --screenshot /tmp/heliacal-media.bmp` captures the Media
overlay with temporary disk/CD targets. Smoke tests use temporary preferences and NVRAM. Unit tests exercise
real-core lifecycle behavior with synthetic firmware, settings failure paths,
all seven SCSI IDs, disk/CD selection, input backpressure, shared modifier
keys, and failed NVRAM save/retry.
See [architecture](docs/architecture.md) for the frontend/core boundary.
