//! The cli used to interact with the woz service.

use std::io;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::fs::File;
use std::fs;
use std::str;
use std::default::Default;
use std::env;
use std::error::Error as StdError;
use std::process;
use toml;

#[macro_use] extern crate clap;
use clap::App;

#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;

use failure::Error;
use failure::ResultExt;

use rusoto_core::{Region, ByteStream};
use rusoto_core::request::HttpClient;
use rusoto_credential::StaticProvider;
use rusoto_cognito_identity::*;
use rusoto_cognito_idp::*;
use rusoto_s3::*;

mod prompt;
mod account;
mod config;
mod template;
mod package;
mod cache;

use config::*;
use template::load_templates;
use package::wasm_package;
use cache::FileCache;


enum Command {
    Init,
    NewProject,
    Setup,
    Deploy,
    Update,
    Unknown,
}

impl From<&str> for Command {
    fn from(s: &str) -> Command {
        match s {
            "init" => Command::Init,
            "new" => Command::NewProject,
            "setup" => Command::Setup,
            "deploy" => Command::Deploy,
            "update" => Command::Update,
            _ => Command::Unknown
        }
    }
}

/// Attempt to get a refresh_token token, prompting the user to log in if
/// refresh token is expired and stores it locally.
fn ensure_refresh_token(cache: &FileCache, client: &CognitoIdentityProviderClient) -> String {
    cache.get_encrypted("refresh_token")
        .or_else::<io::Error, _>(|_| {
            let creds = prompt::login();
            let token = account::login(&client, creds.username, creds.password).sync()
                .and_then(|resp| {
                    let token = resp
                        .authentication_result.expect("Failed")
                        .refresh_token.expect("Missing refresh token");
                    cache.set_encrypted("refresh_token", token.as_bytes().to_vec())
                        .expect("Failed to cache refresh token");
                    Ok(token)})
                .or_else::<io::Error, _>(|_| {
                    Ok(ensure_refresh_token(cache, client))
                })
                .expect("Something went wrong");
            Ok(token)
        })
        .unwrap()
}

fn ensure_identity_id(cache: &FileCache, client: &CognitoIdentityClient, id_token: &str)
                      -> String {
    cache.get("identity")
        .or_else::<io::Error, _>(|_| {
            let id = account::identity_id(client, id_token)
                .sync()
                .expect("Failed to get identity ID")
                .identity_id.expect("No identity ID");
            cache.set("identity", id.as_bytes().to_vec())
                .expect("Failed to add identity to cache");
            Ok(id)
        })
        .unwrap()
}

fn run() -> Result<(), Error> {
    let handlebars = load_templates().context("Failed to load templates")?;

    let yaml = load_yaml!("cli.yaml");
    let app = App::from_yaml(yaml);
    let input = app.get_matches();

    // Get the project path either from being passed in as an arg or
    // default to the current directory
    let project_path = input.args.get("project")
        .map_or(env::current_dir(),
                |arg| Ok(PathBuf::from(&arg.vals[0])))
        .context("Failed to get project path")?;
    println!("Using project path {}", project_path.to_str().unwrap());

    let home_path = input.args.get("home")
        .map_or(default_home_path(),
                |arg| Ok(PathBuf::from(&arg.vals[0])))
        .context("Failed to get woz home path")?;
    let encryption_key = FileCache::make_key(ENCRYPTION_PASSWORD, ENCRYPTION_SALT);
    let cache = FileCache::new(encryption_key, home_path.clone());
    println!("Using home path {}", home_path.to_str().unwrap());

    if let Some(sub) = input.subcommand_name() {
        match Command::from(sub) {
            // Setup should result in
            // 1. An account
            // 2. A unique user ID
            // 3. A configured dotfile
            Command::Setup => {
                fs::create_dir_all(&home_path).context("Failed to make home directory")?;
                // TODO first check if there is an existing installation
                let values = prompt::signup();
                let client = CognitoIdentityProviderClient::new(Region::UsWest2);
                account::signup(client, values.email, values.username, values.password)
                    .sync()
                    .and_then(|resp| {
                        let user_id = resp.user_sub;
                        cache.set("user", user_id.as_bytes().to_vec())
                            .expect("Failed add user ID to cache");
                        Ok(())
                    })
                    .or_else(|e| {
                        println!("Signup failed {}", e.description());
                        Err(e)
                    })?;
                println!("Please check your inbox for an email to verify your account.");
            },
            Command::NewProject => {
                // TODO Create a local config file
                // TODO Create a project landing page in S3
                // TODO Create the .woz home directory
                // Print the url

                // This unwrap is safe because the cli preparses and
                // will show an error if we are missing an argument to
                // the `new` command
                let subcommand_args = input.subcommand_matches("new").unwrap();
                let project_name = subcommand_args.value_of("NAME").unwrap();
                let command = format!("cargo new {} --lib", project_name);

                // Create the skeleton project
                process::Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .output()
                    .context("Failed to create new project using cargo")?;

                // Add Cargo.toml deps and options
                let mut cargo_conf = fs::OpenOptions::new()
                    .append(true)
                    .open(PathBuf::from(format!("{}/Cargo.toml", project_name)))
                    .context("Failed to open Cargo.toml")?;

                cargo_conf.write_all("seed = \"0.2.9\"
wasm-bindgen = \"0.2.37\"
web-sys = \"0.3.14\"

[lib]
crate-type = [\"cdylib\"]".as_bytes()).unwrap();

                // Add a woz config
                // TODO maybe safer to generate the
                // whole file and not just part of it
                let mut woz_conf = File::create(
                    PathBuf::from(format!("{}/woz.toml", project_name))
                ).context("Failed to create woz config")?;
                woz_conf.write_all(format!("name=\"Example: My App\"
project_id=\"{}\"
short_name=\"MyApp\"
lib=\"wasm-bindgen\"
wasm_path=\"target/wasm32-unknown-unknown/release/{}.wasm\"
", project_name, project_name).as_bytes()).unwrap();

                // TODO Write a hello world lib.rs
                // Would be nice if we
                // could make a static out of a file i.e from the
                // example app
            },
            // Init should result in
            // 1. A config file in the current directory
            Command::Init => {
                println!("Initializing current project directory...");
                let cargo_conf = fs::read_to_string("./Config.toml")
                    .context("You must be in a cargo project")?
                    .parse::<toml::Value>()
                    .context("Failed to read Cargo.toml")?;
                let project_name = &cargo_conf["package"]["name"];

                let mut woz_conf = File::create(
                    PathBuf::from(format!("{}/woz.toml", project_name))
                ).context("Failed to create woz config")?;
                woz_conf.write_all(format!("name=\"{}\"
project_id=\"{}\"
short_name=\"{}\"
lib=\"wasm-bindgen\"
wasm_path=\"target/wasm32-unknown-unknown/release/{}.wasm\"
", project_name, project_name, project_name, project_name).as_bytes()).unwrap();

                println!("Ready to be deployed with 'woz deploy'");
            },
            Command::Deploy => {
                println!("Deploying...");
                // Load the config if present or use default config
                let mut conf_path = project_path.clone();
                conf_path.push("woz.toml");
                let conf_str = fs::read_to_string(conf_path.clone())
                    .context(format!("Couldn't find woz config file at {}",
                                     conf_path.clone().to_str().unwrap()))?;
                let conf: Config = toml::from_str(&conf_str).context("Failed to parse woz config")?;

                let ProjectId(project_id) = conf.project_id;

                let id_provider_client = CognitoIdentityProviderClient::new(Region::UsWest2);
                let id_client = CognitoIdentityClient::new(Region::UsWest2);

                let refresh_token = ensure_refresh_token(&cache, &id_provider_client);
                let id_token = account::refresh_auth(&id_provider_client, &refresh_token)
                    .sync()
                    .or_else(|err| {
                        // TODO only login if the failure is due to an auth error
                        println!("Getting refresh token failed: {}", err);
                        let creds = prompt::login();
                        account::login(&id_provider_client, creds.username, creds.password)
                            .sync()
                            .or_else(|e| {
                                println!("Login failed: {}", e);
                                Err(e)
                            })
                            .and_then(|resp| {
                                let token = resp.clone()
                                    .authentication_result.expect("Failed")
                                    .refresh_token.expect("Missing refresh token");
                                cache.set_encrypted(
                                    "refresh_token",
                                    token.as_bytes().to_vec()
                                ).expect("Failed to cache refresh token");
                                Ok(resp)})
                    })
                    .context("Failed to get id token")?
                    .authentication_result.expect("No auth result")
                    .id_token.expect("No access token");
                let identity_id = ensure_identity_id(&cache, &id_client, &id_token);
                let aws_creds = account::aws_credentials(&id_client, &identity_id, &id_token)
                    .sync()
                    .context("Failed to fetch AWS credentials")?
                    .credentials.expect("Missing credentials");
                let creds_provider = StaticProvider::new(
                    aws_creds.access_key_id.expect("Missing access key"),
                    aws_creds.secret_key.expect("Missing secret key"),
                    Some(aws_creds.session_token.expect("Missing session token")),
                    Some(aws_creds.expiration.expect("Missing expiration") as i64)
                );

                let request_dispatcher = HttpClient::new();
                let s3_client = S3Client::new_with(
                    request_dispatcher.context("Failed to make an HttpClient")?,
                    creds_provider,
                    Region::UsWest2
                );

                let mut out_path = home_path.clone();
                out_path.push(&project_id);
                out_path.push("pkg");
                fs::create_dir_all(&out_path).context("Failed to make pkg directory")?;

                let mut wasm_path = project_path.clone();
                wasm_path.push(conf.wasm_path);

                let index_template = handlebars.render("index", &json!({
                    "name": conf.name,
                    "author": conf.author,
                    "description": conf.description,
                    "manifest_path": "./manifest.json",
                    "app_js_path": "./app.js",
                    "sw_js_path": "./sw.js",
                    "wasm_path": "./app.wasm",
                }));
                let manifest_template = handlebars.render("manifest", &json!({
                    "name": conf.name,
                    "short_name": conf.short_name,
                    "bg_color": "#ffffff",
                    "description": conf.description,
                    "icons": {
                        "path_48": "./img/icons/homescreen_48x48.png",
                        "path_72": "./img/icons/homescreen_72x72.png",
                        "path_96": "./img/icons/homescreen_96x96.png",
                        "path_144": "./img/icons/homescreen_144x144.png",
                        "path_168": "./img/icons/homescreen_168x168.png",
                        "path_192": "./img/icons/homescreen_192x192.png"
                    }
                }));
                let service_worker_template = handlebars.render("sw.js", &json!({}));

                let wasm_package = wasm_package(
                    conf.lib.unwrap(),
                    wasm_path,
                    out_path,
                ).context("Failed to generate wasm package")?;

                // All app files will be prefixed in the s3 bucket by the user's
                // cognito identity ID and project_id
                let key_prefix = format!("{}/{}", &identity_id, &project_id);

                let files = vec![
                    (format!("{}/index.html", key_prefix),
                     String::from("text/html"),
                     index_template.context("Failed to render index.html")?.into_bytes()),
                    (format!("{}/manifest.json", key_prefix),
                     String::from("application/manifest+json"),
                     manifest_template.context("Failed to render manifest.json")?.into_bytes()),
                    (format!("{}/sw.js", key_prefix),
                     String::from("application/javascript"),
                     service_worker_template.context("Failed to render sw.js")?.into_bytes()),
                    (format!("{}/app.js", key_prefix),
                     String::from("application/javascript"),
                     fs::read_to_string(wasm_package.js).context("Failed to read js file")?.into_bytes()),
                    (format!("{}/app.wasm", key_prefix),
                     String::from("application/wasm"),
                     {
                         let mut f = File::open(wasm_package.wasm).context("Failed to read wasm file")?;
                         let mut buffer = Vec::new();
                         f.read_to_end(&mut buffer).context("Failed to read to bytes")?;
                         buffer
                     }),
                ];

                for (file_name, mimetype, body) in files.into_iter() {
                    let req = PutObjectRequest {
                        bucket: String::from(S3_BUCKET_NAME),
                        key: file_name.clone(),
                        body: Some(ByteStream::from(body)),
                        content_type: Some(mimetype),
                        ..Default::default()
                    };

                    s3_client.put_object(req)
                        .sync()
                        .context(format!("Failed to upload file to S3: {}", file_name))?;
                };

                let location = format!(
                    "{}://{}/{}/{}/index.html",
                    SCHEME,
                    NETLOC,
                    identity_id,
                    project_id
                );
                println!("{}", format!("Your app is available at {}", location));
            }
            // Sub command parsing will print the error and exit
            // before we get to this match statement so the only way
            // we can reach here is if there is a valid subcommand
            // specified, but it hasn't been implemented
            _ => unimplemented!()
        };
    };
    Ok(())
}

fn main() {
    run().map_err(|e| println!("{}", e)).ok();
}
