mod audit;
mod auth_key;
mod auth_local;
mod auth_token;
mod guide;
mod key;
mod service;
mod user;

use ark_auth::client::{Client, ClientOptions, SyncClient};
pub use ark_auth::client::{Error, RequestError};
use ark_auth::core::{Key, Service, User, UserKey, UserToken, UserTokenPartial};
use ark_auth::server::api::AuthOauth2UrlResponse;
pub use ark_auth::server::api::{
    AuditCreateBody, AuditListQuery, KeyListQuery, ServiceListQuery, UserListQuery,
};
use chrono::Utc;
pub use serde_json::Value;

pub const INVALID_EMAIL: &str = "invalid-email";
pub const INVALID_PASSWORD: &str = "guests";
pub const INVALID_UUID: &str = "5a044d9035334e95a60ac0338904d37c";
pub const INVALID_SERVICE_KEY: &str = "invalid-service-key";
pub const USER_NAME: &str = "user-name";
pub const USER_PASSWORD: &str = "user-name";
pub const KEY_NAME: &str = "key-name";

fn env_test_ark_auth_url() -> String {
    std::env::var("TEST_ARK_AUTH_URL")
        .expect("TEST_ARK_AUTH_URL is undefined, integration test disabled")
}

fn env_test_ark_auth_key() -> String {
    std::env::var("TEST_ARK_AUTH_KEY")
        .expect("TEST_ARK_AUTH_KEY is undefined, integration test disabled")
}

pub fn client_create() -> SyncClient {
    let url = env_test_ark_auth_url();
    let key = env_test_ark_auth_key();
    let options = ClientOptions::new(&url, &key).unwrap();
    SyncClient::new(options)
}

pub fn email_create() -> String {
    let random = uuid::Uuid::new_v4().to_simple().to_string();
    format!("{}@test.com", random)
}

pub fn service_key_create(client: &SyncClient) -> (Service, Key) {
    let create_service = client
        .service_create(true, "test", "http://localhost")
        .unwrap();
    let create_key = client
        .key_create(true, "test", Some(create_service.data.id.to_owned()), None)
        .unwrap();
    (create_service.data, create_key.data)
}

pub fn user_create(
    client: &SyncClient,
    is_enabled: bool,
    name: &str,
    email: &str,
    password: Option<&str>,
) -> User {
    let before = Utc::now();
    let create = client
        .user_create(is_enabled, name, email, password.map(|x| x.to_owned()))
        .unwrap();
    let user = create.data;
    assert!(user.created_at.gt(&before));
    assert!(user.updated_at.gt(&before));
    assert!(!user.id.is_empty());
    assert_eq!(user.is_enabled, is_enabled);
    assert_eq!(user.name, name);
    assert_eq!(user.email, email);
    assert!(user.password_hash.is_none());
    user
}

pub fn user_key_create(
    client: &SyncClient,
    name: &str,
    service_id: &str,
    user_id: &str,
) -> UserKey {
    let create = client
        .key_create(true, name, None, Some(user_id.to_owned()))
        .unwrap();
    let key = create.data;
    assert_eq!(key.name, name);
    assert_eq!(key.service_id.unwrap(), service_id);
    assert_eq!(key.user_id.unwrap(), user_id);
    UserKey {
        user_id: user_id.to_owned(),
        key: key.value.to_owned(),
    }
}

pub fn user_key_verify(client: &SyncClient, key: &UserKey) -> UserKey {
    let verify = client.auth_key_verify(&key.key).unwrap();
    let user_key = verify.data;
    assert_eq!(user_key.user_id, key.user_id);
    assert_eq!(user_key.key, key.key);
    user_key
}

pub fn user_key_verify_bad_request(client: &SyncClient, key: &str) {
    let err = client.auth_key_verify(key).unwrap_err();
    assert_eq!(err, Error::Request(RequestError::BadRequest));
}

pub fn user_token_verify(client: &SyncClient, token: &UserToken) -> UserTokenPartial {
    let verify = client.auth_token_verify(&token.access_token).unwrap();
    let user_token = verify.data;
    assert_eq!(user_token.user_id, token.user_id);
    assert_eq!(user_token.access_token, token.access_token);
    assert_eq!(user_token.access_token_expires, token.access_token_expires);
    user_token
}

pub fn user_token_refresh(client: &SyncClient, token: &UserToken) -> UserToken {
    std::thread::sleep(std::time::Duration::from_secs(1));
    let refresh = client.auth_token_refresh(&token.refresh_token).unwrap();
    let user_token = refresh.data;
    assert_eq!(user_token.user_id, token.user_id);
    assert_ne!(user_token.access_token, token.access_token);
    assert_ne!(user_token.access_token_expires, token.access_token_expires);
    assert_ne!(user_token.access_token, token.access_token);
    assert_ne!(user_token.access_token_expires, token.access_token_expires);
    user_token
}

pub fn auth_local_login(
    client: &SyncClient,
    user_id: &str,
    email: &str,
    password: &str,
) -> UserToken {
    let login = client.auth_local_login(email, password).unwrap();
    let user_token = login.data;
    assert_eq!(user_token.user_id, user_id);
    user_token
}

pub fn auth_microsoft_oauth2_request(client: &SyncClient) -> AuthOauth2UrlResponse {
    let response = client.auth_microsoft_oauth2_request().unwrap();
    assert!(!response.url.is_empty());
    response
}
