//! The cli used to interact with the woz service.
use std::io::Write;
use std::path::PathBuf;
use std::fs::File;
use std::fs;
use std::str;
use std::env;
use std::error::Error as StdError;
use std::process;
use toml;

#[macro_use] extern crate clap;
use clap::App;

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate failure;

use failure::Error;
use failure::ResultExt;

use rusoto_core::Region;
use rusoto_cognito_idp::*;

mod prompt;
mod account;
mod config;
mod template;
mod package;
mod cache;
mod builder;
mod components;
mod upload_client;

use config::*;
use template::load_templates;
use cache::FileCache;
use builder::AppBuilder;
use components::wasm::WasmComponent;
use components::pwa::PwaComponent;
use components::icon::IconComponent;
use components::splashscreen::SplashscreenComponent;


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
wasm-bindgen = \"0.2.40\"
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
lib=\"wasm-bindgen\"
wasm_path=\"target/wasm32-unknown-unknown/release/{}.wasm\"
", project_name, project_name, project_name, project_name).as_bytes()).unwrap();

                println!("Ready to be deployed with 'woz deploy'");
            },
            Command::Deploy => {
                println!("Deploying...");
                // First compile the project in release mode
                let mut build_proc = process::Command::new("sh")
                    .arg("-c")
                    .arg("cargo build --release --target wasm32-unknown-unknown")
                    .stdout(process::Stdio::piped())
                    .spawn()
                    .context("Failed to spawn build")?;
                let exit_code = build_proc.wait().context("Failed to wait for build")?;
                if !exit_code.success() {
                    return Err(format_err!("Build failed, please check output above."))
                }

                // Load the woz config if present or use default config
                let mut conf_path = project_path.clone();
                conf_path.push("woz.toml");
                let conf_str = fs::read_to_string(conf_path.clone())
                    .context(format!("Couldn't find woz config file at {}",
                                     conf_path.clone().to_str().unwrap()))?;
                let conf: Config = toml::from_str(&conf_str)
                    .context("Failed to parse woz config")?;

                let s3_client = upload_client::authenticated_client(&cache)
                    .context("Unable to initialize upload client")?;

                // Here we are relying on this implicit, synchronous
                // behavior and reading the identity ID from the file
                // cache because at this point it should be there.
                //
                // TODO: Probably want to guarantee that the identity
                // ID exists in the file cache before we even get to
                // calling this subcommand e.g. when first setting up
                // the account.
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

                // Build the app with all the components
                let wasm_cmpnt = WasmComponent::new(wasm_path, out_path);
                let pwa_cmpnt = PwaComponent::new(&conf, handlebars);
                let icon_cmpnt = IconComponent::new(&conf);
                let splashscreen_cmpnt = SplashscreenComponent::new(&conf);

                let mut app = AppBuilder::new(s3_client, key_prefix);
                app
                    .component(wasm_cmpnt)
                    .component(pwa_cmpnt)
                    .component(icon_cmpnt)
                    .component(splashscreen_cmpnt);

                // TODO check the size of the app
                app.upload().context("Failed to upload app")?;

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
