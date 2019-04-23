# What is Woz?

[Woz](https://woz.sh) is a WebAssembly progressive web app (PWA) toolchain for building and deploying performant mobile apps with Rust. Distributed your app is as simple as sharing a hyperlink.

## Docs

See https://woz.sh for the latest docs.

### Quick Start

You can create a free account, generate the sample app, and deploy by running the following in your terminal:

```sh
cargo install woz
woz setup
woz new myapp && cd ./myapp
woz deploy
```

## Early Access—Free

You can join for free and deploy an unlimited number of WebAssembly progressive web apps to your workspace. We currently support Rust generated WebAssembly binaries via the `wasm32-unknown-unkown` target that are `wasm-bindgen` compatible.

Coming soon—manage charging for your apps and even provide multiple copies your users can share all with a hyperlink.

## License

Eclipse Public License 1.0 (EPL-1.0)
