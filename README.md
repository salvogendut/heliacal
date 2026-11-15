# Heliacal

![Heliacal](heliacal_logo.png)

Heliacal is a native **Rust/SDL3 Happy Years frontend** for the sibling
[Helios](../helios) Sun SPARC emulator. Its interface follows Prismatic and the
Happy Years family: the guest display owns the window, **F9** opens options,
and a status strip plus machine activity LEDs keep the essential state visible.

The machine is selectable as **Sun SPARCstation 5**, **Sun SPARCstation 20**,
or **Axil 320**. All three use sun4m and a **Sun Type 5 US keyboard**, with
**128 MiB RAM by default**. SS-5 and Axil 320 use a **TurboGX (CGsix)** SBus
card. SS-20 can use either its native **SX/CG14** framebuffer or a **TurboGX**
card; select this with **F9 > General > Display** or `--display sx|cgsix`. The
native SX option does not need a TurboGX ROM. F9's **Memory** row or
`--ram-mib` selects any exact DSIMM total supported by the current
model/display: SS-5 up to 256 MiB, SS-20 with SX up to 448 MiB, and SS-20
without a VSIMM or Axil 320 up to 512 MiB. Invalid totals are rejected, not
rounded. The Axil OBP 2.19 profile uses 16/64 MiB banks: 32 MiB total uses
two 16 MiB banks, while 432/480/496 MiB cannot fit its eight sockets. The
512 MiB maximum is unchanged. Physical Axil support for smaller modules is
not established by this firmware-compatible profile.

Solaris 2.6 graphical-console checks pass at 256 MiB on SS-5, 448 MiB on
SS-20/SX, and 512 MiB on SS-20/CGsix and Axil. These include guest typing
and capture/release; the MBus configurations use two CPUs. The core also
passes memory verification and disk write/reboot checks at each maximum.
These are bounded console checks, not exhaustive desktop compatibility.

For SS-20 with **CGsix** as the Solaris console, use OBP
`setenv sbus-probe-list f0123` once and reset. The default probe list includes
the unimplemented DBRI audio slot, which leaves a nameless firmware node and
causes Solaris 2.6 console discovery to panic even at 128 MiB. This normal
NVRAM setting keeps onboard SCSI/Ethernet and external slots, and avoids the
unsupported audio probe. Native SX does not require it. See the
[core configuration notes](../helios/docs/machines.md#ss-20-cgsix-console-configuration).

**F9 > General > CPUs** selects one or two CPUs on SS-20/Axil; SS-5 is
fixed at one. Two CPUs use two identical single-CPU MBus modules and are
**experimental**: original OBP identifies both, and Solaris 2.6 SMP boots with
both CPUs online. Recorded Linux/X11 checkpoints include SS-20 native-SX
terminal input, capture and dragging, plus Axil/CGsix CDE window operations,
focus-loss key release, disk activity and clean reset/reboot persistence.
These are bounded workloads, not exhaustive SMP compatibility. Save with F9,
then use F5 to confirm a cold reset of an existing guest. CPU count is also
saved with each named machine (`cpus = 2`) and can be overridden for a session
with `--cpus 2`. Old configurations default to one CPU.

SS-20 and Axil can accommodate up to four hardware CPUs using two compatible
dual-CPU MBus modules. Helios currently implements one CPU per module, so
counts other than one or two are rejected; four is not a working frontend
option. Sun's [SS-20 service manual, table 12-1](https://docs.oracle.com/cd/E19127-01/sparc20.ws/801-6189-12/801-6189-12.pdf)
lists dual-processor modules, and the [module installation guide](https://docs.oracle.com/cd/E19957-01/802-2566-11/install.chapter3.html)
requires compatible pairs. Axil's use of the same modules is confirmed by
the project owner; implementing those dual-CPU modules is a later core milestone.

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

The application opens **paused**. Choose the machine, display and original Sun
PROM under **F9 > General**. TurboGX requires a 32 KiB FCode ROM; native
SS-20 SX does not. Save with **F9** and press **Ctrl+P** to boot. The default
is 128 MiB; choose **Memory** in F9 for another exact supported total.
Firmware and guest operating systems are not bundled. The Helios repository
documents firmware acquisition and expected hashes.

You can also select the machine and supply paths for this session:

```sh
./target/release/heliacal --machine ss5 \
  --prom ../helios/local/firmware/sun-ss5.bin \
  --cgsix-rom ../helios/local/firmware/turbogx.bin
./target/release/heliacal --machine ss20 --display sx --ram-mib 448 \
  --prom ../helios/local/firmware/ss10-20-v2.25r-rom
./target/release/heliacal --machine ss20 --display cgsix --ram-mib 512 \
  --prom ../helios/local/firmware/ss10-20-v2.25r-rom \
  --cgsix-rom ../helios/local/firmware/turbogx.bin
./target/release/heliacal --machine axil320 --ram-mib 512 \
  --prom ../helios/local/firmware/axil320-2-19.bin \
  --cgsix-rom ../helios/local/firmware/turbogx.bin
```

SS-5 and Axil 320 require a TurboGX ROM. SS-20 with `--display sx` does not.

**F9 > Media** exposes **SCSI IDs 0–6**. Each target can be a hard disk, a
CD-ROM, or disconnected; **ID 7 is reserved for the host adapter**. Press
**Space** on a target, or click its type, to cycle the device type. Press
**Enter** or click its image path to open the system file picker (Ctrl+B does
the same); **Ctrl+E** lets you type a path instead.
Use **Delete** to detach and clear a target. Multiple disks and CDs may be
attached simultaneously. Changes take effect on the next confirmed cold reset.
The original OBP `probe-scsi-all` discovers attached targets on SS-5 and now
also through the SS-20/Axil standalone IOMMU.

Targets default to empty, with hard disk/CD-ROM types preselected at IDs 3/6.
Hard disks use read/write access by default; CDs never use writable access.
Selecting an image for a disconnected target initially chooses hard disk;
use Space to change it to CD-ROM as needed. Cycling a target to disconnected
retains its path for later use. Saving checks the block size and readability
of each connected image, plus write access for read/write hard disks.

Optional `--disk disk.raw` selects ID 3; `--cdrom install.iso` selects ID 6.
These shortcuts leave other targets unchanged. Images must contain whole
512-byte disk blocks or 2048-byte CD sectors. Hard disks default to **read/write**
so installed systems can update their file systems; CD-ROM images remain
read-only. Select **Advanced > Disk access** to make all hard disks read-only.

## Named machines

Save a complete workstation under **F9 > General > Save machine as...**.
Enter a label, press **Enter**, then **F9** to save. Labels accept 1–32
printable ASCII characters. An existing label is rejected; select that
machine first to update its settings.

**F9 > General > Machine label** cycles saved machines alphabetically with
Enter or a click. Each label recalls the model, CPU count, display, PROM and TurboGX
ROM, all seven SCSI targets, disk access and networking. Smoothing, scanlines,
shortcut hints and Tinker remain shared. Selection and edits stay in the
draft until Save; Discard leaves the saved configuration and guest intact.

After selecting and saving, **Ctrl+P** boots if no guest exists. For an
existing guest, shut down its OS and use **F5** to confirm a cold reset into
the selected machine. Ctrl+P continues to pause/resume the existing guest;
the window title and status strip identify that actual guest until reset.
One workstation runs per window.

You can define labels directly in `heliacal.cfg`; restart Heliacal after editing
the file to reload the catalog:

```toml
machine_label = "SS20 Solaris 2.6"
smoothing = false

[machines."SS20 Solaris 2.6"]
model = "sparcstation20"
display = "sx"
prom = "/path/to/ss10-20-v2.25r-rom"
disk = "/path/to/solaris26-ss20.raw"
cdrom = "/path/to/solaris26.iso"
network = true

[machines."SS5 Solaris 7"]
model = "sparcstation5"
prom = "/path/to/sun-ss5.bin"
cgsix_rom = "/path/to/turbogx.bin"
disk = "/path/to/solaris7-ss5.raw"
```

`machine_label` selects the default; its stanza takes precedence over the
legacy top-level machine fields. Relative image paths use the config's
directory. `disk` and `cdrom` are shorthand for SCSI IDs 3 and 6; use the
seven-entry `scsi` array for other targets. A stanza must not mix a populated
`disk`/`cdrom` shorthand with `scsi`. Menu saves write complete target arrays.
Saving checks the selected machine's image files; inactive labels can refer
to media that is currently offline.

To launch a saved machine directly:

```sh
./target/release/heliacal --machine-label "SS20 Solaris 2.6"
# With a separate catalog:
./target/release/heliacal --config /path/to/heliacal.cfg --machine-label "SS5 Solaris 7"
```

Explicit CLI model, display and image options override the chosen label for
that session; F9 Save persists those overrides. Launch still starts paused.

The first label created from an unnamed setup adopts its existing NVRAM
(`legacy_state = true`). Additional labels get separate NVRAM per label and
model under `<config>.nvram.machines/<hex-encoded-label>/`; hand-written
stanzas use this separate state unless `legacy_state = true` is set. Save As
copies settings, including image paths: it does not clone disk images.
Existing configurations without labels retain their original NVRAM paths.
RAM is part of each named machine's hardware settings and is included in
cold-replacement detection. It does not change the label, running machine
identity or NVRAM path; a saved change applies on the next confirmed F5 cold
reset, and a failed replacement leaves the prior machine running.

## Controls

| Key / action | Effect |
| --- | --- |
| Ctrl+P | Boot, pause, or resume |
| Click guest display / Ctrl+Enter | Capture / toggle keyboard and mouse capture |
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
can be hidden. **Advanced** selects hard-disk read/write or read-only access.
**General > Tinker** enables the Advanced diagnostics block with the core PC,
step count, and recent printable serial A output. Serial output is diagnostic
text, not a terminal emulator. Disk-access changes apply on the next cold reset.

Settings remain a draft until **F9** validates and saves them successfully.
Render options such as smoothing and scanlines apply immediately. Model,
hardware display, RAM, firmware, media and disk-access changes apply at the
next **F5 cold reset**; saving them does not interrupt a running guest.
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
working directory. CLI overrides, including `--ram-mib`, are saved only if you
choose Save in F9. `ram_mib` defaults to 128 and must be an exact valid total
for the selected model/display; named stanzas may omit it to inherit 128.
The `scsi` array contains exactly seven entries, in ID order (0 through 6),
each with `kind = "disconnected"`, `"disk"`, or `"cdrom"` and a `path`.
The early `disk`/`cdrom` config fields migrate to targets 3/6 on load.
`network = true` enables the user-mode NAT (default on). See
[the example configuration](heliacal.cfg.example).

Unnamed configurations load and atomically save NVRAM per model: `<config-path>.nvram`
for SS-5, `<config-path>.nvram.ss20` for SS-20, and
`<config-path>.nvram.axil320` for Axil 320. Named machines use the state
paths described above. The lock file is
`<config-path>.lock`. Concurrent instances cannot use the same config.
Choose a different config for another workstation instance. Keep
firmware/disks under ignored `local/` if storing them in this repository.

Fresh NVRAM lets OBP initialize its defaults. The initial checksum message is
expected. The firmware performs its 128 MiB memory test and may automatically
attempt network boot. After the memory test, capture input and use
**Pause+A** to interrupt it. Helios documents its graphical `ok` prompt
checkpoint; startup may take several minutes.

## Status and validation

Implemented: real Helios execution on a worker thread, TurboGX and native
SS-20 SX/CG14 presentation,
Sun keyboard and mouse input, pause/resume, confirmed cold reset and quit, NVRAM
persistence, named machine configurations, firmware/media selection, writable/read-only disk access, settings
drafts, display options, screenshots, and Tinker diagnostics. The included logo
is used on the paused screen and as the window icon.

Validated on Linux with SDL3's software/dummy driver: the original Sun PROM
renders its SPARCstation 5 banner, reports **128 MB installed** and **Keyboard
Present**, and starts its memory test. Native SS-20 SX/CG14 also boots an
installed Solaris 2.6 disk through the graphical console and runs Xsun with
olwm and xterm. Linux/X11 input checks cover console login, xterm typing,
mouse capture/release, and a 20-pixel title-bar drag with an unchanged
translated client image. A typed guest halt completes filesystem sync and
returns to the graphical OBP prompt. These are specific installed-disk and
desktop checkpoints; SX instruction coverage and full POST remain incomplete.
Audio, interactive serial,
clipboard-to-guest paste, and additional models are not provided yet. Host
networking is available as user-mode NAT: enable it in F9 > Network, then
configure Solaris `le0` with DHCP or a static address on the 192.168.0.0/24
network (gateway 192.168.0.1), for example:
`ifconfig le0 plumb 192.168.0.2 netmask 255.255.255.0 up` and
`route add default 192.168.0.1`. The OS already detects `le0`; it is the
guest network configuration that turns the interface up. The status strip shows
RUN, CPU, NET and attached-media activity lamps; NET pulses as Ethernet frames
cross the NAT.
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
overlay with temporary disk/CD targets and named-machine creation/recall. Smoke
tests use temporary preferences and NVRAM. The SDL smoke also saves and recalls
a two-CPU named machine. Unit tests exercise both CPUs running on the worker,
pause/resume, failed replacement, and a cold switch back to one CPU, plus
real-core lifecycle behavior with synthetic firmware, settings failure paths,
all seven SCSI IDs, disk/CD selection, input backpressure, shared modifier
keys, and failed NVRAM save/retry.
See [architecture](docs/architecture.md) for the frontend/core boundary.
