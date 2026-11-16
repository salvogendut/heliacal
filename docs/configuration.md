# Configuration and saved state

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

Fresh NVRAM lets OBP initialize its defaults. The initial checksum message is
expected. The firmware tests the configured RAM (128 MiB by default) and may
automatically attempt network boot. After the memory test, capture input and use
**Pause+A** to interrupt it. Helios documents its graphical `ok` prompt
checkpoint; startup may take several minutes.
