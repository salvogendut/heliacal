# Local build environment

Confirmed 2026-11-07: the host has SDL3 3.4.18 runtime libraries but neither
Rust on PATH nor SDL3 development linker symlinks. The sibling Xiri work area
contains the toolchain already used by Prismatic:

```sh
export PATH="$PWD/../xiri/.work/rustup/toolchains/nightly-2026-10-28-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_HOME=/tmp/heliacal-cargo
mkdir -p "$CARGO_HOME" target/native
# A private reflink/copy avoids modifying the sibling's Cargo cache.
cp -a --reflink=auto ../xiri/.work/cargo/registry "$CARGO_HOME/"
ln -sf /lib64/libSDL3.so.0 target/native/libSDL3.so
export LIBRARY_PATH="$PWD/target/native${LIBRARY_PATH:+:$LIBRARY_PATH}"
make check
make smoke
cargo build --release --locked --offline
```

Reuse the cache when already present. No container or system package install
is needed in this environment. The ordinary documented build expects Cargo
and SDL3 development libraries, without depending on the Xiri work area.

The original ROMs were available at `../helios/local/firmware/sun-ss5.bin`
and `../helios/local/firmware/turbogx.bin`. Use the README's boot smoke command;
it keeps preferences/NVRAM temporary and never writes to those source images.
The initial validation rendered the SS-5 banner and 128 MiB memory test.
The sibling Helios checkout was being developed concurrently; do not reset,
format, or commit its work as part of frontend changes.
