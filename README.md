# bluetooth-tui

A terminal UI for exploring and managing Bluetooth devices via BlueZ —
`bluetoothctl` with a screen. Browse adapters and devices, pair,
connect, trust/block, watch live events, and attribute a device's
vendor across three tiers of confidence (GATT PnP ID, IEEE OUI,
advertised manufacturer ID) rather than trusting any single guess.

There's also a browser build: the same UI, generic over the driver
backend, running against the [Web Bluetooth
API](https://developer.chrome.com/docs/capabilities/bluetooth) instead
of BlueZ.

## Workspace layout

| Crate | What it is |
|---|---|
| [`bluetooth-driver`](crates/bluetooth-driver) | Backend-agnostic Bluetooth traits (`Adapter`, `Device`, `BluetoothDriver`) mirroring `bluetoothctl`'s capabilities, plus a BlueZ D-Bus implementation. Linux only for the BlueZ half; the traits themselves aren't. |
| [`bluetooth-tui-core`](crates/bluetooth-tui-core) | The shared app state and ratatui rendering, generic over any `BluetoothDriver`. Both frontends below are thin wrappers around this. |
| [`bluetooth-tui`](crates/bluetooth-tui) | The native terminal binary: `bluetooth-tui-core` + `bluetooth-driver`'s BlueZ backend + crossterm. |
| [`bluetooth-driver-web`](crates/bluetooth-driver-web) | A `BluetoothDriver` implementation on top of `navigator.bluetooth`, for `wasm32-unknown-unknown`. |
| [`bluetooth-tui-web`](crates/bluetooth-tui-web) | The browser binary: `bluetooth-tui-core` + `bluetooth-driver-web` + [ratzilla](https://github.com/ratatui/ratzilla) (ratatui-in-the-browser). Source-distributed only — see [why](#the-web-frontend) below. |

## Installing the native TUI

Requires Linux with BlueZ (`bluetoothd`) running — the same daemon
`bluetoothctl` talks to, over the system D-Bus.

```sh
cargo install bluetooth-tui
```

Or from a checkout of this repo:

```sh
cargo install --path crates/bluetooth-tui
```

Then just run it:

```sh
bluetooth-tui
```

## Building from source

```sh
git clone https://github.com/jfernand/bluetooth-tui
cd bluetooth-tui
cargo check --workspace   # bluetooth-driver, bluetooth-tui-core, bluetooth-tui
cargo test --workspace
cargo run -p bluetooth-tui
```

`crates/bluetooth-driver-web` and `crates/bluetooth-tui-web` are
deliberately **not** members of this workspace (see the comment in the
root `Cargo.toml`) — they only build for `wasm32-unknown-unknown`, and
including them would break a plain `cargo check --workspace` for
everyone. Each is its own standalone single-crate workspace; build
them from within their own directory. They provide experimental support 
for Web Bluetooth API

## Running the web frontend

Needs the `wasm32-unknown-unknown` target and
[`trunk`](https://trunkrs.dev/):

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk

cd crates/bluetooth-tui-web
trunk serve --open
```

Open it in **Chrome or Edge** — Web Bluetooth doesn't exist in Firefox
or Safari. Press `s` to open the browser's native device chooser and
grant this page access to a device (Web Bluetooth requires that
explicit, per-device permission; there's no passive scanning).

The repo's `.cargo/config.toml`, scoped to the `wasm32-unknown-unknown`
target, sets `--cfg=web_sys_unstable_apis`, which `web-sys`'s Web
Bluetooth bindings require — Web Bluetooth is a Working Draft, not a
stable web standard, and `web-sys` gates the whole surface behind that
flag accordingly. `trunk build`/`trunk serve` and any `cargo check`/
`build` run from within `crates/bluetooth-tui-web` (or `bluetooth-driver-web`)
all pick this up automatically, since Cargo searches upward through
parent directories for `.cargo/config.toml`.

### The web frontend

`bluetooth-tui-web` isn't published to crates.io and isn't a
`cargo install` target: it's a `wasm32-unknown-unknown` binary meant
to be built with `trunk` and served as static web assets, not
installed as a native executable. Its `Cargo.toml` also carries a
`[patch.crates-io]` section fixing two small upstream bugs in vendored
dependencies (see `crates/bluetooth-tui-web/vendor/`) — `[patch]`
sections don't survive `cargo publish`, so a published copy would
silently lose those fixes. It stays source-distributed via this repo.

## Usage

The native and web frontends share the same keymap (press `?` in
either for the full reference); the web one is necessarily missing a
few actions Web Bluetooth has no equivalent for (adapter power/
discoverable/pairable control, background scanning, trust/block).

| Key | Action |
|---|---|
| `↑` `↓` | Move selection |
| `→` `←` | Drill into / back out of a column |
| `↵` | Connect / disconnect the selected device |
| `p` | Pair |
| `s` | Start/stop scan (native) · open the device chooser (web) |
| `v` | Vendor attribution chain |
| `a` | Edit alias |
| `t` / `T` | Trust / untrust |
| `b` / `B` | Block / unblock |
| `F` | Forget (removes bonding keys) |
| `e` | Event log |
| `A` | Adapter control |
| `f` | Toggle fullscreen detail |
| `:` | Command palette |
| `?` | Keymap help |
| `q` | Quit |

## License

Licensed under either of [Apache License, Version
2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project by you shall be dual licensed
as above, without any additional terms or conditions.
