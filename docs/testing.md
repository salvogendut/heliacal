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

The [README screenshots](../README.md#screenshots) are native 1152×900 guest
framebuffer captures from these sessions: SS-20/SX olwm on 2026-11-13 and
Axil/TurboGX CDE on 2026-11-15. The original PPMs were converted losslessly
to PNG, preserving every pixel without scaling or frontend notifications.

Native desktop file-picker behavior and non-Linux hosts need interactive
validation. Native Wayland input was not separately replayed. Guest audio and
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
Picker checks cover separate firmware/disk/CD history, persistence across
restarts and Discard, relative and missing paths, and new-disk destinations.
The SDL smoke also verifies that an empty picker result leaves both the
draft and saved picker history unchanged.
Desktop tests cover proportional resizing and its limits, host keyboard
layouts, STOPPED guests quitting without confirmation, NVRAM save/retry and
PTY cleanup, and confirmation for running or paused guests.
See [architecture](architecture.md) for the frontend/core boundary.

## Sampling JIT phases in the machine worker

Build with `cargo build --release --locked --features jit-profile` and set
`HELIACAL_JIT_PHASE_DIR` to a fresh private directory for this process.
Select the execution mode normally with `--execution interpreter|jit`.

To request a sample, atomically rename a file containing a new decimal
sequence number to `request` inside that directory. Wait for `sample.toml`
to report the matching request number and process ID. The reply contains the
worker phase and outcome count, virtual time, actual JIT setting, and
cumulative per-CPU counters. The worker processes requests between batches
and while paused. Use one outstanding request at a time. Capture and retain
the reply before requesting another sample.

Subtract counts only between samples from the same run and CPU epoch. A
replacement machine or a reset of that CPU requires a new baseline. Use
the ordinary `jit-native` feature for release timings to keep diagnostic
costs out of comparisons.
