//! The cli used to interact with the woz service.
//!
//! # Usage
//!
//! Install from the [woz site](https://woz.sh) and run the following.
//!
//! ```
//! woz setup
//! ```
//!
//! Follow the instructions to activate your account. Now initialize the current directory to make a woz app.
//!
//! ```
//! woz init
//! ```
//!
//! To deploy to your space:
//!
//! ```
//! woz deploy
//! ```

use std::collections::HashMap;
use std::io;
use std::io::prelude::*;
use std::io::{stdin, stdout, Write};
use std::fs::File;
use std::path::PathBuf;
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
use serde::{Deserialize, Deserializer};
#[macro_use]
extern crate serde_json;

use rusoto_core::{Region, RusotoFuture, ByteStream};
use rusoto_core::request::HttpClient;
use rusoto_credential::StaticProvider;
use rusoto_cognito_identity::*;
use rusoto_cognito_idp::CognitoIdentityProvider;
use rusoto_cognito_idp::*;
use rusoto_s3::*;

use handlebars::Handlebars;


const SCHEME: &str = env!("WOZ_WEB_SCHEME");
const NETLOC: &str = env!("WOZ_WEB_NETLOC");
const IDENTITY_POOL_ID: &str = env!("WOZ_IDENTITY_POOL_ID");
const IDENTITY_POOL_URL: &str = env!("WOZ_IDENTITY_POOL_URL");
const CLIENT_ID: &str = env!("WOZ_CLIENT_ID");
const S3_BUCKET_NAME: &str = env!("WOZ_S3_BUCKET_NAME");

#[derive(Debug, Serialize)]
enum Lib {
    WasmBindgen,
    StdWeb,
    Unknown(String)
}

impl<'de> Deserialize<'de> for Lib {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "wasm-bindgen" => Lib::WasmBindgen,
            "std-web" => Lib::StdWeb,
            _ => Lib::Unknown(s),
        })
    }
}

#[derive(Debug, Serialize)]
enum Environment {
    Release,
    Development,
    Unknown(String)
}

impl<'de> Deserialize<'de> for Environment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "release" => Environment::Release,
            "development" => Environment::Development,
            _ => Environment::Unknown(s),
        })
    }
}

#[derive(Debug, Serialize)]
struct ProjectId(String);

impl<'de> Deserialize<'de> for ProjectId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        if s.chars().all(char::is_alphanumeric) {
            Ok(ProjectId(s))
        } else {
            Err(serde::de::Error::custom(String::from("must be alphanumeric")))
        }
    }
}

#[derive(Debug, Deserialize)]
struct Config {
    project_id: ProjectId,
    lib: Option<Lib>,
    name: String,
    short_name: Option<String>,
    env: Option<Environment>,
    wasm_path: PathBuf
}

impl Default for Config {
    fn default() -> Self {
        Self {
            project_id: ProjectId(String::from("default")),
            lib: Some(Lib::WasmBindgen),
            name: String::from("My App"),
            short_name: Some(String::from("App")),
            env: Some(Environment::Release),
            wasm_path: PathBuf::new(),
        }
    }
}

fn get_home_path() -> PathBuf {
    let home: String = std::env::var_os("XDG_CONFIG_HOME")
        .or_else(|| std::env::var_os("HOME"))
        .map(|v| v.into_string().expect("Unable to parse $HOME to string"))
        .expect("No home");
    let mut buf = PathBuf::new();
    buf.push(home);
    buf.push(".woz");
    buf
}

#[test]
// TODO only compile on macOS
fn get_home_path_test() {
    let user = std::env::var_os("USER")
        .map(|v| v.into_string().expect("Could not parse $USER to string"))
        .expect("Could not get a $USER");
    let path_str = format!("/Users/{}/.woz", user);
    assert_eq!(PathBuf::from(path_str), get_home_path());
}

enum Command {
    Init,
    Setup,
    Deploy,
    Update,
    Unknown,
}

impl From<&str> for Command {
    fn from(s: &str) -> Command {
        match s {
            "init" => Command::Init,
            "setup" => Command::Setup,
            "deploy" => Command::Deploy,
            "update" => Command::Update,
            _ => Command::Unknown
        }
    }
}


fn signup(client: CognitoIdentityProviderClient, email: String, username: String, password: String) -> RusotoFuture<SignUpResponse, SignUpError> {
    // Build the request
    let mut request = SignUpRequest::default();
    request.username = username;
    request.password = password;
    request.client_id = String::from(CLIENT_ID);
    let email = AttributeType {
        name: String::from("email"),
        value: Some(email)
    };
    request.user_attributes = Some(vec![email]);

    // Make the request
    client.sign_up(request)
}

fn login(client: &CognitoIdentityProviderClient, username: String, password: String) -> RusotoFuture<InitiateAuthResponse, InitiateAuthError> {
    let mut request = InitiateAuthRequest::default();
    request.auth_flow = String::from("USER_PASSWORD_AUTH");
    let mut auth_params = HashMap::new();
    auth_params.insert(String::from("USERNAME"), username);
    auth_params.insert(String::from("PASSWORD"), password);
    request.client_id = String::from(CLIENT_ID);
    request.auth_parameters = Some(auth_params);
    client.initiate_auth(request)
}

#[derive(Debug, Clone)]
struct SignupFormValues {
    email: String,
    username: String,
    password: String,
}

fn signup_form() -> SignupFormValues {
    // TODO validate input
    println!("Entering setup...");
    print!("Please enter a username: ");
    stdout().flush().expect("Error");
    let username_buffer = &mut String::new();
    stdin().read_line(username_buffer).expect("Fail");
    let username = username_buffer.trim_end();

    print!("Please enter a password: ");
    stdout().flush().expect("Error");
    let password_buffer = &mut String::new();
    stdin().read_line(password_buffer).expect("Fail");
    let password = password_buffer.trim_end();

    print!("Please enter a email: ");
    stdout().flush().expect("Error");
    let email_buffer = &mut String::new();
    stdin().read_line(email_buffer).expect("Fail");
    let email = email_buffer.trim_end();

    SignupFormValues {
        email: email.to_owned(),
        username: username.to_owned(),
        password: password.to_owned()
    }
}

fn prompt_login() -> Credentials {
    print!("Please enter a username: ");
    stdout().flush().expect("Error");
    let username_buffer = &mut String::new();
    stdin().read_line(username_buffer).expect("Fail");
    let username = username_buffer.trim_end();

    print!("Please enter a password: ");
    stdout().flush().expect("Error");
    let password_buffer = &mut String::new();
    stdin().read_line(password_buffer).expect("Fail");
    let password = password_buffer.trim_end();

    Credentials {
        username: username.to_owned(),
        password: password.to_owned(),
    }
}

struct Credentials {
    username: String,
    password: String,
}

type IdentityID = String;
type RefreshToken = String;

fn refresh_auth(client: &CognitoIdentityProviderClient, refresh_token: &str)
                -> RusotoFuture<InitiateAuthResponse, InitiateAuthError> {
    let mut auth_params = HashMap::new();
    auth_params.insert(String::from("REFRESH_TOKEN"), refresh_token.to_string());
    let req = InitiateAuthRequest {
        client_id: CLIENT_ID.to_string(),
        auth_flow: String::from("REFRESH_TOKEN_AUTH"),
        auth_parameters: Some(auth_params),
        ..Default::default()
    };
    client.initiate_auth(req)
}

fn identity_id(client: &CognitoIdentityClient, id_token: &str)
               -> RusotoFuture<GetIdResponse, GetIdError> {
    let mut logins = HashMap::new();
    logins.insert(IDENTITY_POOL_URL.to_string(), id_token.to_owned());

    let mut req = GetIdInput::default();
    req.identity_pool_id = IDENTITY_POOL_ID.to_string();
    req.logins = Some(logins);
    client.get_id(req)
}

fn aws_credentials(client: &CognitoIdentityClient, identity_id: &str, id_token: &str)
                   -> RusotoFuture<GetCredentialsForIdentityResponse,
                                  GetCredentialsForIdentityError> {
    let mut logins = HashMap::new();
    logins.insert(IDENTITY_POOL_URL.to_string(), id_token.to_owned());

    let mut req = GetCredentialsForIdentityInput::default();
    req.identity_id = identity_id.to_owned();
    req.logins = Some(logins);
    client.get_credentials_for_identity(req)
}

/// Attempt to get a refresh_token token, prompting the user to log in if
/// refresh token is expired and stores it locally.
fn ensure_refresh_token(client: &CognitoIdentityProviderClient) -> RefreshToken {
    let mut path = get_home_path();
    path.push(".refresh_token");
    fs::read_to_string(path)
        .or_else::<io::Error, _>(|_| {
            let creds = prompt_login();
            let token = login(&client, creds.username, creds.password).sync()
                .and_then(|resp| {
                    let token = resp
                        .authentication_result.expect("Failed")
                        .refresh_token.expect("Missing refresh token");
                    store_token(&token);
                    Ok(token)})
                .or_else::<io::Error, _>(|_| Ok(ensure_refresh_token(client)))
                .expect("Something went wrong");
            Ok(token)
        })
        .unwrap()
}

fn ensure_identity_id(client: &CognitoIdentityClient, id_token: &str)
                      -> IdentityID {
    let mut path = get_home_path();
    path.push(".identity");
    fs::read_to_string(path)
        .or_else::<io::Error, _>(|_| {
            let id = identity_id(client, id_token)
                .sync()
                .expect("Failed to get identity ID")
                .identity_id.expect("No identity ID");
            store_identity_id(&id);
            Ok(id)
        })
        .unwrap()
}

fn store_token(refresh_token: &str) {
    let home: String = std::env::var_os("XDG_CONFIG_HOME")
        .or_else(|| std::env::var_os("HOME"))
        .map(|v| v.into_string().unwrap())
        .expect("No home");
    let mut home_path = PathBuf::new();
    home_path.push(home);
    home_path.push(".woz");

    let mut file_path = home_path.clone();
    file_path.push(".refresh_token");

    fs::create_dir_all(home_path).unwrap();
    let mut f = File::create(&file_path).unwrap();
    f.write_all(refresh_token.as_bytes()).unwrap();
}

fn store_user_id(user_id: &str) {
    let home: String = std::env::var_os("XDG_CONFIG_HOME")
        .or_else(|| std::env::var_os("HOME"))
        .map(|v| v.into_string().unwrap())
        .expect("No home");
    let mut home_path = PathBuf::new();
    home_path.push(home);
    home_path.push(".woz");

    let mut file_path = home_path.clone();
    file_path.push(".user");
    fs::create_dir_all(home_path).unwrap();
    let mut f = File::create(&file_path).unwrap();
    f.write_all(user_id.as_bytes()).unwrap();
}

fn store_identity_id(id: &str) {
    let home: String = std::env::var_os("XDG_CONFIG_HOME")
        .or_else(|| std::env::var_os("HOME"))
        .map(|v| v.into_string().unwrap())
        .expect("No home");
    let mut home_path = PathBuf::new();
    home_path.push(home);
    home_path.push(".woz");

    let mut file_path = home_path.clone();
    file_path.push(".identity");
    fs::create_dir_all(home_path).unwrap();
    let mut f = File::create(&file_path).unwrap();
    f.write_all(id.as_bytes()).unwrap();
}

// TODO inline all the templates and register them with handlebars
// TODO can we lazy_static all of these?
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

#[test]
fn test_wasm_package() {
    let actual = wasm_package(
        Lib::WasmBindgen,
        PathBuf::from("/Users/ender/Projects/wozm/examples/wasm-pack-percy/target/wasm32-unknown-unknown/release/wasm_pack_percy.wasm"),
        PathBuf::from("/Users/ender/.woz/bindings"),
    );
    actual.unwrap();
}

// TODO replace Box<Error> with an enum of all the possible errors
fn main() -> Result<(), Box<Error>>{
    let yaml = load_yaml!("cli.yaml");
    let app = App::from_yaml(yaml);
    let handlebars = load_templates()?;
    let input = app.get_matches();

    // Get the project path either from being passed in as an arg or
    // default to the current directory
    let project_path = input.args.get("path")
        .map_or(env::current_dir(), |arg| Ok(PathBuf::from(&arg.vals[0])))?;

    // Load the config if present or use default config
    let mut conf_path = project_path.clone();
    conf_path.push("woz.toml");
    let conf_str = fs::read_to_string(conf_path)?;
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
                let values = signup_form();
                dbg!(values.clone());
                let client = CognitoIdentityProviderClient::new(Region::UsWest2);
                signup(client, values.email, values.username, values.password)
                    .sync()
                    .map_err(|e| {
                        let msg = match e {
                            SignUpError::InvalidParameter(msg) => msg,
                            _ => String::from("An unknown error has occurred")
                        };
                        println!("{}", msg);
                    })
                    .and_then(|resp| {
                        let user_id = resp.user_sub;
                        store_user_id(&user_id);
                        Ok(())
                    })
                    .expect("An error occured");
                println!("Please check your inbox for an email to verify your account.");
            },
            // Init should result in
            // 1. A config file in the current directory
            // 2. A new subdomain on woz
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

                let refresh_token = ensure_refresh_token(&id_provider_client);
                let id_token = refresh_auth(&id_provider_client, &refresh_token)
                    .sync()
                    .or_else(|err| {
                        dbg!(err);
                        let creds = prompt_login();
                        login(&id_provider_client, creds.username, creds.password).sync()
                            .and_then(|resp| {
                                let token = resp.clone()
                                    .authentication_result.expect("Failed")
                                    .refresh_token.expect("Missing refresh token");
                                store_token(&token);
                                Ok(resp)})
                    })
                    .expect("Failed to id token")
                    .authentication_result.expect("No auth result")
                    .id_token.expect("No access token");
                let identity_id = ensure_identity_id(&id_client, &id_token);
                let aws_token = aws_credentials(&id_client, &identity_id, &id_token)
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

                let mut out_path = get_home_path();
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
