# Testing, guest checkpoints and limits

[Back to Heliacal](../README.md) · [All guides](../README.md#documentation)

## Recorded guest checks

Validated on Linux with SDL3's software/dummy driver: the original Sun PROM
renders its SPARCstation 5 banner, reports **128 MB installed** and **Keyboard
Present**, and starts its memory test. Native SS-20 SX/CG14 also boots an
installed Solaris 2.6 disk through the graphical console and runs Xsun with
olwm and xterm. Linux/X11 input checks cover console login, xterm typing,
mouse capture/release, and a 20-pixel title-bar drag with an unchanged
translated client image. A typed guest halt completes filesystem sync and
returns to the graphical OBP prompt. These are specific installed-disk and
desktop checkpoints; SX instruction coverage and full POST remain incomplete.

An Axil 320 with TurboGX, 128 MiB RAM and two CPUs also ran Solaris 2.6 CDE
from an installed disk. Checks covered terminal input, both CPUs online,
CPU-bound loops with filesystem writes, capture/release, window operations,
clean halt, cold reset and persisted-file verification. This is a recorded
desktop workload; the two-CPU configuration remains experimental.

Solaris 2.6 graphical-console checks also cover 256 MiB on SS-5, 448 MiB on
SS-20/SX, and 512 MiB on SS-20/CGsix and Axil. The MBus cases use two CPUs;
these are bounded console, input and persistence workloads. See
[machine support](machines.md#supported-models) for the configuration limits
and SS-20/CGsix firmware setup.

Linux/X11 NetBSD serial-console checks cover typing, control keys, paste,
selection/copy, scrollback, clean guest halt and host quit. Graphical NetBSD
checks on SS-20/CGsix with OpenBIOS cover Forth and shell paste, including
shifted punctuation. The guest halted, but OpenBIOS continued running; that
recorded graphical run does not establish a successful host quit. These
checks do not establish NetBSD X11 or SMP compatibility.

The [README screenshots](../README.md#screenshots) are unmodified frontend
captures from these sessions: SS-20/SX olwm on 2026-11-13, Axil/TurboGX CDE
on 2026-11-15, and the headless NetBSD console after halt on 2026-11-16.

Native desktop file-picker behavior and non-Linux hosts need interactive
validation. Native Wayland input was not separately replayed. Audio and
additional machine models are not provided. Networking limits are documented
in [Networking](networking.md); OpenBIOS compatibility and limits are in
[Firmware selection](firmware.md).

## Checks

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
See [architecture](architecture.md) for the frontend/core boundary.
