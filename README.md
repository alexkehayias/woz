[![Build Status](https://travis-ci.org/alexkehayias/woz.svg?branch=master)](https://travis-ci.org/alexkehayias/woz)

# What is Woz?

[Woz](https://woz.sh) is a progressive WebAssembly app generator (PWAA) for Rust.

## Docs

See https://woz.sh for the latest docs.

### Quick Start

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


Setup a free account and deploy the sample app:

```sh
woz signup
woz new myapp && cd ./myapp
woz deploy
```

Woz can also be self-hosted. See https://woz.sh for the latest docs.

## License

Eclipse Public License 1.0 (EPL-1.0)
