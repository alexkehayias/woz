# woz.sh

WebAssembly + Progressive Web Apps for static mobile applications.

## Installation

```
curl https://woz.sh/installer/init.sh -sSf | sh
```

## Quickstart

### Using Rust

#### Environment
You'll need a recent version of Rust `rustup update` and install the WebAssembly toolchain `rustup target add wasm32-unknown-unknown`. You will also need to install `wasm-bindgen` via `cargo install wasm-bindgen-cli`.

#### Usage

```
# Create an account to host your WebAssembly app
woz setup
# Compile to wasm
cargo build --target wasm32-unknown-unknown --release

# Deploy your first app
woz deploy
>> Your can install your application by visiting https://woz.sh/<username>/<appname>
```
