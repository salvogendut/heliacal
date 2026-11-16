# RTC startup date (2026-11-16)

Heliacal's `fix/rtc-startup` pairs with the same Helios branch, based on
frontend `d81d21c` and core SX merge `c960604`.

The prior worker imported the saved year-2000 RTC without any host-time seed.
NetBSD 11.1 rejects it and substitutes filesystem time (November 2 in the
reported screenshot); Solaris also complained about the old date.

`rtc_host_time = true` is now the default for flat and named records, including
old configurations. The worker samples UTC at construction and calls the core's
explicit initializer on the replacement NVRAM copy. Firmware settings and IDPROM
survive; failed builds preserve the running machine. The Tinker-gated Advanced
`RTC at startup` row selects Host UTC or Saved guest time. The option requires a
new boot/confirmed F5 and changes hardware equality without changing identity
or NVRAM filenames. Save/Discard/Cancel remain transactional. CPU-rate settings,
core time advancement, guest reset and pause/resume are unchanged; there is no
continuous host-clock synchronization. The supported seed range is 1970..2067.

Three new frontend tests cover legacy/named defaults and persistence, gated
transactional editing, and real worker construction across all three profiles
with fixed host fixtures, saved clocks and rejected dates. All 78 tests pass,
as do formatting, Clippy, release build and SDL smoke. The rebuilt release's
isolated SS-20/SX/256 MiB firmware display smoke passes at 53,727,232 steps
(PC ffef0004). This is a firmware display check, not a full frontend OS run.

The core passes 665 default and 683 CHD/perf-counter tests plus both Clippy
configurations, formatting, demos and a release build. Disposable core guest
checks show NetBSD's November-16 boot date without the RTC/fallback warnings,
and Solaris 2.6 root `date -u` on November 16 plus a clean halt. The NetBSD
copy was stopped later during SSH key generation; no new login/halt acceptance
is claimed. Full evidence and limits are in
`../helios/clanker/rtc-startup.md` and ignored
`../helios/local/rtc-startup-2026-11-16/`.

The existing user application was left running. It must exit and relaunch to
load the new binary; the next guest construction uses Host UTC automatically.
User media/configuration were not overwritten. Disposable snapshots and probe
executables are removed after testing; compact source, logs and PNGs remain.
