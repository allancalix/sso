use crate::core::{Audit, Error, Key, Service, User};
use crate::driver;
use chrono::Utc;
use serde::ser::Serialize;
use serde_json::Value;
use time::Duration;

/// Audit meta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditMeta {
    pub user_agent: String,
    pub remote: String,
    // TODO(refactor): Use X-Real-IP header?
    pub forwarded_for: Option<String>,
}

/// Audit paths.
pub enum AuditPath {
    Login(AuditMessageObject<AuditMessage>),
    LoginError(AuditMessageObject<AuditMessage>),
    ResetPassword(AuditMessageObject<AuditMessage>),
    ResetPasswordError(AuditMessageObject<AuditMessage>),
    ResetPasswordConfirm(AuditMessageObject<AuditMessage>),
    ResetPasswordConfirmError(AuditMessageObject<AuditMessage>),
    UpdateEmail(AuditMessageObject<AuditMessage>),
    UpdateEmailError(AuditMessageObject<AuditMessage>),
    UpdateEmailRevoke(AuditMessageObject<AuditMessage>),
    UpdateEmailRevokeError(AuditMessageObject<AuditMessage>),
    UpdatePassword(AuditMessageObject<AuditMessage>),
    UpdatePasswordError(AuditMessageObject<AuditMessage>),
    UpdatePasswordRevoke(AuditMessageObject<AuditMessage>),
    UpdatePasswordRevokeError(AuditMessageObject<AuditMessage>),
}

impl AuditPath {
    /// Return string representation and JSON value of key.
    pub fn to_path_data(&self) -> (String, Value) {
        match self {
            AuditPath::Login(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/login".to_owned(), value)
            }
            AuditPath::LoginError(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/error/login".to_owned(), value)
            }
            AuditPath::ResetPassword(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/reset_password".to_owned(), value)
            }
            AuditPath::ResetPasswordError(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/error/reset_password".to_owned(), value)
            }
            AuditPath::ResetPasswordConfirm(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/reset_password_confirm".to_owned(), value)
            }
            AuditPath::ResetPasswordConfirmError(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/error/reset_password_confirm".to_owned(), value)
            }
            AuditPath::UpdateEmail(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/update_email".to_owned(), value)
            }
            AuditPath::UpdateEmailError(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/error/update_email".to_owned(), value)
            }
            AuditPath::UpdateEmailRevoke(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/update_email_revoke".to_owned(), value)
            }
            AuditPath::UpdateEmailRevokeError(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/error/update_email_revoke".to_owned(), value)
            }
            AuditPath::UpdatePassword(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/update_password".to_owned(), value)
            }
            AuditPath::UpdatePasswordError(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/error/update_password".to_owned(), value)
            }
            AuditPath::UpdatePasswordRevoke(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/update_password_revoke".to_owned(), value)
            }
            AuditPath::UpdatePasswordRevokeError(message) => {
                let value = serde_json::to_value(message).unwrap();
                ("ark_auth/error/update_password_revoke".to_owned(), value)
            }
        }
    }
}

/// Audit messages.
#[derive(Debug, Serialize)]
pub enum AuditMessage {
    UserNotFoundOrDisabled,
    KeyNotFoundOrDisabled,
    PasswordNotSetOrIncorrect,
    Login,
    ResetPassword,
    TokenInvalidOrExpired,
    CsrfNotFoundOrUsed,
    ResetPasswordConfirm,
    UpdateEmail,
    UpdateEmailRevoke,
    UpdatePassword,
    UpdatePasswordRevoke,
}

/// Audit data message container.
#[derive(Debug, Serialize)]
pub struct AuditMessageObject<T: Serialize> {
    pub message: T,
}

impl From<AuditMessage> for AuditMessageObject<AuditMessage> {
    fn from(message: AuditMessage) -> AuditMessageObject<AuditMessage> {
        AuditMessageObject { message }
    }
}

/// Audit log builder pattern.
pub struct AuditBuilder {
    meta: AuditMeta,
    key: Option<String>,
    service: Option<String>,
    user: Option<String>,
    user_key: Option<String>,
}

impl AuditBuilder {
    /// Create a new audit log builder with required parameters.
    pub fn new(meta: AuditMeta) -> Self {
        AuditBuilder {
            meta,
            key: None,
            service: None,
            user: None,
            user_key: None,
        }
    }

    pub fn set_key(&mut self, key: Option<&Key>) -> &mut Self {
        self.key = key.map(|x| x.id.to_owned());
        self
    }

    pub fn set_service(&mut self, service: Option<&Service>) -> &mut Self {
        self.service = service.map(|x| x.id.to_owned());
        self
    }

    pub fn set_user(&mut self, user: Option<&User>) -> &mut Self {
        self.user = user.map(|x| x.id.to_owned());
        self
    }

    pub fn set_user_key(&mut self, key: Option<&Key>) -> &mut Self {
        self.user_key = key.map(|x| x.id.to_owned());
        self
    }

    /// Create audit log from internal parameters.
    /// In case of error, log as warning and return None.
    pub fn create(&self, driver: &driver::Driver, path: AuditPath) -> Option<Audit> {
        match create(
            driver,
            &self.meta,
            path,
            self.key.as_ref().map(|x| &**x),
            self.service.as_ref().map(|x| &**x),
            self.user.as_ref().map(|x| &**x),
            self.user_key.as_ref().map(|x| &**x),
        ) {
            Ok(audit) => Some(audit),
            Err(err) => {
                warn!("{}", err);
                None
            }
        }
    }
}

/// Create one audit log.
pub fn create(
    driver: &driver::Driver,
    meta: &AuditMeta,
    path: AuditPath,
    key: Option<&str>,
    service: Option<&str>,
    user: Option<&str>,
    user_key: Option<&str>,
) -> Result<Audit, Error> {
    let (path, data) = path.to_path_data();
    driver
        .audit_create(meta, &path, &data, key, service, user, user_key)
        .map_err(Error::Driver)
}

/// Delete many audit logs older than days.
pub fn delete_by_age(driver: &driver::Driver, days: usize) -> Result<usize, Error> {
    let days: i64 = 0 - days as i64;
    let created_at = Utc::now() + Duration::days(days);
    match driver.audit_delete_by_created_at(&created_at) {
        Ok(count) => Ok(count),
        Err(err) => {
            warn!("{}", Error::Driver(err));
            Ok(0)
        }
    }
}