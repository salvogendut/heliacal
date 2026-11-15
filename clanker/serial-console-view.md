# Embedded serial view and disabled headless Display row

2026-11-15 follow-up on `feat/headless-pty-console` (Heliacal issue #1).
The user requested a greyed General Display row when headless, serial output
inside the Heliacal window, and an Advanced toggle for that view.

## Changes

- General's Display row shows Headless as a fixed/muted row. Enter/click cannot
  change the remembered CGsix/SX choice; disabling headless restores the row.
- Tinker-gated Advanced > Serial console view controls `show_serial_console`,
  a shared presentation setting defaulting true. Save applies immediately,
  without resetting hardware or touching the PTY.
- The machine worker feeds raw SCC A output to both the PTY and an 80x24 vt100
  parser. No scrollback; no terminal reply/clipboard/resize callbacks. The
  parser dependency disables default features so VTE OSC storage is bounded.
- Immutable screen snapshots are published at the existing cadence only when
  new bytes arrived. The UI redraw key is separate from framebuffer generation.
  SDL groups character runs by attributes and renders ANSI colours, underline
  and a static cursor with the existing ASCII font.
- Parsing continues while hidden. Cold replacement clears the screen; failed
  replacement retains it. The view is output-only; keyboard input uses the PTY.

## Validation

65 tests pass, including split CSI sequences, CR/backspace/tab, colours/cursor,
wrapping/scrolling, immutable snapshots/reset, oversized OSC handling, saved
view preferences with no hardware replacement, and identical PTY/screen output
through the real worker. Formatting and Clippy with -D warnings pass, as do the
SDL dummy/software smoke and release build.

Actual Xvfb/software UI checks with original SS-20 firmware:

- Headless Display is grey/fixed; turning headless off restores native SX.
- Original OBP emits ANSI clear/home/red text. The screen renders
  EMBEDDED-CONSOLE, red RED, and the `ok` prompt with a cursor.
- Advanced toggle Off returns to the connection panel. Output sent while
  hidden appears when toggled On; the PTY endpoint is unchanged and accepts
  commands throughout. No F5 reset is involved.
- A disposable Solaris 2.6 disk boots to login. Root's `TERM=vt100; clear` and
  HELIACAL-EMBEDDED-SOLARIS output render inside the window; commands arrive
  through the PTY. Clean sync/halt and host Quit=0 pass, with alias removal.

Evidence: `../helios/local/serial-view-2026-11-15/` contains the UI harness,
transcript and PNG captures. Source media and the live catalog are untouched;
the disposable disk was removed after acceptance. The original graphical SS-5
smoke passes at 98,574,336 outcomes, PC ffd0c49c. Its log is retained alongside
the other check logs. No test process or active alias remains.
