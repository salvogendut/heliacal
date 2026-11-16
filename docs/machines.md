# Machine settings and saved labels

[Back to Heliacal](../README.md) · [All guides](../README.md#documentation)

## Supported models

The machine is selectable as **Sun SPARCstation 5**, **Sun SPARCstation 20**,
or **Axil 320**. All three use sun4m and a **Sun Type 5 US keyboard**, with
**128 MiB RAM by default**. In graphical mode, SS-5 and Axil 320 use a **TurboGX (CGsix)** SBus
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
[core configuration notes](https://github.com/salvogendut/helios/blob/main/docs/machines.md#ss-20-cgsix-console-configuration).

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

## Named machines

Save a complete workstation under **F9 > General > Save machine as...**.
Enter a label, press **Enter**, then **F9** to save. Labels accept 1–32
printable ASCII characters. An existing label is rejected; select that
machine first to update its settings.

**F9 > General > Machine label** cycles saved machines alphabetically with
Enter or a click. Each label recalls the model, CPU count, display, headless/PTY options, PROM and TurboGX
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
disk = "/path/to/solaris26-ss20-scsi3.raw"
cdrom = "/path/to/solaris26.iso"
network = true

[machines."SS5 Solaris 7"]
model = "sparcstation5"
prom = "/path/to/sun-ss5.bin"
cgsix_rom = "/path/to/turbogx.bin"
disk = "/path/to/solaris7-ss5-scsi3.raw"
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

See [firmware selection](firmware.md), [media](media.md), and
[configuration and state](configuration.md) for image and NVRAM handling.
