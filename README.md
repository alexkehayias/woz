[![Build Status](https://travis-ci.org/alexkehayias/woz.svg?branch=master)](https://travis-ci.org/alexkehayias/woz)

# What is Woz?

[Woz](https://woz.sh) is a progressive WebAssembly app generator (PWAA) for Rust.

## Docs

See https://woz.sh for the latest docs.

## Quickstart

Before we begin you must have a recent version of [Rust](https://www.rust-lang.org) installed as well as [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen).

### Install `wasm-bindgen`

Woz uses `wasm-bindgen` to generate the interop calls between WebAssembly and JavaScript. This allows you to write the entire application in Rust—including rendering to the dom.

```
cargo install -f wasm-bindgen-cli
```

### Install wasm compiler target

```
rustup target add wasm32-unknown-unknown
```


### Install Woz

Install a pre-built binary.

For macOS (64 bit only):

```sh
curl -LSfs https://woz.sh/bin/install.sh | sh -s -- --target x86_64-apple-darwin
```

For linux (via musl):

```sh
curl -LSfs https://woz.sh/bin/install.sh | sh -s -- --target x86_64-unknown-linux-musl
```

For bsd:

```sh
curl -LSfs https://woz.sh/bin/install.sh | sh -s -- --target x86_64-unknown-freebsd
```

```sh
curl -LSfs https://woz.sh/bin/install.sh | sh -s -- --target x86_64-unknown-netbsd
```


### Setup and deploy

```sh
# Follow prompts to create your free Woz account
woz signup
# Create a new app
woz new myapp && cd myapp
# Deploy it
woz deploy
```

## Examples

The 'Seed' example app uses the `seed` framework and clocks in at ~600kb (including ~300kb for an icon and splashscreen), works offline, and can be installed to your homescreen on iOS or Android devices. You can try it out [here](https://woz.sh/us-west-2:f72ab923-2251-4e0d-925e-f3a4408ec70e/seed/index.html)

## Self-hosting

You can self-host by using `woz` to build your app locally and upload the files to your static file hosting service such as AWS S3.

Build the app locally:

```
cd myapp/
woz build
```

Follow the cli output to get the location of the generated app files on disk. It will look something like:

```
App package directory can be found at /Users/myusername/.woz/myapp/pkg
```

The `app` directory contains an `index.html` file that will be the entry point for running the app in a browser.

Note: the security requirements for PWAs and WebAssembly means you will need to serve the files over https. Browsing the files directly in the browser (e.g. `file://`) will result in security-related errors. Use a static file server and install an SSL certificate to be able to install the app to your home screen.

You can also build Woz so that it can use your AWS account and allow multiple users to securely deploy apps to a shared S3 bucket. See https://woz.sh for the latest docs.

## License

Eclipse Public License 1.0 (EPL-1.0)
