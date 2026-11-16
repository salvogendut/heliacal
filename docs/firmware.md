# Firmware selection and OpenBIOS

[Back to Heliacal](../README.md) · [All guides](../README.md#documentation)

## Original system and framebuffer ROMs

Original PROM images are not bundled. The README lists the
[tested images, download sources and MD5 sums](../README.md#firmware-and-prom-images).
Keep firmware under ignored `local/firmware/`; downloading an archive does
not configure the machine automatically.

In **F9 > General**, select the system image in the model's **PROM** row and
the framebuffer image in **TurboGX FCode ROM**. A TurboGX graphical machine needs **both**:
the model's system PROM (or the installed OpenBIOS replacement) and the
separate 32 KiB FCode ROM for the 1 MiB TurboGX card. Extract
`sunw,501-2325.bin` from the linked `turbogx.zip`; the examples call this
file `turbogx.bin`. TurboGX+ ROMs, including `sun-tgx+-vesa.bin`, are incompatible.

Native SS-20 SX/CG14 uses the original SS-20 system PROM without a separate
framebuffer ROM. Headless machines also need no framebuffer ROM.
Save the settings, then boot with **Ctrl+P**; if a machine is already running,
firmware changes take effect after the next confirmed **F5** cold reset.

For example, after saving the files under the local names in the README:

```sh
md5sum local/firmware/sun-ss5.bin \
  local/firmware/ss10-20-v2.25r-rom \
  local/firmware/axil320-2-19.bin \
  local/firmware/turbogx.bin
```

Compare the results with the README table; run the command only for images
you downloaded. The sums identify the tested uncompressed files. The
SS-5 and TurboGX fetch helper also verifies pinned SHA-256 values:

```sh
cd ../helios
python3 tools/fetch_firmware.py sun-ss5
python3 tools/fetch_firmware.py turbogx
```

These commands save to the sibling Helios checkout's ignored
`local/firmware/`. Point Heliacal at those paths, as shown in the
[launch examples](building.md). The linked GitHub `ross225r.bin` is identical
to the tested `ss10-20-v2.25r-rom`; either filename works. The Axil image is
available from [Internet Archive](https://archive.org/download/axil320-2-19).
Its listed size and MD5 match the `axil320-2-19.bin` used in validation.

## OpenBIOS fallback

An empty **F9 > General > PROM** path (`prom = ""`) selects OpenBIOS for flat
settings and named machines. Select the PROM row and press Delete to clear an
existing path and return to automatic selection. Alternatively, use Ctrl+E,
Ctrl+U, then Enter to edit and clear it. Explicit missing/invalid images report an error; they do
not select a fallback. `--prom ""` clears the configured PROM for one session.
`--prom` remains a session override and accepts raw Sun
PROMs or validated SPARC32 OpenBIOS ELF files. **F9 > About** shows the firmware
and path actually loaded by the worker.

Build the pinned Helios OpenBIOS port in the sibling core checkout:

```sh
cd ../helios
python3 tools/build_openbios.py --jobs 4
```

OpenBIOS is installed separately, not bundled or downloaded by the frontend.
The shared loader searches the local firmware,
user data and system firmware directories; `HELIOS_OPENBIOS=/path/to/image.elf`
selects a specific fallback. See [build prerequisites, provenance and full
selection rules](https://github.com/salvogendut/helios/blob/main/docs/firmware.md#automatic-openbios-fallback).
OpenBIOS requires at least 32 MiB and contiguous installed DRAM (128 MiB works
on every model). It has no native SX console: select headless or CGsix, or
provide original OBP for SX. TurboGX still needs its separate FCode ROM.
During initialization, an **OpenBIOS / Console startup** notice replaces the
blank screen until console content appears. RAM clearing delays the first
serial bytes. The optimized port measured about 15 seconds to a serial prompt
and 21 seconds to first TurboGX banner pixels in a 256 MiB SS-5 core benchmark
(fast clocks); host speed and configuration affect startup time. The notice
disappears once and stays gone if the guest later clears its screen.
Graphical mode displays the Forth prompt on screen; headless mode uses TTYA.

Each machine's OpenBIOS NVRAM appends `.openbios` to its existing NVRAM filename.
OBP state is preserved separately, including when switching firmware on the
same named machine. Fresh OpenBIOS headless machines do not need a prior
graphical boot. They start at `0 >`; use `boot disk` or `boot cdrom`. The current
port's halt/abort callbacks power off instead of returning to an OBP monitor.
Serial and TurboGX graphical Forth consoles have been checked on all three
models, including captured keyboard input. NetBSD disk boot/persistence and
Solaris 2.6 CD single-user SMP boot are documented checkpoints; a general
replacement-firmware desktop or installation claim is not implied.
Original OBP remains the hardware-accuracy reference. Recorded compatibility
and limits are in the core firmware document linked above.
