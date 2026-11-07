# Heliacal development

- Implement the desktop frontend in Rust with Cargo and SDL3. Match Prismatic
  and the Happy Years emulator-first interface: F9 overlay, keyboard navigation,
  status hints, notifications, and Tinker-gated diagnostics.
- The emulator is the sibling `../helios` library. Inspect its current APIs
  before integrating. Keep CPU/device changes there; read its AGENTS.md and
  relevant architecture documentation before changing the core.
- Initially support only Sun SPARCstation 5 / microSPARC-II at 85 MHz with
  128 MiB RAM and TurboGX. Validate this in configuration and construction;
  do not expose unimplemented machine or peripheral controls.
- Keep SDL window/render/event work on the main thread and machine work on
  the worker. Bound input and display handoffs. Guest time must not depend
  on UI presentation or refresh rate.
- Settings drafts must be independent. Save validates and persists before
  closing; Discard has no runtime effects. Explain when a saved change takes
  effect and confirm operations that discard guest RAM.
- Preserve user firmware and media. Current frontend attachments are
  read/write by default, with an Advanced read-only option; CDs remain read-only.
  Keep firmware and disk images under ignored local/; use temporary
  configurations and NVRAM in smoke tests.
- Keep README status accurate. A rendered firmware banner does not establish
  an OS boot or installation. Do not claim placeholder controls implement
  emulation.
- Run `cargo fmt -p heliacal --check`,
  `cargo clippy --locked --all-targets --no-deps -- -D warnings`,
  `cargo test --locked`, and `make smoke` for frontend changes. For lifecycle
  or display changes, run the documented real-firmware smoke test when local
  ROMs are available. Do not use `cargo fmt --all` across the sibling core.
- Human documentation belongs in README.md and docs/. Environment notes and
  agent findings belong in clanker/ with an index.
