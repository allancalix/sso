//! # Pattern functions.
use crate::{
    AuditBuilder, Driver, DriverError, DriverResult, Jwt, KeyRead, KeyType, KeyWithValue, Service,
    ServiceRead, User, UserPasswordMeta, UserRead,
};
use libreauth::oath::TOTPBuilder;
use reqwest::Client;
use sha1::{Digest, Sha1};
use url::Url;
use uuid::Uuid;

// TODO(refactor2): Improve usability, composability of pattern functions.

/// User header.
#[derive(Debug, Clone)]
pub enum HeaderAuth {
    Key(String),
    Token(String),
}

impl HeaderAuth {
    /// Parse header value, returns key value.
    /// Formats: `$KEY`, `key $KEY`, `Bearer $KEY`
    pub fn parse_key(value: &str) -> Option<String> {
        let value = value.to_owned();
        if value.starts_with("key ") || value.starts_with("Bearer ") {
            let parts: Vec<&str> = value.split_whitespace().collect();
            if parts.len() > 1 {
                let value = parts[1].trim().to_owned();
                Some(value)
            } else {
                None
            }
        } else {
            Some(value)
        }
    }

    /// Parse header value, extract key or token.
    /// Formats: `$KEY`, `key $KEY`, `token $TOKEN`
    pub fn parse(value: &str) -> Option<Self> {
        let mut type_value = value.split_whitespace();
        let type_ = match type_value.next() {
            Some(type_) => type_,
            None => return None,
        };

        Some(match type_value.next() {
            Some(value) => match type_ {
                "token" => Self::Token(value.to_owned()),
                "key" => Self::Key(value.to_owned()),
                _ => Self::Key(value.to_owned()),
            },
            None => Self::Key(type_.to_owned()),
        })
    }
}

/// Verify TOTP code using key.
pub fn totp_verify(key: &str, code: &str) -> DriverResult<()> {
    let totp = TOTPBuilder::new()
        .base32_key(key)
        .finalize()
        .map_err::<DriverError, _>(Into::into)?;

    if !totp.is_valid(&code) {
        Err(DriverError::TotpInvalid)
    } else {
        Ok(())
    }
}

/// Authenticate root key.
pub fn key_root_authenticate(
    driver: &dyn Driver,
    audit: &mut AuditBuilder,
    key_value: Option<&String>,
) -> DriverResult<()> {
    match key_value {
        Some(key_value) => {
            let read = KeyRead::RootValue(key_value.to_owned());
            driver
                .key_read(&read, None)?
                .ok_or_else(|| DriverError::KeyNotFound)
                .map(|key| {
                    audit.key(Some(&key));
                    key
                })
                .map(|_key| ())
        }
        None => Err(DriverError::KeyUndefined),
    }
}

/// Authenticate service key.
///
/// If audit meta user is some, this function will also verify
/// the user key or token to authenticate this request.
pub fn key_service_authenticate(
    driver: &dyn Driver,
    audit: &mut AuditBuilder,
    key_value: Option<&String>,
) -> DriverResult<Service> {
    let service = key_service_authenticate_try(driver, audit, key_value)?;
    check_audit_user(driver, audit, &service)?;
    Ok(service)
}

/// Authenticate service or root key.
///
/// If audit meta user is some, this function will also verify
/// the user key or token to authenticate this request.
pub fn key_authenticate(
    driver: &dyn Driver,
    audit: &mut AuditBuilder,
    key_value: Option<&String>,
) -> DriverResult<Option<Service>> {
    let key_value_1 = key_value.to_owned();

    let service = key_service_authenticate_try(driver, audit, key_value)
        .and_then(|service| {
            check_audit_user(driver, audit, &service)?;
            Ok(service)
        })
        .map(Some)
        .or_else(|_err| key_root_authenticate(driver, audit, key_value_1).map(|_| None))?;
    Ok(service)
}

fn key_service_authenticate_try(
    driver: &dyn Driver,
    audit: &mut AuditBuilder,
    key_value: Option<&String>,
) -> DriverResult<Service> {
    match key_value {
        Some(key_value) => driver
            .key_read(&KeyRead::ServiceValue(key_value.to_owned()), None)?
            .ok_or_else(|| DriverError::KeyNotFound)
            .and_then(|key| {
                audit.key(Some(&key));
                key.service_id
                    .ok_or_else(|| DriverError::KeyServiceUndefined)
            })
            .and_then(|service_id| key_service_authenticate_inner(driver, audit, service_id)),
        None => Err(DriverError::KeyUndefined),
    }
}

fn key_service_authenticate_inner(
    driver: &dyn Driver,
    audit: &mut AuditBuilder,
    service_id: Uuid,
) -> DriverResult<Service> {
    let service = driver
        .service_read(&ServiceRead::new(service_id), None)?
        .ok_or_else(|| DriverError::ServiceNotFound)?
        .check()?;
    audit.service(Some(&service));
    Ok(service)
}

fn check_audit_user(
    driver: &dyn Driver,
    audit: &mut AuditBuilder,
    service: &Service,
) -> DriverResult<()> {
    let user = audit.meta().user().cloned();
    match user {
        // TODO(refactor2): Duplicate authentication code with api module, refactor.
        Some(user) => match user {
            HeaderAuth::Key(key_value) => {
                // Key verify requires key key type.
                let key =
                    key_read_user_value_checked(driver, &service, audit, key_value, KeyType::Key)?;
                user_read_id_checked(driver, Some(&service), audit, key.user_id.unwrap())?;
                Ok(())
            }
            HeaderAuth::Token(token) => {
                // Unsafely decode token to get user identifier, used to read key for safe token decode.
                let (user_id, _) = Jwt::decode_unsafe(&token, service.id)?;

                // Token verify requires token key type.
                let user = user_read_id_checked(driver, Some(&service), audit, user_id)?;
                let key = key_read_user_checked(driver, &service, audit, &user, KeyType::Token)?;

                // Safely decode token with user key.
                Jwt::decode_access_token(&service, &user, &key, &token)?;
                Ok(())
            }
        },
        None => Ok(()),
    }
}

/// Read user by ID.
/// Checks user is enabled, returns bad request if disabled.
pub fn user_read_id_checked(
    driver: &dyn Driver,
    _service_mask: Option<&Service>,
    audit: &mut AuditBuilder,
    id: Uuid,
) -> DriverResult<User> {
    let read = UserRead::Id(id);
    let user = driver
        .user_read(&read)?
        .ok_or_else(|| DriverError::UserNotFound)?;
    audit.user(Some(&user));
    if !user.is_enabled {
        return Err(DriverError::UserDisabled);
    }
    Ok(user)
}

/// Unchecked read user by ID.
/// Does not check user is enabled.
pub fn user_read_id_unchecked(
    driver: &dyn Driver,
    _service_mask: Option<&Service>,
    audit: &mut AuditBuilder,
    id: Uuid,
) -> DriverResult<User> {
    let read = UserRead::Id(id);
    let user = driver
        .user_read(&read)?
        .ok_or_else(|| DriverError::UserNotFound)?;
    audit.user(Some(&user));
    Ok(user)
}

/// Read user by email address.
/// Also checks user is enabled, returns bad request if disabled.
pub fn user_read_email_checked(
    driver: &dyn Driver,
    _service_mask: Option<&Service>,
    audit: &mut AuditBuilder,
    email: &str,
) -> DriverResult<User> {
    let read = UserRead::Email(email.to_owned());
    let user = driver
        .user_read(&read)?
        .ok_or_else(|| DriverError::UserNotFound)?;
    audit.user(Some(&user));
    if !user.is_enabled {
        return Err(DriverError::UserDisabled);
    }
    Ok(user)
}

/// Read key by user reference and key type.
/// Also checks key is enabled and not revoked, returns bad request if disabled.
pub fn key_read_user_checked(
    driver: &dyn Driver,
    service: &Service,
    audit: &mut AuditBuilder,
    user: &User,
    key_type: KeyType,
) -> DriverResult<KeyWithValue> {
    let key = driver
        .key_read(
            &KeyRead::user_id(service.id, user.id, true, false, key_type),
            None,
        )?
        .ok_or_else(|| DriverError::KeyNotFound)?;
    audit.user_key(Some(&key));
    if !key.is_enabled {
        Err(DriverError::KeyDisabled)
    } else if key.is_revoked {
        Err(DriverError::KeyRevoked)
    } else {
        Ok(key)
    }
}

/// Unchecked read key by user reference.
/// Does not check key is enabled or not revoked.
pub fn key_read_user_unchecked(
    driver: &dyn Driver,
    service: &Service,
    audit: &mut AuditBuilder,
    user: &User,
    key_type: KeyType,
) -> DriverResult<KeyWithValue> {
    let key = driver
        .key_read(
            &KeyRead::user_id(service.id, user.id, true, false, key_type),
            None,
        )?
        .ok_or_else(|| DriverError::KeyNotFound)?;
    audit.user_key(Some(&key));
    Ok(key)
}

/// Read key by user value.
/// Also checks key is enabled and not revoked, returns bad request if disabled.
pub fn key_read_user_value_checked<K>(
    driver: &dyn Driver,
    service: &Service,
    audit: &mut AuditBuilder,
    key: K,
    key_type: KeyType,
) -> DriverResult<KeyWithValue>
where
    K: Into<String>,
{
    let key = driver
        .key_read(
            &KeyRead::user_value(service.id, key.into(), true, false, key_type),
            None,
        )?
        .ok_or_else(|| DriverError::KeyNotFound)?;
    audit.user_key(Some(&key));
    if !key.is_enabled {
        Err(DriverError::KeyDisabled)
    } else if key.is_revoked {
        Err(DriverError::KeyRevoked)
    } else {
        Ok(key)
    }
}

/// Unchecked read key by user value.
/// Does not check key is enabled and not revoked.
pub fn key_read_user_value_unchecked(
    driver: &dyn Driver,
    service: &Service,
    audit: &mut AuditBuilder,
    key: &str,
    key_type: KeyType,
) -> DriverResult<KeyWithValue> {
    let key = driver
        .key_read(
            &KeyRead::user_value(service.id, key, true, false, key_type),
            None,
        )?
        .ok_or_else(|| DriverError::KeyNotFound)?;
    audit.user_key(Some(&key));
    Ok(key)
}

/// Password strength and pwned checks.
///
/// If password is empty, returns 0 for strength and true for pwned.
/// If password is none, returns none for strength and pwned.
pub fn password_meta(
    client: &Client,
    enabled: bool,
    password: Option<String>,
) -> DriverResult<UserPasswordMeta> {
    match password.as_ref().map(|x| &**x) {
        Some("") => Ok(UserPasswordMeta::invalid()),
        Some(password) => {
            let password_strength = match password_meta_strength(password) {
                Ok(entropy) => Some(entropy.score()),
                Err(err) => {
                    warn!("{}", err);
                    None
                }
            };
            let password_pwned = match password_meta_pwned(client, enabled, password) {
                Ok(password_pwned) => Some(password_pwned),
                Err(err) => {
                    warn!("{}", err);
                    None
                }
            };
            Ok(UserPasswordMeta {
                password_strength,
                password_pwned,
            })
        }
        None => Ok(UserPasswordMeta::default()),
    }
}

/// Returns password strength test performed by `zxcvbn`.
/// <https://github.com/shssoichiro/zxcvbn-rs>
fn password_meta_strength(password: &str) -> DriverResult<zxcvbn::Entropy> {
    zxcvbn::zxcvbn(password, &[]).map_err(DriverError::Zxcvbn)
}

/// Returns true if password is present in `Pwned Passwords` index, else false.
/// <https://haveibeenpwned.com/Passwords>
fn password_meta_pwned(client: &Client, enabled: bool, password: &str) -> DriverResult<bool> {
    if enabled {
        // Make request to API using first 5 characters of SHA1 password hash.
        let mut hash = Sha1::new();
        hash.input(password);
        let hash = format!("{:X}", hash.result());
        let url = format!("https://api.pwnedpasswords.com/range/{:.5}", hash);

        match Url::parse(&url).map_err(DriverError::UrlParse) {
            Ok(url) => {
                client
                    .get(url)
                    .send()
                    .map_err(DriverError::Reqwest)
                    .and_then(|res| res.error_for_status().map_err(DriverError::Reqwest))
                    .and_then(|mut res| res.text().map_err(DriverError::Reqwest))
                    .and_then(move |text| {
                        // Compare suffix of hash to lines to determine if password is pwned.
                        for line in text.lines() {
                            if hash[5..] == line[..35] {
                                return Ok(true);
                            }
                        }
                        Ok(false)
                    })
            }
            Err(e) => Err(e),
        }
    } else {
        Err(DriverError::PwnedPasswordsDisabled)
    }
}
