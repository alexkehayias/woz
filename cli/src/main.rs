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
use std::io::{stdin, stdout, Write};
use std::error::Error;
use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::fs;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate clap;
use clap::App;

use rusoto_core::{Region, RusotoFuture};
use rusoto_cognito_identity::{CognitoIdentity, CognitoIdentityClient, GetIdInput, ListIdentityPoolsInput};
use rusoto_cognito_idp::*;


// fn get_home_path() -> Path {
//     let home: String = std::env::var_os("XDG_CONFIG_HOME")
//         .or(std::env::var_os("HOME"))
//         .map(|v| v.into_string().expect("Unable to parse env var"))
//         .expect("No home");
//     Path::new(&home).join("/.woz").as_path()
// }

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
            "Update" => Command::Update,
            _ => Command::Unknown
        }
    }
}


fn signup(client: CognitoIdentityProviderClient, email: String, username: String, password: String) -> RusotoFuture<SignUpResponse, SignUpError> {
    // Build the request
    let mut request = SignUpRequest::default();
    request.username = username;
    request.password = password;
    request.client_id = String::from("fbg7q8rv3iu8d8r7n86isq4mg");
    let email = AttributeType {
        name: String::from("email"),
        value: Some(email)
    };
    request.user_attributes = Some(vec![email]);

    // Make the request
    client.sign_up(request)
}

fn login(client: CognitoIdentityProviderClient, username: String, password: String) -> RusotoFuture<InitiateAuthResponse, InitiateAuthError> {
    let mut request = InitiateAuthRequest::default();
    request.auth_flow = String::from("USER_PASSWORD_AUTH");
    let mut auth_params = HashMap::new();
    auth_params.insert(String::from("USERNAME"), username);
    auth_params.insert(String::from("PASSWORD"), password);
    request.client_id = String::from("fbg7q8rv3iu8d8r7n86isq4mg");
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
    let username = username_buffer.trim_right();

    print!("Please enter a password: ");
    stdout().flush().expect("Error");
    let password_buffer = &mut String::new();
    stdin().read_line(password_buffer).expect("Fail");
    let password = password_buffer.trim_right();

    print!("Please enter a email: ");
    stdout().flush().expect("Error");
    let email_buffer = &mut String::new();
    stdin().read_line(email_buffer).expect("Fail");
    let email = email_buffer.trim_right();

    SignupFormValues {
        email: email.to_owned(),
        username: username.to_owned(),
        password: password.to_owned()
    }
}

struct Credentials {
    username: String,
    password: String,
}

fn fetch_credentials() -> Credentials {
    Credentials {
        username: String::from(""),
        password: String::from(""),
    }
}

fn main() {
    let yaml = load_yaml!("cli.yaml");
    let app = App::from_yaml(yaml);
    let input = app.get_matches();

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
                        dbg!(user_id);
                        // TODO store this someplace;
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
                let creds = fetch_credentials();
                let client = CognitoIdentityProviderClient::new(Region::UsWest2);
                login(client, creds.username, creds.password)
                    .sync()
                    .map_err(|e| println!("{}", e.to_string()))
                    .and_then(|resp| {
                        if let Some(AuthenticationResultType {access_token, refresh_token, ..} ) = resp.authentication_result {
                            // TODO save the refresh token
                            let token = refresh_token.expect("Missing refresh token");

                            let home: String = std::env::var_os("XDG_CONFIG_HOME")
                                .or(std::env::var_os("HOME"))
                                .map(|v| v.into_string().unwrap())
                                .expect("No home");
                            let mut home_path = PathBuf::new();
                            home_path.push(home);
                            home_path.push(".woz");
                            dbg!(home_path.clone());

                            let mut file_path = home_path.clone();
                            file_path.push(".refresh_token");

                            fs::create_dir_all(home_path).unwrap();
                            let mut f = File::create(&file_path).unwrap();
                            f.write_all(token.as_bytes()).unwrap();
                            Ok(())
                        } else {
                            Err(())
                        }
                    })
                    .expect("An error occured");

                // TODO preflight auth
            },
            // TODO default to showing help
            Command::Unknown => println!("Unknown command"),
            _ => unimplemented!("Not implemented yet")
        };
    }
}
