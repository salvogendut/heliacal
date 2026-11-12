# Machine selection: SS-5, SS-20 and Axil 320

Settings gain `model` (`sparcstation5`, `sparcstation20`, `axil320`), selectable
with `--machine` or the General settings row. The model maps directly to
`helios::machine::MachineProfile`.

- SS-5 and Axil 320 require a TurboGX FCode ROM; SS-5 installs it in SBus
  slot 1 and Axil 320 in slot 2. SS-20 can use native SX/CG14 without a ROM.
- SS-20 has an explicit display choice. Native SX/CG14 now opens through OBP
  with an 8 MiB VSIMM model and matching monitor sense; TurboGX remains
  available. Selecting CG6 on SS-20 removes SX so firmware uses the SBus card.
- NVRAM is separated per machine: the existing `<config>.nvram` remains SS-5,
  while SS-20 and Axil use `<config>.nvram.ss20` and
  `<config>.nvram.axil320`. New files start from that profile's IDPROM identity.
- Model changes count as hardware changes and require the normal confirmed cold
  reset. Firmware/media paths remain explicit; switching model does not guess a
  matching PROM path.

The Helios core now enables the SS-20/Axil standalone IOMMU Arbiter Enable
Register and masks IOPTE physical bits correctly. Axil `probe-scsi-all` lists
an attached ID 3 disk; the frontend's existing Media selection needs no change.
