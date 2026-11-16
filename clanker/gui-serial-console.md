# Interactive console, scrollback and clipboard

Heliacal issue #2, branch `feat/gui-serial-console`, 2026-11-16.

The embedded headless console accepts typing after a click/Ctrl+Enter. The
mouse remains free. Copy command copies the exact shell-quoted minicom recipe.
The user's final colour preference restores the original dark background and
light foreground, retaining ANSI attributes, inverse text and cursor contrast.

## History and selection

- The worker's vt100 parser retains 10,000 history rows. A `SerialFrame` copies
  only the visible 80x24 cells, wrap flags, cursor/input modes and history
  counts. Neither output publication nor selection clones the full history.
- Scrollbar dragging/track paging, wheel, Shift+PageUp/PageDown and
  Shift+Home/End work while running, paused or stopped. A coalesced,
  generation-tagged request does not step guest time. Positive offsets remain
  anchored under new output; the bottom follows output. Typing goes live.
- Alternate-screen applications do not fill main history. Guest reset/erase
  commands and cold replacement clear history. Hiding the view preserves it.
- Drag selects the displayed page. Selection holds one immutable viewport, so
  incoming output cannot change the text being copied. Ctrl+Shift+C copies it
  without ANSI escapes or row padding, preserving soft wraps and cell text.
  Without selection, that shortcut copies minicom. Esc/scroll/typing clears it.

## Input and paste

- SDL text events preserve host layouts/composition for serial input. Keydown
  encodes control/editing keys and normal/application cursors. AltGr does not
  inject Ctrl bytes. Ctrl+C and Ctrl+V remain guest controls.
- Ctrl+Shift+V serial paste normalizes line endings, retains tabs/UTF-8, filters
  other controls and honours bracketed-paste mode. Its bounded 4 KiB queue
  rejects overflow atomically and retries a full TTYA receiver without loss.
- Graphical paste validates ASCII before translating to balanced US Sun key
  events. It releases held modifiers, retries keyboard FIFO backpressure and
  uses the actual device's 1200-baud guest-time pacing. A second paste cannot
  discard the first. Physical typing does not interleave with queued paste.
- Release, focus loss, overlays, pause and replacement cancel queued embedded
  input/paste; already delivered device bytes cannot be recalled. PTY pause
  and reconnect behavior remain independent.
- Receiver reset/replacement flushes stale serial input. Generations are
  synchronized with output before publication; a worker regression caught
  and fixed the previous prompt/new-receiver generation race. Graphical input
  also rejects stale generations after machine replacement.

## Validation and evidence

96 frontend tests pass, including history anchoring/capacity/alternate screen,
immutable bounded snapshots, selection/wraps/Unicode, paste encoding and limits,
modifier balance, queue cancellation, stale generations, TTYA backpressure and
paused-worker scrolling without additional CPU steps. Formatting, strict
Clippy, release build and SDL dummy/software smoke pass. Original SS-5
OBP/TurboGX display smoke passes at 101,326,848 steps (not an OS-boot claim).
The Helios core is unchanged.

Private evidence: `../helios/local/heliacal-scrollback-2026-11-16/`.
The earlier base-console checks remain under
`../helios/local/heliacal-gui-serial-2026-11-16/`.

The serial SDL/NetBSD run passed 22 checks: firmware arithmetic, focus and
release, exact clipboard command with no guest bytes, external PTY input,
NetBSD typing/Backspace/Ctrl+C/top, multiline paste, exact copied row text,
wheel/page/drag/track scrolling, anchored output, paused history, oversized
paste rejection, halt and host Quit=0. Original-source SHA matched for that
run (`641d358887968a8f845652aeda17ffa9e10c266575a0fda608170c713f245cf5`).

Graphical SDL checks use SS-20/CGsix, 128 MiB, one CPU and automatic OpenBIOS,
with a disposable NetBSD CHD: Forth arithmetic through pasted keyboard events,
NetBSD shell paste, multiline commands and shifted shell punctuation. Native
framebuffer screenshots and OCR are retained. The OCR harness was corrected
for small fonts, punctuation and cursor/line segmentation; direct captures
confirmed the earlier apparent assertion failures were OCR errors.

A separate host Heliacal session was observed during graphical validation;
the user's original CHD changed during that interval. All test configurations
attach only their own distinct-inode clones. The final replay uses the frozen
initial test clone as its baseline, independent of the active user disk.
No test opened the user's CHD for writes, and no source disk or live catalog
was replaced. The graphical guest synced, unmounted and printed halted, but its OpenBIOS
firmware continued running. The replay harness incorrectly required frontend
STOPPED, timed out, and cleaned up its processes; no graphical host Quit=0
claim is made. The serial run independently verifies Quit=0. All three resumed
graphical paste assertions passed, and `host-window.png` shows the completed
guest halt. `graphical/acceptance.json` records these exact limits.

The frozen baseline clone's hash matched after the replay. Disposable CHDs,
private NVRAM/locks, temporary editing scripts and intermediate PPM/OCR images
were removed. Compact logs, acceptance JSON, scripts and screenshots remain.
The rebuilt release is ready; user firmware, live configuration and source disks
were not replaced or opened for writes by the tests.

Limits: fixed 80x24 VT subset and existing ASCII font; selection is limited to
one displayed page. Graphical paste assumes a US guest keyboard layout and
ASCII; no new Solaris desktop acceptance is claimed. Clipboard/input acceptance
uses isolated X11/software SDL; native Wayland was not separately replayed.
