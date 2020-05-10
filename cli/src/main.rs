//! The cli used to interact with the woz service.
use std::io::Write;
use std::path::PathBuf;
use std::fs::File;
use std::fs;
use std::str;
use std::env;
use std::process;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use toml;

#[macro_use] extern crate clap;
use clap::App;

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate failure;
extern crate regex;
extern crate tokio;

use failure::Error;
use failure::ResultExt;

use rusoto_core::Region;
use rusoto_cognito_idp::*;
use rusoto_cognito_identity::*;

mod prompt;
mod account;
mod config;
mod template;
mod cache;
mod builder;
mod components;
mod upload_client;
mod file_upload;

use config::*;
use template::load_templates;
use cache::FileCache;
use builder::AppBuilder;
use components::wasm::WasmComponent;
use components::pwa::PwaComponent;
use components::icon::IconComponent;
use components::splashscreen::SplashscreenComponent;
use components::landing_page::LandingPageComponent;


enum Command {
    Build,
    Deploy,
    Init,
    NewProject,
    Setup,
    Signup,
    Update,
    Unknown,
}

impl From<&str> for Command {
    fn from(s: &str) -> Command {
        match s {
            "build" => Command::Build,
            "deploy" => Command::Deploy,
            "init" => Command::Init,
            "new" => Command::NewProject,
            "setup" => Command::Setup,
            "signup" => Command::Signup,
            "update" => Command::Update,
            _ => Command::Unknown
        }
    }
}

fn random_version() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .collect()
}

#[test]
fn random_version_works() {
    assert_eq!(7, random_version().len());
}

async fn run() -> Result<(), Error> {
    let handlebars = load_templates().context("Failed to load templates")?;

    let yaml = load_yaml!("cli.yaml");
    let app = App::from_yaml(yaml).version(&crate_version!()[..]);
    let input = app.get_matches();

    // Get the project path either from being passed in as an arg or
    // default to the current directory
    let project_path = input.args.get("project")
        .map_or(env::current_dir(),
                |arg| Ok(PathBuf::from(&arg.vals[0])))
        .context("Failed to get project path")?;
    println!("Using project path {}", project_path.to_str().unwrap());

    let conf_path = input.args.get("config")
        .map_or({let mut c_path = project_path.clone();
                 c_path.push("woz.toml");
                 c_path},
                |arg| PathBuf::from(&arg.vals[0]));
    println!("Using config path {}", conf_path.to_str().unwrap());

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
            // 2. A verified email address
            // 3. A unique user ID
            // 4. A refresh token
            Command::Signup => {
                fs::create_dir_all(&home_path).context("Failed to make home directory")?;

                // TODO first check if there is an existing installation
                let values = prompt::signup();
                let id_provider_client = account::anonymous_identity_provider_client();
                let id_client = account::anonymous_identity_client();

                let user_id = account::signup(&id_provider_client,
                                              values.email.clone(),
                                              values.username.clone(),
                                              values.password.clone())
                    .await
                    .context("Signup failed")?
                    .user_sub;

                cache.set("user", user_id.as_bytes().to_vec())
                    .context("Failed to add user ID to cache")?;

                // Prompt the user to confirm they clicked the verification link
                let mut email_verified = false;

                while !email_verified {
                    println!("Please check your inbox for an email to verify your account.");
                    if prompt::is_email_verified() {
                        // It's still possible for this to fail if the user
                        // says they are verified, but they aren't
                        account::setup(&id_provider_client,
                                       &id_client,
                                       &cache,
                                       values.username.clone(),
                                       values.password.clone())
                            .await
                            .or_else(|e| {
                                match e {
                                    InitiateAuthError::UserNotConfirmed(_) => {
                                        println!("Auth failed, have you clicked the link in the verification email?");
                                        email_verified = false;
                                        Ok(())
                                    },
                                    _ => Err(e)
                                }
                            })
                            .context("Failed to set up account")?;

                        email_verified = true;
                    }
                };

                println!("Your account has been successfully set up! You can now deploy to your applications using 'woz deploy'");
            },
            Command::Setup => {
                let values = prompt::login();
                let id_provider_client = account::anonymous_identity_provider_client();
                let id_client = account::anonymous_identity_client();

                account::setup(&id_provider_client,
                               &id_client,
                               &cache,
                               values.username.clone(),
                               values.password.clone())
                    .await
                    .expect("Unable to login and perform local set up");
            },
            Command::NewProject => {
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

                cargo_conf.write_all("seed = \"0.4.0\"
wasm-bindgen = \"0.2.48\"
web-sys = \"0.3.25\"

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
# Optional project url that will be used in html meta tags
# project_url=\"https://example.com\"
bg_color=\"black\"
lib=\"wasm-bindgen\"
# For production deploys
env=\"production\"
wasm_path=\"target/wasm32-unknown-unknown/release/{}.wasm\"
# Uncomment these values for faster to compile development builds
# env=\"development\"
# wasm_path=\"target/wasm32-unknown-unknown/debug/seed_app.wasm\"
", project_name, project_name).as_bytes()).unwrap();

                // Write a hello world lib.rs
                let mut default_lib_rs = File::create(
                    PathBuf::from(format!("{}/src/lib.rs", project_name))
                ).context("Failed to create lib.rs")?;
                default_lib_rs.write_all(DEFAULT_PROJECT_LIB_RS.as_bytes()).unwrap();
                println!("New project created! Please cd to ./{}", project_name);
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
# Optional project url that will be used in html meta tags
# project_url=\"https://example.com\"
bg_color=\"black\"
lib=\"wasm-bindgen\"
# For production deploys
env=\"production\"
wasm_path=\"target/wasm32-unknown-unknown/release/{}.wasm\"
# Uncomment these values for faster to compile development builds
# env=\"development\"
# wasm_path=\"target/wasm32-unknown-unknown/debug/seed_app.wasm\"
", project_name, project_name, project_name, project_name).as_bytes()).unwrap();

                println!("Ready to be deployed with 'woz deploy'");
            },
            Command::Build => {
                println!("Building...");
                let version = random_version();

                // Load the woz config if present or use default config
                let conf_str = fs::read_to_string(conf_path.clone())
                    .context(format!("Couldn't find woz config file at {}",
                                     conf_path.clone().to_str().unwrap()))?;
                let conf: Config = toml::from_str(&conf_str)
                    .context("Failed to parse woz config")?;

                let ProjectId(project_id) = conf.project_id.clone();
                let mut out_path = home_path.clone();
                out_path.push(&project_id);
                out_path.push("pkg");
                fs::create_dir_all(&out_path).context("Failed to make pkg directory")?;

                let mut wasm_path = project_path.clone();
                wasm_path.push(conf.wasm_path.clone());

                let url = conf.project_url.clone().unwrap_or(format!(
                    "{}://{}/{}/index.html",
                    SCHEME,
                    NETLOC,
                    project_id
                ));

                // Build the app with all the components
                let landing_page_cmpnt = LandingPageComponent::new(
                    &conf,
                    &url,
                    &handlebars
                );
                let wasm_cmpnt = WasmComponent::new(wasm_path, &out_path);
                let pwa_cmpnt = PwaComponent::new(
                    &conf,
                    &url,
                    &handlebars,
                    &version
                );
                let icon_cmpnt = IconComponent::new(&conf);
                let splashscreen_cmpnt = SplashscreenComponent::new(&conf);

                let file_prefix = String::from(out_path.to_str().unwrap());
                let build_env = &conf.env.to_owned().unwrap_or(Environment::Development);
                let mut app = AppBuilder::new();
                app
                    .component(&landing_page_cmpnt)
                    .component(&wasm_cmpnt)
                    .component(&pwa_cmpnt)
                    .component(&icon_cmpnt)
                    .component(&splashscreen_cmpnt)
                    .build(&project_path, &file_prefix, &build_env)
                    .context("Failed to build app")?;

                app.download().context("Failed to download files from the build")?;
                println!("App package directory can be found at {}", file_prefix);
            },
            Command::Deploy => {
                println!("Deploying...");
                let version = random_version();

                // Load the woz config if present or use default config
                let conf_str = fs::read_to_string(conf_path.clone())
                    .context(format!("Couldn't find woz config file at {}",
                                     conf_path.clone().to_str().unwrap()))?;
                let conf: Config = toml::from_str(&conf_str)
                    .context("Failed to parse woz config")?;

                let s3_client = upload_client::authenticated_client(&cache)
                    .await
                    .context("Unable to initialize upload client")?;

                let identity_id = cache.get("identity")
                    .context("Unable to retrieve user ID")?;

                let ProjectId(project_id) = conf.project_id.clone();
                let mut out_path = home_path.clone();
                out_path.push(&project_id);
                out_path.push("pkg");
                fs::create_dir_all(&out_path).context("Failed to make pkg directory")?;

                // All app files will be prefixed in the s3 bucket by the user's
                // cognito identity ID and project_id
                let key_prefix = format!("{}/{}", &identity_id, &project_id);

                let mut wasm_path = project_path.clone();
                wasm_path.push(conf.wasm_path.clone());

                let url = conf.project_url.clone().unwrap_or(format!(
                    "{}://{}/{}/{}/index.html",
                    SCHEME,
                    NETLOC,
                    identity_id,
                    project_id
                ));

                // Build the app with all the components
                let landing_page_cmpnt = LandingPageComponent::new(
                    &conf,
                    &url,
                    &handlebars
                );
                let wasm_cmpnt = WasmComponent::new(wasm_path, &out_path);
                let pwa_cmpnt = PwaComponent::new(
                    &conf,
                    &url,
                    &handlebars,
                    &version
                );
                let icon_cmpnt = IconComponent::new(&conf);
                let splashscreen_cmpnt = SplashscreenComponent::new(&conf);

                let build_env = &conf.env.to_owned().unwrap_or(Environment::Development);
                let mut app = AppBuilder::new();
                app
                    .component(&landing_page_cmpnt)
                    .component(&wasm_cmpnt)
                    .component(&pwa_cmpnt)
                    .component(&icon_cmpnt)
                    .component(&splashscreen_cmpnt)
                    .build(&project_path, &key_prefix, &build_env)
                    .context("Failed to build app")?;

                // Sets an upper bounds for the size and app that can
                // be uploaded to prevent allowing really big files
                // from being uploaded accidentally
                let app_size = app.size();
                if (app_size / 1_000_000) > MAX_APP_SIZE_MB {
                    return Err(
                        format_err!(
                            "The maximum size for deploying an app is {}MB. Your app is {}MB",
                            MAX_APP_SIZE_MB,
                            app_size
                        )
                    )
                }
                app.upload(s3_client).context("Failed to upload app")?;
                println!("{}", format!("Your app is available at {}", url));
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

#[tokio::main]
async fn main() {
    run().await
        .map_err(|e| {
            println!("{}\n{}", e,
                     e.iter_causes()
                     .map(|f| format!("Caused by: {}", f))
                     .collect::<Vec<String>>()
                     .join("\n"));
            std::process::exit(1)
        })
        .ok();
    std::process::exit(0)
}
