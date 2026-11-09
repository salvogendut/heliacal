# Machine selection: SS-5, SS-20 and Axil 320

Settings gain `model` (`sparcstation5`, `sparcstation20`, `axil320`), selectable
with `--machine` or the General settings row. The model maps directly to
`helios::machine::MachineProfile`.

- SS-5, SS-20 and Axil 320 require a TurboGX FCode ROM in the frontend. SS-5 installs it in SBus slot 1; Axil
  320 installs it in slot 2 through Helios's layout.
- SS-20 normally has the built-in SX/CG14, but its firmware console currently
  renders a white screen. The frontend therefore removes SX and requires a
  TurboGX card for SS-20 too. Display publishing still accepts either card
  generation key for future SX console work.
- NVRAM is separated per machine: the existing `<config>.nvram` remains SS-5,
  while SS-20 and Axil use `<config>.nvram.ss20` and
  `<config>.nvram.axil320`. New files start from that profile's IDPROM identity.
- Model changes count as hardware changes and require the normal confirmed cold
  reset. Firmware/media paths remain explicit; switching model does not guess a
  matching PROM path.
