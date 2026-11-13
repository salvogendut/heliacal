# Native SS-20 desktop acceptance, 2026-11-13

Helios completion evidence is in the sibling checkout under
`local/completion-2026-11-13/` and documented in
`../helios/clanker/ss20-desktop-completion.md`.

The release frontend was rebuilt against the completed core. Its 22 tests,
formatting, Clippy with `-D warnings`, and `make smoke` pass. The paused-screen
firmware hint now omits TurboGX when native SX is selected.

An isolated Xvfb display with SDL_VIDEO_DRIVER=x11 and
SDL_RENDER_DRIVER=software exercised the real SDL event loop via XTEST.
Original 2.25R OBP, native 8 MiB VSIMM-2 CG14, 128 MiB RAM, Type 5 keyboard,
and a private migrated Solaris 2.6 disk clone were used. No user preferences,
source firmware, or original disks were changed.

Console root login, xterm typing, capture/release, recapture, click-to-focus,
and a paced title-bar drag pass. Guest xwininfo reports the client moved
from +105+125 to +125+125; all 152,944 translated client pixels are identical.
Released typing is ignored; recaptured input writes CAPTURE-RECAPTURE-OK to
a guest file, independently read back from UFS. Typing sync and halt through
xterm returns to graphical OBP after a clean filesystem sync.

This validation exposed incomplete VSIMM identity in the core: revision-zero
MDI and a missing VBC configuration selected an old Solaris mode-switch path
that stalled when the framebuffer was the console. The core now models a
coherent VSIMM 2, including read-only MDI/VBC identity, video power/blanking,
and regression tests. No frontend workaround bypasses guest hardware.

The test-only rc2 X startup script and root shell live only on private ignored
clones. Full CDE login, other operating systems and non-Linux hosts are not
established by this workload.
