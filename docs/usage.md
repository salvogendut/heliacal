# Controls, capture and desktop settings

[Back to Heliacal](../README.md) · [All guides](../README.md#documentation)

| Key / action | Effect |
| --- | --- |
| Ctrl+P | Boot, pause, or resume |
| Click guest display / Ctrl+Enter | Capture / toggle keyboard and mouse capture; headless console takes keyboard focus only |
| Copy command | Copy the active headless minicom command to the host clipboard |
| Ctrl+Shift+C | Copy selected serial text, or the minicom command when nothing is selected |
| Ctrl+Shift+V | Paste into the focused serial console or captured graphical guest |
| Wheel / scrollbar / Shift+PageUp or PageDown | Browse serial history; Shift+End returns to live output |
| Pause + A | Sun Stop-A; F13 can also serve as Stop |
| F4 | Save the guest framebuffer as a native-resolution PPM in the working directory |
| F5 | Confirm a cold reset using the saved machine settings |
| F9 | Open settings / save settings |
| F11 | Toggle fullscreen |
| F12 / window close | Confirm quit and save NVRAM |
| Shift + reserved function key | Pass the shifted function key to the guest while captured |

In options, use **Left/Right** or **Tab/Shift+Tab** to switch sections,
**Up/Down** to select a row, and **Enter** to choose a file or toggle it. Tabs
and rows are clickable. For paths, **Enter** or **Ctrl+B** opens the native file picker
with firmware or disk/CD filters; **Ctrl+E** opens text entry, **Ctrl+V**
pastes, **Ctrl+U** clears, and **Delete** detaches the selected image when
outside text entry. **Escape** cancels text entry; with a changed draft it
offers **Save**, **Discard**, or **Cancel**. Confirmation dialogs default to
Cancel; use arrows and Enter, or click a choice.

File pickers remember the last chosen file across restarts, with separate
locations for firmware (system PROM and TurboGX ROM), hard disks, and CDs.
A row with an existing image starts there; an empty row uses its remembered
location. If a file or directory has disappeared, the picker uses the nearest
existing parent directory. New blank disks reuse the last disk directory
with a fresh `new-disk-scsiN.chd` suggestion. Cancel leaves the history unchanged.
History is saved when you choose a file, independently of Save/Discard;
machine settings and attachments still change only through the settings draft.

Display offers smoothing, scanlines, and shortcut hints. The bottom strip shows
RUN, CPU, and configured hard disk/CD activity; shortcut hints sit above it and
can be hidden. **Advanced** selects hard-disk read/write or read-only access.
**General > Tinker** enables the Advanced diagnostics block with the core PC,
step count and a recent printable serial A excerpt. **Advanced > Serial console
view** toggles the interactive serial console for headless guests; Save
applies it immediately. Disk-access changes apply on the next cold reset.

Settings remain a draft until **F9** validates and saves them successfully.
Render options such as smoothing and scanlines apply immediately. Model,
hardware display, headless/PTY options, RAM, firmware, media and disk-access changes apply at the
next **F5 cold reset**; saving them does not interrupt a running guest.
Choosing **Discard** closes the draft without changing the machine. Reset
replaces guest RAM and retains NVRAM; shut down a guest OS before resetting or
quitting.

## Clipboard and guest input

Serial keyboard input is available through both the focused embedded console
and the PTY. **Ctrl+Shift+V** also pastes into a captured graphical guest through
the emulated Sun keyboard, using its US layout (ASCII). Serial paste accepts
UTF-8; each paste is limited to 4 KiB, and newlines act as Enter. Release input
with **Ctrl+Enter** to cancel queued paste.

## Headless quick start

Enable **F9 > General > Tinker**, then **Advanced > Headless guest**. An optional
**Serial PTY symlink** gives the machine a stable path, for example
`/tmp/heliacal-ss20`. Save and boot, or confirm F5 after cleanly halting the guest.
The Heliacal control window remains open and shows an interactive 80x24 serial
console with the original dark colour scheme, the active endpoint and minicom
command. Click the console or press **Ctrl+Enter** to type directly to TTYA.
**Ctrl+Enter** releases keyboard focus; the mouse remains available for the UI.
Use **Copy command** for the minicom command. Drag across console text and use
**Ctrl+Shift+C** to copy it, or **Ctrl+Shift+V** to paste into the focused
console. **Ctrl+C** goes to the guest. The scrollbar, mouse wheel and
**Shift+PageUp/PageDown** browse up to 10,000 saved lines; **Shift+End** returns
to live output.
**Advanced > Serial console view** hides or shows that screen on Save, without
restarting the guest. **Open serial terminal** and **Send serial BREAK** are
also in Advanced. No framebuffer ROM is needed; SS-20 headless supports 512 MiB RAM.

Original firmware needs initialized NVRAM; initialize a new machine graphically
first. Existing boot settings and machine identity are preserved. See
[serial console setup, lifecycle and firmware limits](serial-console.md)
for the CLI/config options, terminal requirements and reconnect behavior.

See [machine labels](machines.md#named-machines), [firmware selection](firmware.md),
and [Advanced settings](advanced.md) for the saved machine options.
