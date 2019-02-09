use std::collections::HashMap;
use std::io::{stdin, stdout, Write};


#[macro_use]
extern crate clap;
use clap::{App, ArgMatches};

use rusoto_core::{Region, ByteStream, RusotoFuture};
use rusoto_cognito_identity::{CognitoIdentity, CognitoIdentityClient, GetIdInput, ListIdentityPoolsInput};
use rusoto_cognito_idp::*;

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
    request.auth_parameters = Some(auth_params);
    client.initiate_auth(request)
}

fn main() {
    let yaml = load_yaml!("cli.yaml");
    let app = App::from_yaml(yaml);
    let input = app.get_matches();
    // TODO based on the sub command sign up the user

    if let Some(sub) = input.subcommand_name() {
        match Command::from(sub) {
            Command::Setup => {
                println!("Entering setup...");
                let mut username = String::new();

                print!("Username: ");
                stdout().flush().expect("Error");
                stdin().read_line(&mut username).expect("Please enter a username");
                if let Some('\n') = username.chars().next_back() {
                    username.pop();
                }
                if let Some('\r') = username.chars().next_back() {
                    username.pop();
                }
                println!("Your username: {}", username);
            },
            Command::Init => println!("Init..."),
            // TODO default to showing help
            Command::Unknown => println!("Unknown command"),
            _ => unimplemented!("Not implemented yet")
        };
    } else {
        // TODO show help

    }
}
