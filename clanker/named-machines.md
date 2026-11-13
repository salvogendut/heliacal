# Named workstation configurations, 2026-11-13

The implementation follows Prismatic's named-machine catalog pattern, adapted
to Heliacal's flat hardware schema and one-window configuration lock. The
selected `machine_label` loads a `[machines."label"]` stanza. Desktop settings
remain global. The catalog captures model, display, RAM, PROM, TurboGX ROM,
all seven targets, disk access and NAT. Disk/CD shorthand also works inside
a stanza; saved files expand it to the complete target array.

F9 General opens on Machine label. Enter/click cycles the sorted catalog;
Save machine as... creates a new entry without overwriting its source. Name
errors keep the text editor open. Save/Discard semantics remain unchanged:
a selection is only a draft until Save, and an existing guest switches only
through confirmed F5 cold reset. Window title/status identify the actual
worker until replacement succeeds. --machine-label selects a stanza before
other CLI overrides; no launch-time config rewrite is performed.

The first label created from unnamed settings adopts legacy NVRAM. Other
labels use a complete hex encoding under <config>.nvram.machines, retaining
per-model suffixes. Worker reset compares the full state path rather than
only the model. Tests prove two SS-5 labels keep different NVRAM bytes,
same-workstation reset retains live firmware state, and failed replacement
keeps the previous paused guest/name/state target. Source disk paths are
references, not disk clones.

Validation: 29 frontend tests, formatting, Clippy -D warnings, extended SDL
smoke and release build pass against Helios f560056. The SDL smoke creates,
saves and recalls labels and checks the persisted catalog. The native menu
capture was inspected for layout. Original-firmware smoke uses a diskless
catalog and temporary NVRAM; artifacts are in the sibling Helios checkout at
local/named-machines-2026-11-13/. This tests configuration selection and display
startup for SS-5, native SS-20 and Axil 320; all three pass. This is not a new
OS compatibility claim. No source disks, firmware or user
preferences were modified.
