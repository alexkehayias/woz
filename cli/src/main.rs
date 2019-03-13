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

use config::*;
use template::load_templates;
use package::wasm_package;


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
fn ensure_refresh_token(home_path: &PathBuf, client: &CognitoIdentityProviderClient) -> String {
    let mut path = home_path.clone();
    path.push(".refresh_token");

    fs::read_to_string(path)
        .or_else::<io::Error, _>(|_| {
            let creds = prompt::login();
            let token = account::login(&client, creds.username, creds.password).sync()
                .and_then(|resp| {
                    let token = resp
                        .authentication_result.expect("Failed")
                        .refresh_token.expect("Missing refresh token");
                    store_token(&home_path, &token);
                    Ok(token)})
                .or_else::<io::Error, _>(|_| {
                    Ok(ensure_refresh_token(home_path, client))
                })
                .expect("Something went wrong");
            Ok(token)
        })
        .unwrap()
}

fn ensure_identity_id(home_path: &PathBuf, client: &CognitoIdentityClient, id_token: &str)
                      -> String {
    let mut path = home_path.clone();
    path.push(".identity");

    fs::read_to_string(path)
        .or_else::<io::Error, _>(|_| {
            let id = account::identity_id(client, id_token)
                .sync()
                .expect("Failed to get identity ID")
                .identity_id.expect("No identity ID");
            store_identity_id(&home_path, &id);
            Ok(id)
        })
        .unwrap()
}

fn store_token(home_path: &PathBuf, refresh_token: &str) {
    let mut file_path = home_path.clone();
    file_path.push(".refresh_token");
    fs::create_dir_all(home_path).unwrap();
    let mut f = File::create(&file_path).unwrap();
    f.write_all(refresh_token.as_bytes()).unwrap();
}

fn store_user_id(home_path: &PathBuf, user_id: &str) {
    let mut file_path = home_path.clone();
    file_path.push(".user");
    fs::create_dir_all(home_path).unwrap();
    let mut f = File::create(&file_path).unwrap();
    f.write_all(user_id.as_bytes()).unwrap();
}

fn store_identity_id(home_path: &PathBuf, id: &str) {
    let mut file_path = home_path.clone();
    file_path.push(".identity");
    fs::create_dir_all(home_path).unwrap();
    let mut f = File::create(&file_path).unwrap();
    f.write_all(id.as_bytes()).unwrap();
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
    println!("Using home path {}", home_path.to_str().unwrap());

    // Load the config if present or use default config
    let mut conf_path = project_path.clone();
    conf_path.push("woz.toml");
    let conf_str = fs::read_to_string(conf_path.clone())
        .context(format!("Couldn't find woz config file at {}",
                         conf_path.clone().to_str().unwrap()))?;
    let conf: Config = toml::from_str(&conf_str)
        .context("Failed to parse woz config")?;

    let ProjectId(project_id) = conf.project_id;

    if let Some(sub) = input.subcommand_name() {
        match Command::from(sub) {
            // Setup should result in
            // 1. An account
            // 2. A unique user ID
            // 3. A configured dotfile
            Command::Setup => {
                // TODO first check if there is an existing installation
                let values = prompt::signup();
                let client = CognitoIdentityProviderClient::new(Region::UsWest2);
                account::signup(client, values.email, values.username, values.password)
                    .sync()
                    .and_then(|resp| {
                        let user_id = resp.user_sub;
                        store_user_id(&home_path, &user_id);
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
            },
            // Init should result in
            // 1. A config file in the current directory
            Command::Init => {
                println!("Init...");
                // TODO Create a local config file
                // TODO Create a project landing page in S3
                // TODO Create the .woz home directory
                // Print the url
            },
            Command::Deploy => {
                println!("Deploying...");
                let id_provider_client = CognitoIdentityProviderClient::new(Region::UsWest2);
                let id_client = CognitoIdentityClient::new(Region::UsWest2);

                let refresh_token = ensure_refresh_token(&home_path, &id_provider_client);
                let id_token = account::refresh_auth(&id_provider_client, &refresh_token)
                    .sync()
                    .or_else(|err| {
                        // TODO only login if the failure is due to auth error
                        println!("Getting refresh token failed {}", err);
                        let creds = prompt::login();
                        account::login(&id_provider_client, creds.username, creds.password)
                            .sync()
                            .or_else(|e| {
                                println!("Login failed {}", e);
                                Err(e)
                            })
                            .and_then(|resp| {
                                let token = resp.clone()
                                    .authentication_result.expect("Failed")
                                    .refresh_token.expect("Missing refresh token");
                                store_token(&home_path, &token);
                                Ok(resp)})
                    })
                    .context("Failed to get id token")?
                    .authentication_result.expect("No auth result")
                    .id_token.expect("No access token");
                let identity_id = ensure_identity_id(&home_path, &id_client, &id_token);
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
                    "name": "Test App",
                    "author": "Alex Kehayias",
                    "description": "Description here",
                    "manifest_path": "./manifest.json",
                    "app_js_path": "./app.js",
                    "sw_js_path": "./sw.js",
                    "wasm_path": "./app.wasm",
                }));
                let manifest_template = handlebars.render("manifest", &json!({
                    "name": conf.name,
                    "short_name": "",
                    "bg_color": "#ffffff",
                    "description": "Description here",
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

// TODO replace Box<Error> with an enum of all the possible errors
fn main() {
    run().map_err(|e| println!("{}", e)).ok();
}
