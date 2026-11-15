# Clock, serial boot and CDE follow-up

2026-11-15, `fix/serial-boot-clock-followup`, paired with Helios
`fix/profile-clock-timing`. Full diagnosis and core evidence are recorded in
[Helios's handoff](../../helios/clanker/clock-serial-cde-followup.md).

Heliacal now shows the selected 75 MHz SuperSPARC-II profile for SS-20/Axil.
Settings save calls the core's allocation-free TurboGX ROM validation and
transactionally rejects the known 4 MiB TurboGX+ images; construction already
creates the card before guest RAM/media. Headless configuration ignores an
unused graphics ROM. A renamed, synthetic CRC fixture covers save rejection
without publishing an invalid config or bundling firmware.

The worker still owns all guest execution and time. The frontend selects a core
clock policy during construction; UI refresh never changes guest time. No
firmware instructions are patched. The release includes optional fractional
85/75 MHz core clocks, corrected SS-5 speed pins, timer phase handling and the
keyboard reset-response correction discovered during actual Solaris boot.

69 tests, formatting, Clippy with warnings denied, release build and SDL smoke
pass. Original-SS5 firmware display smoke passed before the final keyboard
correction; final boot/input checkpoints are tracked in the core handoff.
Axil 512 MiB final real-PTY boot reached root, verified /usr and psrinfo, printed
HELIACAL-AXIL-CLOCK-ACCEPTED, and halted cleanly. Host Quit returned 0 and the
owned PTY alias disappeared. The unknown-keyboard/layout warning is absent.

Local user configuration was backed up while holding its config lock, then
updated to use 1 MiB turbogx.bin and the separately recovered Axil disk. Other
preferences, labels and NVRAM are unchanged. Original SS-5 disk paths caused
the Axil /usr failure; 512 MiB/headless was not the cause. Testing used private
Xvfb sessions, temporary catalogs and disposable disks under the core's
local/serial-clock-review-2026-11-15/. Final cleanup/acceptance is recorded there.


The user requested fast clocks as an Advanced option after observing boot cost.
Accurate CPU clocks is now Tinker-gated and defaults Off. Its boolean is
captured/restored in NamedMachine, included in hardware_eq, and applied before
media construction through the core's explicit clock-policy reset. Existing
records default to Fast. The UI provides a cold-reset hint and moves diagnostics
below the new row. Tests cover old-record defaults, named-machine roundtrips,
NVRAM path retention, isolated draft edits and both clock policies for all
profiles (including dual MBus CPUs). Required frontend checks pass at 69 tests.
Hardware-rate and fast-mode guest results are in the core handoff.

Final private SDL acceptance confirmed the Advanced Off → On → Off flow,
including Save and confirmed F5: SS-5 module-info read 10 → 85 → 10 MHz.
The restored fast-mode guest reached CDE, accepted FAST-CDE-OK terminal input
and paced mouse motion, cleanly halted to OBP, and exited Heliacal with status 0.
Completed disposable disks/builds were removed; compact evidence and the
separate recovered Axil disk remain.
