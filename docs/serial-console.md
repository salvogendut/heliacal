# Headless guests and serial consoles

Heliacal keeps its control window when a guest is headless. The machine has no
framebuffer or VSIMM; its console uses SCC channel A (TTYA), exposed as a Unix
PTY. The guest keyboard remains electrically present for original firmware
startup. The embedded console sends typed bytes to TTYA, not to the emulated
Sun keyboard. It does not grab or hide the mouse. CPU execution and device
time remain in Helios.

1. Enable **F9 > General > Tinker**.
2. In **Advanced**, turn **Headless guest** on. Optionally enter a **Serial PTY
   symlink**, such as `/tmp/heliacal-ss20`. Its parent directory must exist.
3. Save with **F9**. Start with **Ctrl+P**, or cleanly halt an already running
   guest and confirm **F5** to replace it with the saved hardware settings.
4. Connect to the endpoint shown in the control window:

   ```sh
   minicom -o -8 -b 9600 -D /tmp/heliacal-ss20
   ```

**Advanced > Serial console view** shows the serial console inside Heliacal's
window. It defaults to **On** and is gated by Tinker. Save applies this view
preference immediately, without a cold reset. Off returns to the connection
information panel; the PTY and terminal state keep running while hidden.

The embedded console is an 80-column, 24-row terminal with the original dark
background and light text, wrapping, cursor positioning, erase commands, ANSI
colours and underline. Guest-selected colours and inverse video remain supported.

Click inside the console or press **Ctrl+Enter** while the machine is running
to focus its keyboard input. The border and hint show the focused state.
Type normally; Enter, Backspace, Tab, Escape, arrow/editing keys and control
characters go to TTYA. Printable text uses the host keyboard layout and SDL
text composition; Backspace sends DEL and **Ctrl+H** sends BS. The cursor keys
respect the guest's normal/application cursor mode. Use `TERM=vt100` for
terminal-aware guest applications. This is a bounded VT subset, not a complete
xterm implementation.

**Ctrl+Enter** releases focus. Opening F9, a confirmation dialog, losing host
window focus, pausing, or hiding the view also releases it and discards queued
embedded input. A stopped/powered-off guest cannot take input; use confirmed
**F5** to cold-start it. Existing frontend shortcuts remain reserved, including
**Ctrl+P** for pause and **F9** for settings; Shift passes reserved function keys
to the guest. The mouse remains free to operate the frontend.

**Copy command**, beside the connection command, copies the full shell-quoted
minicom command to the host clipboard. Drag over the console to select text,
then press **Ctrl+Shift+C** to copy the selection. Without a selection that
shortcut copies the minicom command, including while the console view is hidden.
A selection holds its displayed page so new output cannot alter the text being
copied. **Esc**, scrolling or typing clears it. Copying never sends guest input;
**Ctrl+C** remains the guest interrupt character.

**Ctrl+Shift+V** pastes clipboard text into the focused console. CRLF and LF
become Enter, tabs and UTF-8 are preserved, and other control characters are
filtered. The guest's bracketed-paste mode is honoured. A paste must fit the
4 KiB input queue (including delimiters); an oversized paste is rejected whole.
Release focus to discard queued paste. Unmodified **Ctrl+V** still goes to the
guest, and F9 path editors retain their existing Ctrl+V shortcut.

The scrollbar and mouse wheel browse up to **10,000 saved lines**. Drag the
thumb or click its track to move by a page. **Shift+PageUp/PageDown** scroll a
page; **Shift+Home** goes to the oldest retained line and **Shift+End** returns
to live output. Unmodified PageUp/PageDown are still guest keys. Output follows
automatically at the bottom; while reading history, new lines keep the viewport
anchored until those lines age out of the bounded buffer. Typing or pasting
returns to live output. History is available while paused or stopped, and
survives hiding the console. A cold reset/replacement starts fresh history.
Terminal reset/erase-scrollback sequences also clear it; alternate-screen
applications do not fill the main history with redraws.

The embedded console and minicom both receive the original SCC A output;
showing the embedded screen does not consume bytes from the PTY. Both can send
input, so use one input source at a time to avoid interleaved commands.
The current ASCII font substitutes `?` for unsupported glyphs. Terminal-query
replies are not injected by the embedded view.

Use **9600 baud, 8 data bits, no parity, 1 stop bit**, with hardware and software
flow control off. `-o` skips modem initialization. Connect one terminal client
at a time. Leaving minicom with Ctrl+A, X leaves the guest running. Reconnect
using the same endpoint. Bytes sent while paused are held until resume, subject
to the bounded input queue. A cold reset or SCC receiver reinitialization
flushes pending host input so that old commands cannot leak into a new console.

**Open serial terminal** launches minicom in the first available supported
terminal (`x-terminal-emulator`, `gnome-terminal`, `konsole`, `xfce4-terminal`,
or `xterm`). Both programs must be on Heliacal's PATH. It passes the endpoint
as one argument without a shell. If the host has no suitable terminal/minicom,
the action reports that and manual connection remains available. A terminal
launched inside a container must see the same PTY namespace as Heliacal.

Use **Send serial BREAK** in Advanced to interrupt a running console. PTY byte
transport cannot forward minicom's host `tcsendbreak` ioctl; a NUL byte is not
used as a substitute. The action calls Helios's SCC BREAK API and waits for
FIFO space when releasing it. The pulse lasts at least one million execution
outcomes, matching the existing original-OBP probe; exact serial bit timing
is not modeled. Paused guests receive the action on resume.

## Configuration and lifecycle

These fields belong to each named machine and default to graphical/no alias:

```toml
headless = true
serial_pty_link = "/tmp/heliacal-ss20"
```

An empty link selects only the allocated `/dev/pts/N`. Relative link paths are
relative to the configuration file. `--headless`, `--graphical`, and
`--serial-pty-link PATH` override the selected machine for this session; CLI
paths are relative to the launch directory. Save makes overrides persistent.
Turning Tinker off hides these controls without changing the running machine.

The shared desktop setting `show_serial_console = true` controls the embedded
view; it is a presentation preference like smoothing, rather than per-machine
hardware. It is saved at the top level of the configuration.

Headless machines do not load a TurboGX ROM. The General **Display** row shows
**Headless** and is disabled while headless mode is on. The graphical `display`
choice is remembered and becomes editable again when headless mode is off.
Maximum RAM is 256 MiB for SS-5 and 512 MiB for SS-20/Axil. Switching an SS-20
draft from headless 512 MiB back to SX visibly adjusts it to 448 MiB;
Save and confirmed F5 apply the change. The machine label and NVRAM path do not
change when the console mode changes.

Cold replacement is transactional: a failed machine build or PTY reservation
keeps the previous machine and endpoint. Reset with an unchanged alias retains
the PTY so minicom stays connected. Changing the alias creates a new endpoint;
connect to the new path. Successful graphical replacement and normal shutdown
remove only the symlink owned by that PTY instance. A regular file, foreign
symlink, or alias used by another instance is never overwritten.

`<link>.heliacal-lock` is a small persistent coordination file, not a guest
image. It remains after shutdown to avoid lock-inode races. A stale alias is
recovered only when this record identifies its target and that target no
longer exists; otherwise choose another path or inspect the old link yourself.
All PTY descriptors close on exec. Non-Unix hosts report an unsupported PTY
error without replacing the running machine.

PTY transport uses nonblocking reads/writes and bounded 64 KiB input/output queues.
Embedded input has its own 4 KiB queue; it retries a full SCC without dropping
bytes and reports overflow instead of accepting a partial text event. Machine
and receiver resets flush both input sources. Generation checks reject stale
UI events, and each published prompt carries its matching generation.
A disconnected or slow reader never blocks emulation. Partial writes retain
the unsent suffix. If the output queue fills, its oldest bytes are discarded;
the control window displays the count. The small printable diagnostics view
is separate from the unmodified byte stream sent to the PTY.

## Original firmware console selection

For the three pinned original PROMs, cold construction selects `ttya` for both
`input-device` and `output-device` in headless mode, and `keyboard`/`screen` in
graphical mode. Other NVRAM bytes, including boot policy, device paths, IDPROM
and clock, are preserved. No PROM instructions are patched.

The frontend recognizes firmware by image size and CRC32, matching the core's
[pinned firmware references](../../helios/docs/firmware.md):

| Profile | Original image | Size | CRC32 | Input / output count offsets |
| --- | --- | ---: | --- | --- |
| SS-5 | `sun-ss5.bin` | 262144 | `f438e2ee` | `0x769` / `0x78b` |
| SS-20 | `ss10-20-v2.25r-rom` | 524288 | `105ba132` | `0x769` / `0x78b` |
| Axil 320 | `axil320-2-19.bin` | 524288 | `65290ef7` | `0x781` / `0x7a3` |

These counted strings have a one-byte length, up to 32 data bytes and a NUL.
The frontend changes only the new count/data/NUL, retaining unused tail bytes,
and XOR-adjusts the existing checksum byte at offset zero for those changes.
This format and delta were checked against images written by the original
OBP `setenv` commands and then booted again; they are firmware-specific, not
an SRAM hardware interface. Sun's published
[OpenBoot fixed-string access code](https://github.com/openbios/openboot/blob/1a08f436a931a5cedc10112a106dc1ae09d5507d/obp/pkg/confvar/fixed-access.fth)
also documents the counted-string convention (that newer source alone does
not establish these OBP 2.x offsets).

**Initialized NVRAM is required for automatic selection.** If the pinned
firmware's console fields are blank or malformed, headless construction fails
with an actionable message and preserves the existing guest. Initialize that
machine graphically first. This is not a full-POST implementation or a repair
for corrupt firmware state. An unrecognized PROM is left untouched and reports
that automatic console selection is unavailable on stderr; configure its own
serial console before using it headlessly. OpenBIOS instead uses its separate
CHRP environment and explicit console selection; see [firmware selection](../README.md#firmware-selection).

## Solaris disk paths and clock reporting

An installed SS-5 disk cannot be moved directly to SS-20/Axil: its `/dev/dsk`
links and `/etc/path_to_inst` refer to the SS-5 SBus map. A boot failure naming
`/usr`, missing `/usr/sbin/fsck`, or `no such device` can result from those
stale paths. It is independent of serial versus graphical console selection
and of the supported 512 MiB RAM size. Use a disk installed for the selected
model, or migrate a copy using the [Helios SCSI migration procedure](../../helios/docs/scsi.md).
Preserve the original disk; selecting another model does not rewrite guest paths.

With **Advanced > Accurate CPU clocks** on, named profiles use 85 MHz for
SS-5 and 75 MHz for SS-20/Axil. Off is the default fast 100 ns timebase,
which firmware measures as roughly 9–10 MHz. Save and confirmed F5 apply
the per-machine choice. The core
retains fractional cycles and shares device time across CPUs. SS-5 OBP reports
85.0 MHz and SS-20 reports 75 MHz; Axil OBP 2.19 truncates its calibration to
74 MHz on this 75 MHz timebase. These are functional clocks, without a full
pipeline or cache timing model.

## Terminal input reference

The normal/application cursor and editing sequences follow the
[XTerm control-sequence reference](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-PC-Style-Function-Keys).
The pinned `vt100` parser supplies the application-cursor flag; keyboard input
and explicit clipboard actions remain separate from terminal-query parsing.

## Pasting into a graphical guest

Capture the SDL guest display, then press **Ctrl+Shift+V**. Heliacal translates
ASCII text into balanced Sun Type 5 US key presses/releases, so paste works at
firmware prompts, OS consoles and focused graphical terminal applications.
It uses the guest's US keyboard mapping; non-ASCII text is rejected before any
keys are sent. Each paste is limited to 4 KiB. Newlines press Return; tabs press
Tab. This sends keystrokes, not a guest clipboard transfer.

The worker retries keyboard backpressure; the emulated 1200-baud keyboard
provides pacing in guest time. Existing held keys are released before paste,
and physical typing is suppressed while its queue is nonempty to avoid mixing
modifiers. **Ctrl+Enter**, focus loss, pause or reset cancels queued paste and
releases synthetic keys. Bytes already delivered to the device cannot be
recalled. No terminal or firmware memory is patched to inject text.
