//! The cli used to interact with the woz service.
use failure::Error;
use failure::ResultExt;
use rusoto_core::Region;
use rusoto_core::request::HttpClient;
use rusoto_credential::StaticProvider;
use rusoto_cognito_identity::*;
use rusoto_cognito_idp::*;
use rusoto_s3::*;

use crate::cache::FileCache;
use crate::prompt;
use crate::account;


/// Attempt to get a refresh_token token, prompting the user to log in if
/// refresh token is expired and stores it locally.
async fn ensure_refresh_token(cache: &FileCache, client: &CognitoIdentityProviderClient) -> String {
    let token = match cache.get_encrypted("refresh_token") {
        Ok(token) => token,
        Err(_) => {
            let creds = prompt::login();
            let resp = account::login(&client, creds.username, creds.password).await;

            // TODO recursion not allowed in async functions so need
            // to replace this to loop over account login until
            // successful
            // if resp.is_err() {
            //     return ensure_refresh_token(cache, client).await
            // };

            let token = resp.unwrap().authentication_result
                .expect("Failed")
                .refresh_token
                .expect("Missing refresh token");

            cache.set_encrypted("refresh_token", token.as_bytes().to_vec())
                .expect("Failed to cache refresh token");

            token
        }
    };
    token
}

pub async fn authenticated_client(cache: &FileCache) -> Result<S3Client, Error> {
    let id_provider_client = CognitoIdentityProviderClient::new(Region::UsWest2);
    let id_client = CognitoIdentityClient::new(Region::UsWest2);

    let refresh_token = ensure_refresh_token(&cache, &id_provider_client).await;
    let identity_id = cache.get("identity").context("Unable to retrieve user ID")?;

    let id_token = account::refresh_auth(&id_provider_client, &refresh_token).await
        // TODO fix this to work with async
        // .or_else(|err| {
        //     // TODO only login if the failure is due to an auth error
        //     println!("Getting refresh token failed: {}", err);
        //     let creds = prompt::login();
        //     account::login(&id_provider_client, creds.username, creds.password).await
        //         .or_else(|e| {
        //             println!("Login failed: {}", e);
        //             Err(e)
        //         })
        //         .and_then(|resp| {
        //             let token = resp.clone()
        //                 .authentication_result.expect("Failed")
        //                 .refresh_token.expect("Missing refresh token");
        //             cache.set_encrypted("refresh_token", token.as_bytes().to_vec())
        //                 .expect("Failed to cache refresh token");
        //             Ok(resp)})
        // })
        .context("Failed to get id token")?
        .authentication_result.expect("No auth result")
        .id_token.expect("No access token");

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
