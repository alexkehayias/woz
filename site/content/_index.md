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

## Hosting

### Woz.sh Early Access—Free

Due to the security requirements of PWAs and WebAssembly, the easiest way to deploy is to create a free woz.sh account. To create an account use `woz signup` and follow the prompts. If you already have an account use `woz setup` to set up your local computer with credentials. Then use `woz deploy` to deploy your app.

### Self-hosting

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

### Multi-user AWS hosting

You can compile `woz` so that it uses your AWS account and allow for multiple users to securely deploy apps to a shared S3 bucket.

To do that, clone the Woz repo locally and put the following environment variables in a file:

```
# Scheme to use when constructing URLs to your app
WOZ_WEB_SCHEME="https"

# Domain to use links to your app
WOZ_WEB_NETLOC="example.com"

# Cognito identity pool to use for user registration
WOZ_USER_POOL_URL="cognito-idp.<REGION>.amazonaws.com/<USER POOL ID>"

# Cognito identity pool to use for authentication
WOZ_IDENTITY_POOL_ID="<REGION>:<IDENTITY POOL ID>"

# Cognito user pool app client to use for use with the CLI
WOZ_CLIENT_ID="<USER POOL APP CLIENT ID>"

# S3 bucket where static files will be stored
WOZ_S3_BUCKET_NAME="<S3 BUCKET NAME>"

# Password used for encrypting tokens on disk
WOZ_ENCRYPTION_PASSWORD="<STRONG PASSWORD>"

# Salt used for encrypting tokens on disk
WOZ_ENCRYPTION_SALT="<RANDOM SALT>"

# Location of the woz repo, needed to generate new projects
WOZ_PROJECT_ROOT="<PATH TO WOZ REPO>"

# Location of the woz repo, needed to include assets
WOZ_CLI_PROJECT_ROOT="<PATH TO WOZ REPO>/cli"
```

In your terminal, add the environment variables to the session:

```
set -a; . ../my-env; set +a
```

You can now build and deploy to your own AWS account. For example:

```
cargo run setup
cargo run new myapp
cargo run deploy --project-root ./myapp
```

## Open Source

Woz is open source and available on [GitHub](https://github.com/alexkehayias/woz). Please share your suggestions and bug reports [here](https://github.com/alexkehayias/woz/issues).
