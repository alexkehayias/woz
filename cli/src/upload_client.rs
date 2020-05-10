//! The cli used to interact with the woz service.
use failure::Error;
use failure::ResultExt;
use rusoto_core::Region;
use rusoto_core::request::HttpClient;
use rusoto_credential::StaticProvider;
use rusoto_cognito_idp::*;
use rusoto_s3::*;

use crate::cache::FileCache;
use crate::prompt;
use crate::account;


/// Attempt to get a refresh_token token, prompting the user to log in if
/// refresh token is expired and stores it locally.
async fn ensure_refresh_token(cache: &FileCache, client: &CognitoIdentityProviderClient) -> String {
    let mut token = None;

    while let None = token {
        match cache.get_encrypted("refresh_token") {
            Ok(t) => token = Some(t),
            Err(_) => {
                let creds = prompt::login();
                let resp = account::login(&client, &creds.username, &creds.password).await;

                // Keep retrying login until successful
                if resp.is_err() {
                    continue
                };

                let t = resp.unwrap().authentication_result
                    .expect("Failed")
                    .refresh_token
                    .expect("Missing refresh token");

                cache.set_encrypted("refresh_token", t.as_bytes().to_vec())
                    .expect("Failed to cache refresh token");

                token = Some(t)
            }
        }
    };

    token.unwrap()
}

async fn ensure_id_token(cache: &FileCache, id_provider_client: &CognitoIdentityProviderClient, refresh_token: &String) -> String {
    let result = account::refresh_auth(&id_provider_client, &refresh_token).await;

    match result {
        Ok(resp) => resp
            .authentication_result.expect("No auth result")
            .id_token.expect("No ID token"),
        Err(error) => {
            println!("Getting refresh token failed: {}", error);
            let mut id_token = None;

            while let None = id_token {
                let creds = prompt::login();
                let username = creds.username;
                let password = creds.password;

                let result = account::login(&id_provider_client, &username, &password).await;

                if result.is_err() {
                    println!("Login failed: {}", error);
                    continue
                };

                let resp = result.unwrap();

                // Store the refresh token too
                let refresh_token = resp.clone()
                    .authentication_result.expect("Failed")
                    .refresh_token.expect("Missing refresh token");

                cache.set_encrypted("refresh_token", refresh_token.as_bytes().to_vec())
                    .expect("Failed to cache refresh token");

                id_token = Some(resp
                                .authentication_result.expect("No auth result")
                                .id_token.expect("No ID token"));

                break
            };

            id_token.unwrap()
        }
    }
}

pub async fn authenticated_client(cache: &FileCache) -> Result<S3Client, Error> {
    let id_provider_client = account::anonymous_identity_provider_client();
    let id_client = account::anonymous_identity_client();

    let refresh_token = ensure_refresh_token(&cache, &id_provider_client).await;
    let identity_id = cache.get("identity").context("Unable to retrieve user ID")?;
    let id_token = ensure_id_token(&cache, &id_provider_client, &refresh_token).await;

    let aws_creds = account::aws_credentials(&id_client, &identity_id, &id_token).await
        .context("Failed to fetch AWS credentials")?
        .credentials.expect("Missing credentials");

    let creds_provider = StaticProvider::new(
        aws_creds.access_key_id.expect("Missing access key"),
        aws_creds.secret_key.expect("Missing secret key"),
        Some(aws_creds.session_token.expect("Missing session token")),
        Some(aws_creds.expiration.expect("Missing expiration") as i64)
    );

    let request_dispatcher = HttpClient::new();
    let client = S3Client::new_with(
        request_dispatcher.context("Failed to make an HttpClient")?,
        creds_provider,
        Region::UsWest2
    );
    Ok(client)
}
