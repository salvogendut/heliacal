# Automatic OpenBIOS fallback

Issue: Helios #8. Branch `feat/openbios-fallback` in both repositories.

The worker uses `helios::firmware::FirmwareImage` once per replacement. Empty
PROM paths select the core's installed fallback; explicit paths take precedence
and errors do not fall back. The active image/path is logged and shown in About.
The F9 PROM row displays `OpenBIOS (automatic)` when empty. CLI file filters/help
and the example configuration accept ELF firmware as well as original raw PROMs.

Flat and named-machine NVRAM retains its existing OBP filename. OpenBIOS uses
that name with `.openbios` appended. Switching formats saves the old state and
loads or creates the separate target; failed construction retains the running
machine. Tracked worker tests cover both directions for all profiles, invalid
replacement paths, separate environments and pre-allocation firmware limits.
OpenBIOS CHRP validation, contiguous RAM and native-SX rejection happen before
writable media attachment. Ordinary OBP console handling is unchanged.

Validation: 80 tests, formatting, Clippy with `-D warnings`, release rebuild,
SDL dummy/software smoke, and original-OBP display smoke on SS-5, two-CPU
SS-20/SX and two-CPU Axil/CGsix. Core: 683 unique default / 700 all-feature tests,
Clippy, formatting and four demos. Rust code did not change during the later
OpenBIOS source-port debugging.

Private evidence: `../helios/local/openbios-2026-11-16/`.

- `ui-{ss5,ss20,axil}-keyboard/6.png`: final automatic OpenBIOS image, 128 MiB,
  one CPU for SS-5 and two for SS-20/Axil, CGsix. Actual SDL capture and typing
  execute `1 2 + . cr`, visibly returning `3`; release/Quit return status 0.
- `ui-headless-keyboard/`: final automatic image with a disposable NetBSD CHD,
  PTY typing, shell/`uname`, checksum `3147149250 24 /openbios-test`, embedded
  serial screenshot, clean guest halt, host Quit=0 and owned-link removal.
- `obp-*-smoke.log`: original-firmware regressions. No live catalog or source
  disk was used as an output. Each run has private config, NVRAM, Xvfb and PTY.

The source port, build recipe and detailed Solaris/NetBSD core acceptance are
recorded in `../helios/clanker/openbios-fallback.md` and its human firmware docs.
Native SX still requires original OBP. Below-32-MiB or sparse DRAM populations
require OBP too. The supplied NetBSD GENERIC kernel remains UP. No replacement
firmware desktop, complete installation, Stop-A return or network-boot claim.

Initial acceptance fallback SHA-256:
`fda89b11311f24d14e6d4ef6f0086a0d5e2bb56bc2a01ab4041392635330d0c1`.
The rebuilt frontend resolves it automatically from the core's local firmware
directory. Completed test processes, disposable media/NVRAM, superseded builds
and editing scripts were cleaned; compact evidence and final firmware remain.

## Startup indication and timing follow-up

The 256 MiB SS-5 blank-screen report reproduced as delayed startup. After the
firmware's whole-RAM clear was optimized, first serial output was at 11.2 s and
the Forth prompt at 47.3 s (previously 43.6/78.6 s). The graphical prompt was
visible at 180 s before any injected key; keyboard arithmetic still returns 3.
TurboGX initialization remains slow. Both private frontend runs quit with 0.

`Snapshot::waiting_for_console` is set for each successful OpenBIOS cold boot.
The UI shows a startup/pause notice; serial bytes clear it for headless mode,
nonuniform decoded pixels for graphics. This is a presentation latch, not a
firmware-completion signal: initialization patterns can clear it before the
final prompt. Later blank guest screens do not bring it back, and no extra
framebuffer decoding occurs. A tracked regression covers blank/uniform frames,
first content, subsequent clears and headless isolation. The event loop redraws
when the flag changes. Original OBP has no notice.

Latest validation: 81 tests, formatting, strict Clippy, release rebuild, SDL
smoke and original-OBP SS-5 display smoke. Screenshots/timing and core full-RAM
reset tests are in `../helios/local/openbios-startup-review/`. The installed
fallback is now SHA-256
`084fa665e089edb0a00532f15bcc5fe383401f23f78ef1fd88dc8221bf161616`.
Source disks, live catalog and user NVRAM remain untouched. Temporary runs,
firmware sources/builds, scratch probes and private NVRAM were cleaned.

## Startup optimization follow-up

The sibling core's [optimization report](../../helios/clanker/openbios-startup-optimization.md)
supersedes the previous installed image/timings. Target `-O2` and a native
Forth dictionary-chain search reduce the same 256 MiB SS-5 measurement from
43.6 to 15.2 s for the serial prompt and 157.6 to 21.4 s for first graphics.
The Heliacal SS-5 graphical prompt is visible in the 25-second capture; SS-5,
two-CPU SS-20 and two-CPU Axil all display `3` after captured `1 2 + . cr`,
release input and quit cleanly. NetBSD CHD persistence and Solaris CD SMP
single-user boot were rerun against the exact optimized firmware.

Installed ELF SHA-256:
`665b42e56419fec7defb41cda68d027f6077cf52d9a36329082603a862b18947`.
Heliacal's notice no longer predicts several minutes. An empty `--prom ""`
override now clears the configured image instead of calling `absolute("")`.
The release binary is rebuilt; 81 tests, formatting, strict Clippy, SDL smoke
and original-OBP display smoke pass. Evidence and cleanup notes are in the core
report. Source images, live machine catalog and user NVRAM are preserved.
