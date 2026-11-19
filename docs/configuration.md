# Configuration and saved state

## Experimental idle parking

`idle_parking` defaults to `false`; `--idle-parking` enables it for the next
boot. It is scoped to a single-CPU SS-20 Solaris idle boundary and reduces
measured idle worker CPU by roughly 69–80%. Parking is suppressed for 40 ms
after each successfully delivered serial byte, renewed only by another
successful delivery. This keeps measured command and character responses
responsive, but continuous serial input can keep the worker at full CPU:
observed command-phase usage was about 99–101%, versus 30–33% for parked
control without the grace. Quiet input returned to roughly 6.8–8.8% settled
idle CPU and improved command medians by about 12 ms over parking without the
grace. The grace expires unconditionally without waiting for echo or output.
All proof, wakeup and 2 ms wait limits remain active. Treat it as an
experimental default-off setting; the measurements are not universal latency
or CPU guarantees.

With a `jit-native` build, `execution_jit = true` or `--execution jit` selects
the opt-in native instruction bridge. It is default-off and currently covers
only supported ALU/SETHI instructions after the interpreter performs fetch and
device/interrupt handling. Unsupported instructions, control flow, memory,
traps and dual-CPU execution remain interpreted.

[Back to Heliacal](../README.md) · [All guides](../README.md#documentation)

The default config is `heliacal.cfg` under:

- Linux: `$XDG_CONFIG_HOME/heliacal/`, or `~/.config/heliacal/`.
- macOS: `~/Library/Application Support/heliacal/`.
- Windows: `%APPDATA%/heliacal/`.

`--config PATH` selects an independent configuration. Image paths in that
file are relative to its directory; CLI image paths are relative to the
working directory. CLI overrides, including `--ram-mib`, are saved only if you
choose Save in F9. `ram_mib` defaults to 128 and must be an exact valid total
for the selected model/display; named stanzas may omit it to inherit 128.
The `scsi` array contains exactly seven entries, in ID order (0 through 6),
each with `kind = "disconnected"`, `"disk"`, or `"cdrom"` and a `path`.
The early `disk`/`cdrom` config fields migrate to targets 3/6 on load.
`network = true` enables the user-mode NAT (default on). See
[the example configuration](../heliacal.cfg.example).

Unnamed configurations load and atomically save NVRAM per model: `<config-path>.nvram`
for SS-5, `<config-path>.nvram.ss20` for SS-20, and
`<config-path>.nvram.axil320` for Axil 320. Named machines use the state
paths described in [Named machines](machines.md#named-machines). The lock file
is `<config-path>.lock`. Concurrent instances cannot use the same config.
Choose a different config for another workstation instance. Keep
firmware/disks under ignored `local/` if storing them in this repository.

File-picker locations are saved separately in `<config-path>.picker.toml`.
They are shared by the named machines in that catalog and survive restarting
Heliacal or discarding a settings draft. Selecting a file updates this history
without saving hardware settings or attaching media. Cancelling a picker
does not update it. Deleting this optional file resets the remembered locations.

Fresh NVRAM lets OBP initialize its defaults. The initial checksum message is
expected. The firmware tests the configured RAM (128 MiB by default) and may
automatically attempt network boot. After the memory test, capture input and use
**Pause+A** to interrupt it. Helios documents its graphical `ok` prompt
checkpoint; startup may take several minutes.
