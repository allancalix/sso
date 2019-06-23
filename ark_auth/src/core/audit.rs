use crate::core::{Audit, AuditMeta, Error, Key, Service, User};
use crate::driver;
use chrono::Utc;
use serde_json::Value;
use time::Duration;

/// Audit paths.
pub enum AuditPath {
    // TODO(refactor): Login type/provider.
    Login,
    LoginError(AuditLoginError),
}

/// Audit login error reasons.
#[derive(Debug, Serialize, Deserialize)]
pub enum AuditLoginError {
    UserNotFoundOrDisabled,
    KeyNotFoundOrDisabled,
    PasswordIncorrect,
}

impl AuditPath {
    /// Return string representation and JSON value of key.
    pub fn to_path_data(&self) -> (String, Value) {
        match self {
            AuditPath::Login => ("ark_auth/login".to_owned(), Value::default()),
            AuditPath::LoginError(reason) => {
                let value = serde_json::to_value(reason).unwrap();
                ("ark_auth/login_error".to_owned(), value)
            }
        }
    }
}

/// Create one audit log.
pub fn create(
    driver: &driver::Driver,
    meta: &AuditMeta,
    path: AuditPath,
    key: &Key,
    service: Option<&Service>,
    user: Option<&User>,
    user_key: Option<&Key>,
) -> Result<Audit, Error> {
    let (path, data) = path.to_path_data();
    driver
        .audit_create(
            &meta.user_agent,
            &meta.remote,
            meta.forwarded_for.as_ref().map(|x| &**x),
            &path,
            &data,
            &key.id,
            service.map(|x| x.id.as_ref()),
            user.map(|x| x.id.as_ref()),
            user_key.map(|x| x.id.as_ref()),
        )
        .map_err(Error::Driver)
}

/// Create one audit log and warn only on error result.
pub fn create_warn(
    driver: &driver::Driver,
    meta: &AuditMeta,
    path: AuditPath,
    key: &Key,
    service: Option<&Service>,
    user: Option<&User>,
    user_key: Option<&Key>,
) -> Option<Audit> {
    let (path, data) = path.to_path_data();
    match driver.audit_create(
        &meta.user_agent,
        &meta.remote,
        meta.forwarded_for.as_ref().map(|x| &**x),
        &path,
        &data,
        &key.id,
        service.map(|x| x.id.as_ref()),
        user.map(|x| x.id.as_ref()),
        user_key.map(|x| x.id.as_ref()),
    ) {
        Ok(audit) => Some(audit),
        Err(err) => {
            warn!("{}", Error::Driver(err));
            None
        }
    }
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
