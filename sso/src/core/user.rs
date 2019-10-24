use crate::{
    client_msg::Get, AuditDiff, AuditDiffBuilder, AuditSubject, ClientActor, CoreError, CoreResult,
    Driver, DriverError, DriverResult, Service,
};
use actix::Addr;
use chrono::{DateTime, Utc};
use futures::{future, Future};
use libreauth::pass::HashBuilder;
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::fmt;
use uuid::Uuid;

/// User name maximum length.
pub const USER_NAME_MAX_LEN: usize = 100;

/// User locale maximum length.
pub const USER_LOCALE_MAX_LEN: usize = 10;

/// User default locale.
pub const USER_DEFAULT_LOCALE: &str = "en";

/// User timezone maximum length.
pub const USER_TIMEZONE_MAX_LEN: usize = 50;

/// User default timezone.
pub const USER_DEFAULT_TIMEZONE: &str = "Etc/UTC";

/// User password hash version.
///
/// Passed to libreauth hash builder.
pub const USER_PASSWORD_HASH_VERSION: usize = 1;

/// User password minimum length.
pub const USER_PASSWORD_MIN_LEN: usize = 8;

/// User password maximum length.
pub const USER_PASSWORD_MAX_LEN: usize = 128;

/// User.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub id: Uuid,
    pub is_enabled: bool,
    pub name: String,
    pub email: String,
    pub locale: String,
    pub timezone: String,
    pub password_allow_reset: bool,
    pub password_require_update: bool,
    #[serde(skip)]
    pub password_hash: Option<String>,
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "User {}", self.id)?;
        write!(f, "\n\tcreated_at {}", self.created_at)?;
        write!(f, "\n\tupdated_at {}", self.updated_at)?;
        write!(f, "\n\tis_enabled {}", self.is_enabled)?;
        write!(f, "\n\tname {}", self.name)?;
        write!(f, "\n\temail {}", self.email)?;
        write!(f, "\n\tlocale {}", self.locale)?;
        write!(f, "\n\ttimezone {}", self.timezone)?;
        write!(f, "\n\tpassword_allow_reset {}", self.password_allow_reset)?;
        write!(
            f,
            "\n\tpassword_require_update {}",
            self.password_require_update
        )
    }
}

impl AuditSubject for User {
    fn subject(&self) -> String {
        format!("{}", self.id)
    }
}

impl AuditDiff for User {
    fn diff(&self, previous: &Self) -> Value {
        AuditDiffBuilder::default()
            .compare("is_enabled", &self.is_enabled, &previous.is_enabled)
            .compare("name", &self.name, &previous.name)
            .compare("email", &self.email, &previous.email)
            .compare("locale", &self.locale, &previous.locale)
            .compare("timezone", &self.timezone, &previous.timezone)
            .compare(
                "password_allow_reset",
                &self.password_allow_reset,
                &previous.password_allow_reset,
            )
            .compare(
                "password_require_update",
                &self.password_require_update,
                &previous.password_require_update,
            )
            .into_value()
    }
}

/// User password metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct UserPasswordMeta {
    pub password_strength: Option<u8>,
    pub password_pwned: Option<bool>,
}

impl Default for UserPasswordMeta {
    fn default() -> Self {
        UserPasswordMeta {
            password_strength: None,
            password_pwned: None,
        }
    }
}

/// User list query.
#[derive(Debug)]
pub enum UserListQuery {
    /// Where ID greater than.
    IdGt(Uuid, i64),
    /// Where ID less than.
    IdLt(Uuid, i64),
    /// Where name greater than or equal.
    NameGe(String, i64, Option<Uuid>),
    /// Where name less than or equal.
    NameLe(String, i64, Option<Uuid>),
}

/// User list filter.
#[derive(Debug)]
pub struct UserListFilter {
    pub id: Option<Vec<Uuid>>,
    pub email: Option<Vec<String>>,
}

/// User list.
#[derive(Debug)]
pub struct UserList<'a> {
    pub query: &'a UserListQuery,
    pub filter: &'a UserListFilter,
}

/// User create.
#[derive(Debug)]
pub struct UserCreate {
    pub is_enabled: bool,
    pub name: String,
    pub email: String,
    pub locale: String,
    pub timezone: String,
    pub password_allow_reset: bool,
    pub password_require_update: bool,
    pub password_hash: Option<String>,
}

impl UserCreate {
    pub fn new<N, E>(is_enabled: bool, name: N, email: E) -> Self
    where
        N: Into<String>,
        E: Into<String>,
    {
        Self {
            is_enabled,
            name: name.into(),
            email: email.into(),
            locale: USER_DEFAULT_LOCALE.into(),
            timezone: USER_DEFAULT_TIMEZONE.into(),
            password_allow_reset: false,
            password_require_update: false,
            password_hash: None,
        }
    }

    pub fn locale<L>(mut self, locale: L) -> Self
    where
        L: Into<String>,
    {
        self.locale = locale.into();
        self
    }

    pub fn timezone<T>(mut self, timezone: T) -> Self
    where
        T: Into<String>,
    {
        self.timezone = timezone.into();
        self
    }

    pub fn with_password<P>(
        mut self,
        allow_reset: bool,
        require_update: bool,
        password: P,
    ) -> DriverResult<Self>
    where
        P: AsRef<str>,
    {
        self.password_allow_reset = allow_reset;
        self.password_require_update = require_update;
        self.password_hash = Some(hash_password(password.as_ref())?);
        Ok(self)
    }
}

/// User read.
#[derive(Debug)]
pub enum UserRead {
    Id(Uuid),
    Email(String),
}

/// User update.
#[derive(Debug)]
pub struct UserUpdate {
    pub is_enabled: Option<bool>,
    pub name: Option<String>,
    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub password_allow_reset: Option<bool>,
    pub password_require_update: Option<bool>,
}

/// User update 2.
///
/// This is split from `UserUpdate` to prevent `User::update_email` or
/// `User::update_password` functions being bypassed which could
/// allow an unhashed password to be saved to the database.
#[derive(Debug)]
pub struct UserUpdate2 {
    pub email: Option<String>,
    pub password_hash: Option<String>,
}

impl UserUpdate2 {
    /// Update user email.
    pub fn email<E>(email: E) -> Self
    where
        E: Into<String>,
    {
        Self {
            email: Some(email.into()),
            password_hash: None,
        }
    }

    /// Update user password.
    pub fn password<P>(password: P) -> DriverResult<Self>
    where
        P: AsRef<str>,
    {
        Ok(Self {
            email: None,
            password_hash: Some(hash_password(password.as_ref())?),
        })
    }
}

/// User token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserToken {
    pub user: User,
    pub access_token: String,
    pub access_token_expires: i64,
    pub refresh_token: String,
    pub refresh_token_expires: i64,
}

/// User access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTokenAccess {
    pub user: User,
    pub access_token: String,
    pub access_token_expires: i64,
}

/// User key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserKey {
    pub user: User,
    pub key: String,
}

impl User {
    /// Get nullable reference to user password hash.
    pub fn password_hash(&self) -> Option<&str> {
        self.password_hash.as_ref().map(|x| &**x)
    }

    /// Check if password string and password hash match, an error is returned if they do not match or the hash is none.
    /// Returns true if the hash version does not match the current hash version.
    pub fn password_check<P>(&self, password: P) -> DriverResult<bool>
    where
        P: AsRef<str>,
    {
        match self.password_hash() {
            Some(password_hash) => {
                let checker =
                    HashBuilder::from_phc(password_hash).map_err(DriverError::libreauth_pass)?;

                if checker.is_valid(password.as_ref()) {
                    Ok(checker.needs_update(Some(USER_PASSWORD_HASH_VERSION)))
                } else {
                    Err(DriverError::UserPasswordIncorrect)
                }
            }
            None => Err(DriverError::UserPasswordUndefined),
        }
    }

    /// Read user.
    pub fn read(
        driver: &dyn Driver,
        service_mask: Option<&Service>,
        read: &UserRead,
    ) -> CoreResult<User> {
        Self::read_opt(driver, service_mask, read)?.ok_or_else(|| CoreError::UserNotFound)
    }

    /// Read user (optional).
    pub fn read_opt(
        driver: &dyn Driver,
        _service_mask: Option<&Service>,
        read: &UserRead,
    ) -> CoreResult<Option<User>> {
        driver.user_read(read).map_err(CoreError::Driver)
    }

    /// Returns password strength and pwned checks.
    pub fn password_meta(
        enabled: bool,
        client: &Addr<ClientActor>,
        password: Option<&str>,
    ) -> impl Future<Item = UserPasswordMeta, Error = CoreError> {
        match password {
            Some(password) => {
                let password_strength = User::password_meta_strength(password).then(|r| match r {
                    Ok(entropy) => future::ok(Some(entropy.score)),
                    Err(err) => {
                        warn!("{}", err);
                        future::ok(None)
                    }
                });
                let password_pwned =
                    User::password_meta_pwned(enabled, client, password).then(|r| match r {
                        Ok(password_pwned) => future::ok(Some(password_pwned)),
                        Err(err) => {
                            warn!("{}", err);
                            future::ok(None)
                        }
                    });
                future::Either::A(password_strength.join(password_pwned).map(
                    |(password_strength, password_pwned)| UserPasswordMeta {
                        password_strength,
                        password_pwned,
                    },
                ))
            }
            None => future::Either::B(future::ok(UserPasswordMeta::default())),
        }
    }

    /// Returns password strength test performed by `zxcvbn`.
    /// <https://github.com/shssoichiro/zxcvbn-rs>
    fn password_meta_strength(
        password: &str,
    ) -> impl Future<Item = zxcvbn::Entropy, Error = CoreError> {
        // TODO(fix): Fix "Zxcvbn cannot evaluate a blank password" warning.
        future::result(zxcvbn::zxcvbn(password, &[]).map_err(CoreError::Zxcvbn))
    }

    /// Returns true if password is present in `Pwned Passwords` index, else false.
    /// <https://haveibeenpwned.com/Passwords>
    fn password_meta_pwned(
        enabled: bool,
        client: &Addr<ClientActor>,
        password: &str,
    ) -> impl Future<Item = bool, Error = CoreError> {
        if enabled {
            // Make request to API using first 5 characters of SHA1 password hash.
            let mut hash = Sha1::new();
            hash.input(password);
            let hash = format!("{:X}", hash.result());
            let route = format!("/range/{:.5}", hash);

            future::Either::A(
                // Make API request.
                client
                    .send(Get::new("https://api.pwnedpasswords.com", route))
                    .map_err(CoreError::ActixMailbox)
                    .and_then(|res| res.map_err(CoreError::Client))
                    // Compare suffix of hash to lines to determine if password is pwned.
                    .and_then(move |text| {
                        for line in text.lines() {
                            if hash[5..] == line[..35] {
                                return Ok(true);
                            }
                        }
                        Ok(false)
                    }),
            )
        } else {
            future::Either::B(future::err(CoreError::PwnedPasswordsDisabled))
        }
    }
}

/// Hash password string.
/// <https://github.com/breard-r/libreauth>
fn hash_password(password: &str) -> DriverResult<String> {
    let hasher = HashBuilder::new()
        .version(USER_PASSWORD_HASH_VERSION)
        .min_len(USER_PASSWORD_MIN_LEN)
        .max_len(USER_PASSWORD_MAX_LEN)
        .finalize()
        .map_err(DriverError::libreauth_pass)?;
    hasher.hash(password).map_err(DriverError::libreauth_pass)
}
