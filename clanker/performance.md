# Frontend performance

Branch `performance` batches worker I/O and repaint work:

- The 2048-step inner loop locks the host input mutex once per batch, drains
  key and mouse queues into worker-local queues, and applies at most one key
  event per guest step so keyboard FIFO backpressure still retains events.
  Release commands clear pending events and synthesize releases from the
  accepted-key state.
- NAT polling and the host clock read run once per batch instead of once per
  instruction. Guest network DMA and interrupt sampling still happen at the
  existing instruction boundaries.
- Repaint conversion is skipped unless the CGsix scanout key (generation plus
  blink phase) changes. The worker reuses its indexed-pixel buffer and uses
  `Arc::make_mut` for the RGBA snapshot, and increments `Snapshot::frame` only
  when pixels change.
- `Snapshot` now carries the `network_activity` counter used by the status
  lamps; the previous main branch used the field without declaring it.

Verified with `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and all
18 Heliacal tests in the distrobox container (the host lacks the SDL3 development
symlink). SDL desktop behavior still needs a live-window run.
