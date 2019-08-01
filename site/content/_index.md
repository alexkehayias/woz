+++
+++

# What is Woz?

Woz is a WebAssembly progressive web app (PWA) toolchain for building and deploying performant mobile apps with Rust. Distributed your app is as simple as sharing a hyperlink.

## Quickstart

Before we begin you must have a recent version of [Rust](https://www.rust-lang.org) installed as well as [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen).

### Install `wasm-bindgen`

Woz uses `wasm-bindgen` to generate the interop calls between WebAssembly and JavaScript. This allows you to write the entire application in Rust—including rendering to the dom.

```
cargo install -f wasm-bindgen-cli
```

### Install Woz

Using `cargo`:

```sh
cargo install woz
```

### Setup and deploy

```sh
# Follow prompts to create your Woz account
woz setup
# Create a new app
woz new myapp && cd myapp
# Deploy it for free
woz deploy
```

## Early Access—Free

You can join for free and deploy an unlimited number of WebAssembly progressive web apps to your workspace. We currently support Rust generated WebAssembly binaries via the `wasm32-unknown-unkown` target that are `wasm-bindgen` compatible.

Coming soon—manage charging for your apps and even provide multiple copies your users can share all with a hyperlink.


## Examples

The 'Seed' example app uses the `seed` framework and clocks in at ~600kb (including ~300kb for an icon and splashscreen), works offline, and can be installed to your homescreen on iOS or Android devices. You can try it out [here](https://woz.sh/us-west-2:f72ab923-2251-4e0d-925e-f3a4408ec70e/seed/index.html)

## Open Source

Woz is open source and available on [GitHub](https://github.com/alexkehayias/woz). Please share your suggestions and bug reports [here](https://github.com/alexkehayias/woz/issues).
