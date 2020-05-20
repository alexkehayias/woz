use std::collections::HashMap;
use rusoto_core::{RusotoError, Region};
use rusoto_core::credential::{StaticProvider, AwsCredentials};
use rusoto_core::request::HttpClient;
use rusoto_cognito_identity::*;
use rusoto_cognito_idp::CognitoIdentityProvider;
use rusoto_cognito_idp::*;

use crate::cache::FileCache;
use crate::config::*;


/// By default, CognitoIdentityProviderClient::new will attempt to use the aws
/// credentials on the user's machine (e.g. ~/.aws/credentials or
/// environment variables). If the user doesn't have any credentials,
/// any calls with this client will fail. The client returned by this
/// function uses anonymous credentials which prevents this issue.
pub fn anonymous_identity_provider_client() -> CognitoIdentityProviderClient {
    CognitoIdentityProviderClient::new_with(
        HttpClient::new().expect("Failed to create HTTP client"),
        StaticProvider::from(AwsCredentials::default()),
        Region::UsWest2
    )
}

/// By default, CognitoIdentityClient::new will attempt to use the aws
/// credentials on the user's machine (e.g. ~/.aws/credentials or
/// environment variables). If the user doesn't have any credentials,
/// any calls with this client will fail. The client returned by this
/// function uses anonymous credentials which prevents this issue.
pub fn anonymous_identity_client() -> CognitoIdentityClient {
    CognitoIdentityClient::new_with(
        HttpClient::new().expect("Failed to create HTTP client"),
        StaticProvider::from(AwsCredentials::default()),
        Region::UsWest2
    )
}

pub async fn signup(client: &CognitoIdentityProviderClient,
                    email: &str,
                    username: &str,
                    password: &str)
                    -> Result<SignUpResponse, RusotoError<SignUpError>> {
    let mut request = SignUpRequest::default();
    request.username = username.to_owned();
    request.password = password.to_owned();
    request.client_id = String::from(CLIENT_ID);
    let email = AttributeType {
        name: String::from("email"),
        value: Some(email.to_owned())
    };
    request.user_attributes = Some(vec![email]);
    client.sign_up(request).await
}

pub async fn login(client: &CognitoIdentityProviderClient, username: &str, password: &str) -> Result<InitiateAuthResponse, RusotoError<InitiateAuthError>> {
    let mut request = InitiateAuthRequest::default();
    request.auth_flow = String::from("USER_PASSWORD_AUTH");
    let mut auth_params = HashMap::new();
    auth_params.insert(String::from("USERNAME"), username.to_owned());
    auth_params.insert(String::from("PASSWORD"), password.to_owned());
    request.client_id = String::from(CLIENT_ID);
    request.auth_parameters = Some(auth_params);
    client.initiate_auth(request).await
}

pub async fn refresh_auth(client: &CognitoIdentityProviderClient, refresh_token: &str)
                          -> Result<InitiateAuthResponse, RusotoError<InitiateAuthError>> {
    let mut auth_params = HashMap::new();
    auth_params.insert(String::from("REFRESH_TOKEN"), refresh_token.to_string());

    let req = InitiateAuthRequest {
        client_id: CLIENT_ID.to_string(),
        auth_flow: String::from("REFRESH_TOKEN_AUTH"),
        auth_parameters: Some(auth_params),
        ..Default::default()
    };

    client.initiate_auth(req).await
}

pub async fn identity_id(client: &CognitoIdentityClient, id_token: &str)
                         -> Result<GetIdResponse, RusotoError<GetIdError>> {
    let mut logins = HashMap::new();
    logins.insert(USER_POOL_URL.to_string(), id_token.to_owned());

    let mut req = GetIdInput::default();
    req.identity_pool_id = IDENTITY_POOL_ID.to_string();
    req.logins = Some(logins);
    client.get_id(req).await
}

/// After a user has been signed up via `signup`, set up their account
/// by generating and storing an identity and refresh token. Result
/// will fail if the user has not confirmed their email address.
pub async fn setup(id_provider_client: &CognitoIdentityProviderClient,
                   id_client: &CognitoIdentityClient,
                   cache: &FileCache,
                   username: &str,
                   password: &str) -> Result<(), InitiateAuthError> {
    match login(&id_provider_client, username, password).await {
        Ok(resp) => {
            let auth_result = resp.authentication_result
                .expect("No auth result");

            // Store the refresh token
            let refresh_token = auth_result.refresh_token
                .expect("No access token found");

            cache.set_encrypted("refresh_token", refresh_token.as_bytes().to_vec())
                .expect("Failed to set refresh token in cache");

            // Store the identity ID
            let id_token = auth_result.id_token
                .expect("No ID token found");

            let identity_id = identity_id(&id_client, &id_token).await
                .expect("Getting identity ID didn't work")
                .identity_id.expect("No identity ID");

            cache.set("identity", identity_id.as_bytes().to_vec())
                .expect("Failed to set identity ID in cache");

            Ok(())
        },
        Err(error) => {
            match error {
                RusotoError::Service(e) => Err(e),
                _ => panic!("Login failed: {}", error)
            }
        }
    }
}

pub type AWSCredentialsResponse = Result<GetCredentialsForIdentityResponse,
                                         RusotoError<GetCredentialsForIdentityError>>;
pub async fn aws_credentials(client: &CognitoIdentityClient, identity_id: &str, id_token: &str)
                             ->  AWSCredentialsResponse {
    let mut logins = HashMap::new();
    logins.insert(USER_POOL_URL.to_string(), id_token.to_owned());

    let mut req = GetCredentialsForIdentityInput::default();
    req.identity_id = identity_id.to_owned();
    req.logins = Some(logins);
    client.get_credentials_for_identity(req).await
}


#[cfg(test)]
mod account_tests {
    use super::*;
    use rusoto_mock::{MockRequestDispatcher, MockResponseReader, ReadMockResponse};

    fn mock_id_provider_client(resp: &String) -> CognitoIdentityProviderClient {
        CognitoIdentityProviderClient::new_with(
            MockRequestDispatcher::default().with_body(resp),
            StaticProvider::from(AwsCredentials::default()),
            Region::UsWest2
        )
    }

    fn mock_id_client(resp: &String) -> CognitoIdentityClient {
        CognitoIdentityClient::new_with(
            MockRequestDispatcher::default().with_body(resp),
            StaticProvider::from(AwsCredentials::default()),
            Region::UsWest2
        )
    }

    #[tokio::test]
    async fn test_signup() {
        let resp = MockResponseReader::read_response(&"test_data", &"signup_response_success.json");
        let client = mock_id_provider_client(&resp);

        signup(&client, &"test@example.com", &"test_user", &"test1234").await.unwrap();
    }


    #[tokio::test]
    async fn test_login() {
        let resp = MockResponseReader::read_response(&"test_data", &"login_response_success.json");
        let client = mock_id_provider_client(&resp);

        login(&client, &"test_user", &"test1234").await.unwrap();
    }

    #[tokio::test]
    async fn test_refresh_auth() {
        let resp = MockResponseReader::read_response(&"test_data", &"initiate_auth_response_success.json");
        let client = mock_id_provider_client(&resp);

        let token = refresh_auth(&client, &"old_token_123")
            .await.unwrap()
            .authentication_result.unwrap()
            .id_token.unwrap();

        assert_eq!(token, "id_token_123")
    }

    #[tokio::test]
    async fn test_identity_id() {
        let resp = MockResponseReader::read_response(&"test_data", &"get_id_response_success.json");
        let client = mock_id_client(&resp);

        let id = identity_id(&client, &"id_token_123")
            .await.unwrap()
            .identity_id.unwrap();

        assert_eq!(id, "id_123");
    }

    #[tokio::test]
    async fn test_setup() {
        let id_provider_resp = MockResponseReader::read_response(&"test_data", &"initiate_auth_response_success.json");
        let id_provider_client = mock_id_provider_client(&id_provider_resp);

        let id_resp = MockResponseReader::read_response(&"test_data", &"get_id_response_success.json");
        let id_client = mock_id_client(&id_resp);

        let encryption_key = FileCache::make_key(&"test_password", &"test_salt");
        let tmp_dir = std::env::temp_dir();
        let cache = FileCache::new(encryption_key, tmp_dir);

        setup(
            &id_provider_client,
            &id_client,
            &cache,
            &"test_username",
            &"test_password"
        ).await.unwrap();

        assert_eq!(
            cache.get_encrypted(&"refresh_token").unwrap(),
            "refresh_token_123"
        );

        assert_eq!(
            cache.get(&"identity").unwrap(),
            "id_123"
        );
    }
}
