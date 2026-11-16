# README refresh — 2026-11-16

Reorganized the README following the sibling Prismatic entry point, moving
detailed usage into `docs/`. Retained the logo and added three real PNG
captures without image editing. Sources in the sibling Helios checkout:

| Published asset under `images/` | Original capture under `../helios/local/` |
| --- | --- |
| `solaris26-ss20-sx.png` | `completion-2026-11-13/desktop-input.png` |
| `solaris26-axil320-cde.png` | `smp-slice4-2026-11-14/heliacal-axil/input-final.png` |
| `netbsd-ss20-serial.png` | `heliacal-scrollback-2026-11-16/halted.png` |

Each copied asset was compared byte for byte with its source. Captions follow
`ss20-desktop-acceptance.md`, `gui-serial-console.md`, and the sibling core's
`clanker/smp-progress.md` and `clanker/smp-slice4-review.md`.

Computed MD5 from the four tested files under `../helios/local/firmware/`.
Their SHA-256 values match the existing core firmware documentation and
fetch manifest. Public GitHub API checks using `gh` in `my-distrobox` verified:

- `andarazoroflove/sparc` at `a6f20b0740cfb4440d36ac04d7ad67e0207a73d4`
  contains `ss5.bin` and `ross225r.bin`. Their Git blob hashes match the
  local `sun-ss5.bin` and `ss10-20-v2.25r-rom`, respectively. The latter is
  also identical to the local `ross225r.bin`.
- `abdess/retrobios` at `ae7feba5398fb1258b381d2a0a1c7779f7630d5f`
  contains `bios/Arcade/Arcade/turbogx.zip`, blob
  `2229515edb3888c3357a5324f11b80437b76109a`. Downloaded that blob to
  `/tmp`, verified its Git blob hash and compared the extracted
  `sunw,501-2325.bin` with the local `turbogx.bin`: identical.
- The user supplied the [Internet Archive source](https://archive.org/download/axil320-2-19)
  for Axil `axil320-2-19.bin`. Checked the item's metadata through the existing
  distrobox container: its size is 524288 bytes and MD5 is
  `374ea922f97d4a7a4d37b378ae74e486`, matching the tested local image. The
  README and firmware guide now link to that archive.

The user clarified that the README should describe the current **installed
OpenBIOS fallback**, rather than adding firmware bundling. No original ROMs
or OpenBIOS binaries were added, and no emulator behavior changed.

Validation: local documentation links and heading anchors, Helios links
against sibling sources, balanced code fences, parsed TOML examples/config,
the four README MD5 values, copied image equality, and `git diff --check`.
Cargo and guest smoke tests were not rerun for this documentation-only change.
