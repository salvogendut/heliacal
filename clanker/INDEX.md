# Agent notes

- [Clock, serial boot and CDE follow-up](clock-serial-cde-followup.md) — compatible TurboGX ROM validation, selected CPU clocks, recovered Axil disk and rebuilt-core acceptance.

- [serial-console-view.md](serial-console-view.md) — greyed headless Display row, live Advanced serial-view toggle, bounded ANSI terminal output, PTY fan-out and UI/Solaris validation.

- [headless-pty-completion.md](headless-pty-completion.md) — implemented headless/TTYA PTY controls, original-OBP console selection, minicom/Solaris acceptance, lifecycle and cleanup.

- [headless-pty-console.md](headless-pty-console.md) — issue #1: Tinker-gated headless guest startup, raw serial PTY, stable alias, optional minicom launcher and acceptance slices.

- [ram-capacity-completion.md](ram-capacity-completion.md) — final core corrections, 48 frontend tests, maximum-RAM graphical consoles, input/reset/draft checks and cleanup.

- [ram-capacity.md](ram-capacity.md) — issue #3 frontend handoff: shared RAM validation, F9/named-machine selection, actual allocation, cold replacement and acceptance.
- [ram-capacity-slice-c.md](ram-capacity-slice-c.md) — configurable RAM frontend slice: core-backed choices, selected allocation/video, cold replacement, CLI, tests and smoke evidence.

- [cpu-selection.md](cpu-selection.md) — one/two-CPU settings, named machines, scheduled worker integration, original-firmware smoke and remaining SMP acceptance.

- [build-environment.md](build-environment.md) — reuse the existing local Rust/SDL3 setup.
- [led-strip-and-unsaved-dialog.md](led-strip-and-unsaved-dialog.md) — Prismatic-style LED strip, F9-only save, and helios activity counters.
- [disk-access.md](disk-access.md) — read/write hard-disk default and the Advanced read-only toggle.
- [mouse-input.md](mouse-input.md) — Sun Mouse Systems packets on SCC channel B and captured SDL pointer forwarding.
- [machine-selection.md](machine-selection.md) — SS-5, SS-20 and Axil 320 profiles, firmware and per-machine NVRAM.
- [ss20-desktop-acceptance.md](ss20-desktop-acceptance.md) — native CG14 graphical-console boot, actual SDL typing/mouse/capture checks, clean guest halt, and final core integration.
- [named-machines.md](named-machines.md) — labeled configuration stanzas, F9 recall/Save As, CLI selection, independent NVRAM and validation.
