# CHD frontend implementation

Developed with the paired core on `feat/chd-images`. Core details and guest
acceptance are in [Helios progress](../../helios/clanker/chd-images-progress.md).

Heliacal enables the optional core CHD feature. Settings validation uses
logical-media inspection for all active and inactive named-machine records;
Save never creates a writable child. The existing worker construction retains
the running guest on replacement failure.

F9 → Media → New blank CHD disk opens an independent form for destination,
capacity (default 2048 MiB) and unused target. Ctrl+E supports typed paths;
Enter uses SDL's save dialog, suggesting `new-disk-scsiN.chd` for the selected
target. Creation uses a separate worker and the core's
no-overwrite helper. Editing/Quit wait for the short sparse creation to finish.
A success changes only the settings draft. Cancel before creation changes
nothing; Discard afterward keeps the explicit file. Save and confirmed F5
apply the new target, preserving labels/NVRAM/unrelated targets.

75 tests pass, including new form/capacity/target/discard/named-record and
bad-companion tests. Clippy, formatting, release rebuild and SDL smoke pass.
Smoke executes creation → draft → discard → reattach → save, then existing
named-machine checks. Real SS-5 graphical firmware smoke passes (98,172,928
steps), using an explicit isolated config and --graphical. It is not an OS claim.

Actual Xvfb GUI acceptance is under Helios `local/chd-development/gui3/`:
creation, saved target 1, confirmed F5, OBP probe, Solaris reconfiguration boot
and format/label/verify all pass, followed by clean halt, UI Quit (status 0)
and removal of the owned PTY symlink. Independent chdman extraction confirms
the created Sun label magic dabe and XOR checksum zero. Initial harness
attempts needed `boot disk -r` for new /dev nodes and the observed
`Label it now?` prompt. The completed private disk copies have been removed;
logs, screenshots, extracted label and hash manifest remain.

The user's live catalog and guest were not changed or restarted. The release
binary has CHD enabled; an already running old process needs reopening to
show the new menu. Native save-dialog selection itself and non-Linux hosts
remain unverified; typed-path creation and the existing SDL picker callback
mechanism are covered separately.

After the NetBSD investigation, Heliacal was rebuilt against the final
floppy byte-probe completion, V8 reserved-ASR readback and absent-coprocessor
trap corrections. All 75 frontend tests, formatting, Clippy and SDL smoke
pass (`netbsd/final-heliacal-checks.log` in the core evidence root). This adds
no floppy-image UI or media engine. The headless core has now completed a
NetBSD 11.1 SS-20 CHD installation,
normal GENERIC root login, source/persistence checksums and clean halt.
See the core handoff for the retained disk and configuration. This does not
claim additional NetBSD GUI acceptance in Heliacal.
