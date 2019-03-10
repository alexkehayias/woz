//! The cli used to interact with the woz service.

use std::io;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::fs::File;
use std::fs;
use std::str;
use std::error::Error;
use std::default::Default;
use std::env;
use std::fmt;
use std::process;
use toml;

#[macro_use]
extern crate clap;
use clap::App;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use rusoto_core::{Region, ByteStream};
use rusoto_core::request::HttpClient;
use rusoto_credential::StaticProvider;
use rusoto_cognito_identity::*;
use rusoto_cognito_idp::*;
use rusoto_s3::*;

use handlebars::Handlebars;

mod prompt;
mod account;
mod config;

use config::*;


fn default_home_path() -> Result<PathBuf, Box<Error>> {
    let home: String = std::env::var_os("XDG_CONFIG_HOME")
        .or_else(|| std::env::var_os("HOME"))
        .map(|v| v.into_string().expect("Unable to parse $HOME to string"))
        .expect("No home");
    let mut buf = PathBuf::new();
    buf.push(home);
    buf.push(".woz");
    Ok(buf)
}

#[test]
// TODO only compile on macOS
fn default_home_path_test() {
    let user = std::env::var_os("USER")
        .map(|v| v.into_string().expect("Could not parse $USER to string"))
        .expect("Could not get a $USER");
    let path_str = format!("/Users/{}/.woz", user);
    assert_eq!(PathBuf::from(path_str), default_home_path().unwrap());
}

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
    dbg!(path.clone());

    fs::read_to_string(path)
        .or_else::<io::Error, _>(|err| {
            dbg!(err);
            let creds = prompt::login();
            let token = account::login(&client, creds.username, creds.password).sync()
                .and_then(|resp| {
                    let token = resp
                        .authentication_result.expect("Failed")
                        .refresh_token.expect("Missing refresh token");
                    store_token(&home_path, &token);
                    Ok(token)})
                .or_else::<io::Error, _>(|err| {
                    dbg!(err);
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
    dbg!(file_path.clone());

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

const INDEX_TEMPLATE: &str = include_str!("templates/index.html");
const MANIFEST_TEMPLATE: &str = include_str!("templates/manifest.json");
const SERVICE_WORKER_JS_TEMPLATE: &str = include_str!("templates/serviceworker.js");

fn load_templates() -> Result<Handlebars, Box<Error>> {
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);
    handlebars.register_template_string("index", INDEX_TEMPLATE)?;
    handlebars.register_template_string("manifest", MANIFEST_TEMPLATE)?;
    handlebars.register_template_string("sw.js", SERVICE_WORKER_JS_TEMPLATE)?;
    Ok(handlebars)
}

#[test]
fn test_index_templates() {
    let loader = load_templates().expect("Failed to load templates");
    let res = loader.render(
        "index",
        &json!({
            "name": "Test App",
            "author": "Alex Kehayias",
            "description": "Description here",
            "loader_js_path": "./loader.js",
            "sw_js_path": "./sw.js",
            "wasm_path": "./app.wasm",
        }));
    dbg!(res.expect("Failed to render"));
}

#[derive(Debug)]
struct WasmPackageError;

impl fmt::Display for WasmPackageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Failed to generate wasm package")
    }
}

impl Error for WasmPackageError {
    fn description(&self) -> &str {
        "Failed to generate wasm package"
    }
}

struct WasmPackage {
    lib: Lib,
    js: PathBuf,
    wasm: PathBuf,
}

impl WasmPackage {
    fn new(lib: Lib, wasm_path: PathBuf, js_path: PathBuf) -> Self {
        WasmPackage {lib: lib, wasm: wasm_path, js: js_path}
    }
}

/// Generates a js file that manages the interop between js and wasm
fn wasm_package(lib: Lib, wasm_path: PathBuf, out_path: PathBuf)
                -> Result<WasmPackage, WasmPackageError> {
    match lib {
        Lib::WasmBindgen => {
            let command = format!(
                "wasm-bindgen {} --no-typescript --no-modules --out-dir {} --out-name app",
                wasm_path.into_os_string().into_string().unwrap(),
                out_path.clone().into_os_string().into_string().unwrap()
            );

            let output = process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .expect("failed to execute process");
            dbg!(output);

            let mut js_path = out_path.clone();
            js_path.push("app.js");

            let mut wasm_path = out_path.clone();
            wasm_path.push("app_bg.wasm");

            Ok(WasmPackage::new(lib, wasm_path, js_path))
        },
        _ => Err(WasmPackageError)
    }
}

// TODO replace Box<Error> with an enum of all the possible errors
fn main() -> Result<(), Box<Error>>{
    let handlebars = load_templates()?;

    let yaml = load_yaml!("cli.yaml");
    let app = App::from_yaml(yaml);
    let input = app.get_matches();

    // Get the project path either from being passed in as an arg or
    // default to the current directory
    let project_path = input.args.get("project")
        .map_or(env::current_dir(),
                |arg| Ok(PathBuf::from(&arg.vals[0])))?;
    println!("Using project path {}", project_path.to_str().unwrap());

    let home_path = input.args.get("home")
        .map_or(default_home_path(),
                |arg| Ok(PathBuf::from(&arg.vals[0])))?;
    println!("Using home path {}", home_path.to_str().unwrap());

    // Load the config if present or use default config
    let mut conf_path = project_path.clone();
    conf_path.push("woz.toml");
    let conf_str = fs::read_to_string(conf_path.clone())
        .or_else(|e| {
            println!("Couldn't find woz config at {}", conf_path.to_str().unwrap());
            Err(e)
        })
        .expect("Failed to load conf");
    let conf: Config = toml::from_str(&conf_str)?;

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
                    .map_err(|e| println!("{}", e.description()))
                    .and_then(|resp| {
                        let user_id = resp.user_sub;
                        store_user_id(&home_path, &user_id);
                        Ok(())
                    })
                    .expect("An error occured");
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
                        dbg!(err);
                        let creds = prompt::login();
                        account::login(&id_provider_client, creds.username, creds.password).sync()
                            .and_then(|resp| {
                                let token = resp.clone()
                                    .authentication_result.expect("Failed")
                                    .refresh_token.expect("Missing refresh token");
                                store_token(&home_path, &token);
                                Ok(resp)})
                    })
                    .expect("Failed to id token")
                    .authentication_result.expect("No auth result")
                    .id_token.expect("No access token");
                let identity_id = ensure_identity_id(&home_path, &id_client, &id_token);
                let aws_token = account::aws_credentials(&id_client, &identity_id, &id_token)
                    .sync()
                    .expect("Failed to fetch AWS credentials");

                let aws_creds = aws_token.credentials.expect("Missing credentials");
                let creds_provider = StaticProvider::new(
                    aws_creds.access_key_id.expect("Missing access key"),
                    aws_creds.secret_key.expect("Missing secret key"),
                    Some(aws_creds.session_token.expect("Missing session token")),
                    Some(aws_creds.expiration.expect("Missing expiration") as i64)
                );

                let request_dispatcher = HttpClient::new();
                let s3_client = S3Client::new_with(
                    request_dispatcher.expect("Failed to make an HttpClient"),
                    creds_provider,
                    Region::UsWest2
                );

                let mut out_path = home_path.clone();
                out_path.push(&project_id);
                out_path.push("pkg");
                fs::create_dir_all(&out_path).expect("Failed to make pkg directory");

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
                ).expect("Failed to generate wasm package");

                // All app files will be prefixed in the s3 bucket by the user's
                // cognito identity ID and project_id
                let key_prefix = format!("{}/{}", &identity_id, &project_id);

                let files = vec![
                    (format!("{}/index.html", key_prefix),
                     String::from("text/html"),
                     index_template.expect("Failed to render index.html").into_bytes()),
                    (format!("{}/manifest.json", key_prefix),
                     String::from("application/manifest+json"),
                     manifest_template.expect("Failed to render manifest.json").into_bytes()),
                    (format!("{}/sw.js", key_prefix),
                     String::from("application/javascript"),
                     service_worker_template.expect("Failed to render sw.js").into_bytes()),
                    (format!("{}/app.js", key_prefix),
                     String::from("application/javascript"),
                     fs::read_to_string(wasm_package.js).expect("Failed to read js file").into_bytes()),
                    (format!("{}/app.wasm", key_prefix),
                     String::from("application/wasm"),
                     {
                         let mut f = File::open(wasm_package.wasm).expect("Failed to read wasm file");
                         let mut buffer = Vec::new();
                         f.read_to_end(&mut buffer).expect("Failed to read to bytes");
                         buffer
                     }),
                ];

                for (file_name, mimetype, body) in files.into_iter() {
                    let req = PutObjectRequest {
                        bucket: String::from(S3_BUCKET_NAME),
                        key: file_name,
                        body: Some(ByteStream::from(body)),
                        content_type: Some(mimetype),
                        ..Default::default()
                    };

                    s3_client.put_object(req)
                        .sync()
                        .map_err(|err| {
                            match err {
                                PutObjectError::Unknown(resp) => {
                                    dbg!(str::from_utf8(&resp.body)
                                         .expect("Failed to parse response body"));
                                },
                                PutObjectError::Credentials(e) => {dbg!(e);},
                                _ => {dbg!("err");}
                            };
                            panic!("ut oh");
                        })
                        .and_then(|resp| Ok(dbg!(resp)))
                        .ok();
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
    }
    Ok(())
}
