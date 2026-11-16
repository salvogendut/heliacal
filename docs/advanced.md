# Advanced settings and clocks

[Back to Heliacal](../README.md) · [All guides](../README.md#documentation)

Enable **F9 > General > Tinker** to expose clock, headless/PTY and diagnostic
controls in Advanced. Disk access is available without Tinker. See
[media](media.md) for disk write policy and [serial consoles](serial-console.md)
for headless setup, PTY aliases, terminal launch and serial BREAK.

## CPU clock policy

For faster boots, **F9 > Advanced > Accurate CPU clocks** defaults to **Off**.
Enable **General > Tinker** to expose it. Off uses the legacy 100 ns timebase;
firmware may report roughly 9–10 MHz. On uses nominal 85 MHz for SS-5 and
75 MHz for SS-20/Axil (Axil OBP rounds down to 74), which requires more
interpreted instructions during guest timer waits. It does not model full
pipeline/cache timing or improve host throughput.

The choice is stored per named machine as `accurate_cpu_clocks = false`.

Save applies it to the next boot; an existing guest requires confirmed **F5**
replacement so firmware and Solaris recalibrate. Discard/Cancel leave the
running clock unchanged. Labels, NVRAM identity and disk paths are retained.

## RTC at startup

**F9 > Advanced > RTC at startup** defaults to **Host UTC**. Enable
**General > Tinker** to expose it. Each boot or confirmed F5 cold replacement
sets the RTC calendar to current host UTC, preserving the machine's firmware
settings and identity. Old configurations also use this default. To keep a
historical or guest-set date, select **Saved guest time** (`rtc_host_time = false`).
The choice is saved per named machine and takes effect at the next boot/F5;
saving a draft does not change a running guest's clock. Pause/resume and guest
resets retain the clock. After startup, time advances with the emulated machine;
this is not continuous host-time synchronization and does not change CPU speed.
This avoids the year-2000 initial clock and NetBSD's fallback to the disk's older
filesystem date.
