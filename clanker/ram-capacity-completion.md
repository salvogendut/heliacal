# Configurable RAM completion

2026-11-15; configurable RAM, approved by the user for integration into both
`main` branches and closure of Helios issue #3. See the authoritative
[Helios completion review](../../helios/clanker/ram-capacity-completion.md)
for the core corrections, full guest matrix, sources and cleanup manifest.
This supersedes the remaining-acceptance statements in the slice C handoff.

## Final implementation and tests

RAM uses the core's allocation-free topology validation and exact choice list,
including inactive named machines and final CLI overrides. Construction selects
the installed video before allocating RAM. A RAM change uses confirmed cold
replacement and preserves the label/NVRAM identity; failed construction keeps
the current guest.

The Axil OBP 2.19 profile now uses 16/64 MiB banks, with eight slots and a
512 MiB maximum. 32 MiB total uses two 16 MiB banks; 432/480/496 MiB are rejected.
This is a firmware/profile restriction, not a claim about all physical Axil
module inventories. SS-20 still permits 16/32/64 MiB modules, with corrected
split mapping for 32 MiB DSIMMs.

Six independent review fixtures were promoted into `src/ram_acceptance_tests.rs`:
exact RAM enumeration, model/display transitions, transactional drafts,
real memory access at 192 MiB across confirmed 128-to-256 MiB replacement,
failed replacement preservation, and named/CLI validation before startup.
The live F9 check exposed overlapping title/dirty labels and an overlong footer;
both labels now fit the overlay.

## Acceptance

- 48 tracked frontend tests, formatting, Clippy `-D warnings`, release build and
  SDL smoke pass against the final core (624 normal / 626 perf-counter tests).
- Original SS-5 256 MiB firmware display smoke passes at 96,614,400 steps.
  This separate smoke is not an OS-boot assertion.
- SS-5/CGsix 256 MiB, SS-20/CGsix 512 MiB and Axil/CGsix 512 MiB: visible
  Solaris 2.6 maintenance shells, memory/CPU reporting, guest typing,
  release/recapture with absent leak-file assertion, clean halt to OBP,
  and host Quit status 0. The MBus cases run processors 0 and 2.
- Axil additionally passes confirmed F5 reset/reboot, rereads its disk marker
  (`879949744 18 /heliacal-ram-gui`), and discards CPU/RAM drafts without
  changing the 512 MiB saved configuration. SS-5 also discards an 8 MiB draft
  while retaining the 256 MiB saved/running configuration.
- Earlier independently reviewed native-SX evidence shows root login at
  448 MiB, processors 0/2 online, typed `HELIACAL-MAX-CONSOLE-OK`, and capture
  release. The final core's unchanged maximum-size mapping retains that result.
- SS-20/CGsix requires normal OBP `setenv sbus-probe-list f0123` to skip the
  unimplemented DBRI placeholder. The default also panics at 128 MiB; this
  is independent of RAM size. No firmware or Solaris patch was used.

Evidence is under Helios `local/ram-completion-2026-11-15/` and the earlier
RAM acceptance/review directories. Source disks, ROMs and the user's live
catalog retain their original hashes. Temporary disks/NVRAM, builds and
redundant captures are removed after retaining compact logs and assertions.
No maximum-size full CDE interaction or exhaustive POST claim is added.
