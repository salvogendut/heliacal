# Writable disk access

The user requested read/write hard-disk attachments by default, with an Advanced
toggle for read-only operation. The frontend now has a persisted
`disk_read_only` setting (false by default). Populated hard-disk targets call
Helios `ScsiBus::attach_disk_rw`; CD-ROM targets always call `attach` and remain
read-only. Settings saving validates write access for read/write disks before
saving. The Advanced tab is always visible; Tinker still controls the diagnostic
block (core PC, step count and recent serial A output). Disk access changes are
hardware changes and apply on the next cold reset.
