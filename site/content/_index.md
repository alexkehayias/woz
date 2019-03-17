+++
+++

# What is Woz?

Woz is a WebAssembly progressive web app (PWA) toolchain for deploying performant mobile apps distributed for free with a hyperlink.

## Quickstart

Before we begin you must have a recent version of [Rust](https://www.rust-lang.org) installed as well as [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen).

Install Woz:

```sh
wget https://woz.sh/install/macos/woz &&
mv ./woz /usr/local/bin/ &&
chmod +x /usr/local/bin/woz
```

Setup and deploy your project:

```sh
woz setup
# Follow prompts to create your Woz account
woz new myapp
woz deploy
>> Your app is now available at https://woz.sh/myusername/myapp
```

## Early Access—Free

You can join for free and deploy an unlimited number of WebAssembly progressive web apps to your workspace. We currently support Rust generated WebAssembly binaries via the `wasm32-unknown-unkown` target that are `wasm-bindgen` compatible.

Coming soon—manage charging for your apps and even provide multiple copies your users can share all with a hyperlink.

## Examples

* [`seed` framework example](https://woz.sh/us-west-2:f72ab923-2251-4e0d-925e-f3a4408ec70e/seed/index.html)

## Open Source

Woz is open source and available on [GitHub](https://github.com/alexkehayias/woz). Please share your suggestions and bug reports [here](https://github.com/alexkehayias/woz/issues).
