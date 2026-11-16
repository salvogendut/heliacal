# Building and first boot

[Back to Heliacal](../README.md) · [All guides](../README.md#documentation)

Run these commands from the Heliacal repository root. Keep the checkouts
alongside each other:

```text
Dev/
  helios/
  heliacal/
```

Use a current stable Rust toolchain (edition 2024), Cargo, a C++17 compiler,
a native linker, and SDL3 development libraries. SDL3 is linked dynamically;
SDL_ttf and SDL_image are
not required. On a Fedora development system, the native dependencies are
`gcc`, `gcc-c++`, `make`, and `SDL3-devel`.

```sh
make
./target/release/heliacal
# Or build and launch together:
make run
# Direct Cargo builds also use the sibling Helios checkout:
cargo run --release --locked
```

The application opens **paused**. Choose the machine, display and firmware
under **F9 > General**. An empty PROM path selects the installed OpenBIOS
fallback; an explicit original Sun PROM takes precedence, including any errors.
TurboGX needs both system firmware and its separate 32 KiB FCode ROM;
native SS-20 SX uses original OBP without a separate framebuffer ROM.
Save with **F9** and press **Ctrl+P** to boot. The default
is 128 MiB; choose **Memory** in F9 for another exact supported total.
Original PROMs, OpenBIOS and guest operating systems are not bundled in the
current source build. See the README's [original PROM sources and MD5 sums](../README.md#firmware-and-prom-images)
and the [OpenBIOS installation instructions](firmware.md#openbios-fallback).

You can also select the machine and supply paths for this session:

```sh
./target/release/heliacal --machine ss5 \
  --prom ../helios/local/firmware/sun-ss5.bin \
  --cgsix-rom ../helios/local/firmware/turbogx.bin
./target/release/heliacal --machine ss20 --display sx --ram-mib 448 \
  --prom ../helios/local/firmware/ss10-20-v2.25r-rom
./target/release/heliacal --machine ss20 --display cgsix --ram-mib 512 \
  --prom ../helios/local/firmware/ss10-20-v2.25r-rom \
  --cgsix-rom ../helios/local/firmware/turbogx.bin
./target/release/heliacal --machine axil320 --ram-mib 512 \
  --prom ../helios/local/firmware/axil320-2-19.bin \
  --cgsix-rom ../helios/local/firmware/turbogx.bin
```

Graphical SS-5 and Axil 320 require the ROM for the **1 MiB TurboGX card**
(`turbogx.bin`); the ROM file itself is 32 KiB. ROMs for the 4 MiB TurboGX+
card, including `sun-tgx+-vesa.bin`, are incompatible with the emulated card
and known images are rejected at setup. SS-20 with `--display sx` and headless
guests do not need a TurboGX ROM.

`make` builds the current sources against `../helios`; it does not fetch or
update the core. Heliacal enables Helios's `chd` and `host-icmp` features.
Linux is the validated host; Unix PTYs are required for headless guests.
See [firmware selection](firmware.md) before the first boot and
[testing](testing.md) for the available checks.
