# SCSI targets, CHD disks and new images

[Back to Heliacal](../README.md) · [All guides](../README.md#documentation)

## SCSI targets

**F9 > Media** exposes **SCSI IDs 0–6**. Each target can be a hard disk, a
CD-ROM, or disconnected; **ID 7 is reserved for the host adapter**. Press
**Space** on a target, or click its type, to cycle the device type. Press
**Enter** or click its image path to open the system file picker (Ctrl+B does
the same); **Ctrl+E** lets you type a path instead.
Empty targets start the picker at the last chosen disk or CD location, with
separate histories that persist across restarts. Targets with an image
already selected start at that image.
Use **Delete** to detach and clear a target. Multiple disks and CDs may be
attached simultaneously. Changes take effect on the next confirmed cold reset.
The original OBP `probe-scsi-all` discovers attached targets on SS-5 and now
also through the SS-20/Axil standalone IOMMU.

Targets default to empty, with hard disk/CD-ROM types preselected at IDs 3/6.
Hard disks use read/write access by default; CDs never use writable access.
Selecting an image for a disconnected target defaults to **CD-ROM at ID 6**
and **hard disk at IDs 0–5**, including after Delete detaches an image.
A disconnected ID 6 also uses the remembered CD directory in the picker.
An explicitly selected or saved device type is preserved; use **Space** to
change it. Cycling a target to disconnected retains its path for later use.
Saving checks the block size and readability
of each connected image, plus write access for read/write hard disks.

Optional `--disk disk-scsi3.raw` selects ID 3; `--cdrom install.iso` selects ID 6.
These shortcuts leave other targets unchanged. Images must contain whole
512-byte disk blocks or 2048-byte CD sectors. Hard disks default to **read/write**
so installed systems can update their file systems; CD-ROM images remain
read-only. Select **Advanced > Disk access** to make all hard disks read-only.

## CHD disks and new images

Heliacal enables Helios's CHD hard-disk backend. Existing raw disks and cooked
ISO CDs continue to work. CHDs must be v5 hard disks with 512-byte sectors;
CHD CDs/DVDs/audio and arbitrary parent chains are not supported.

F9 → Media → **New blank CHD disk...** lets you choose the destination,
capacity in MiB (2048 by default), and an unused SCSI target. Enter chooses a
path in the save dialog; Ctrl+E also lets you type it. The suggested filename
includes the selected target, such as `new-disk-scsi3.chd`. Keep the intended
SCSI ID in custom disk names. The save dialog reuses the last disk directory
with this new filename. Attach new disks at their intended ID. Select
**Create blank disk** to create an uncompressed, sparse, zero-filled CHD. Existing files are
never overwritten. The form can be cancelled before creation; while the
short creation job finishes, editing and Quit wait for completion. A failed
creation leaves the form available for correction.

The completed image is attached only to the settings draft. Save and the
usual confirmed F5 reset apply it to the machine. Discard keeps the created
file on disk. The guest installer must label and format this new disk.

Compressed CHD disks keep guest writes in a persistent `.diff.chd` companion
appended to the full image filename. Keep that file with the base. No automatic
merge or deletion happens on shutdown. Uncompressed CHDs without a companion
are written in place. Advanced → Disk access → Read-only also reads existing
child data but never creates a child. Merely saving settings creates none.
Malformed/mismatched companions produce errors rather than silently reverting
to the base. Do not open the same writable image in multiple machines.

CHD codecs are built from the pinned `libchdman-rs` dependency in Helios; no
external `chdman` executable is needed at runtime. See Helios
[storage documentation](https://github.com/salvogendut/helios/blob/main/docs/scsi.md) for the format and durability
limits. The core CLI can also create raw or CHD disks with `create-disk`.
