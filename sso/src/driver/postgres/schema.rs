table! {
    sso_audit (audit_id) {
        created_at -> Timestamptz,
        audit_id -> Uuid,
        audit_user_agent -> Varchar,
        audit_remote -> Varchar,
        audit_forwarded -> Nullable<Varchar>,
        audit_type -> Varchar,
        audit_data -> Jsonb,
        key_id -> Nullable<Uuid>,
        service_id -> Nullable<Uuid>,
        user_id -> Nullable<Uuid>,
        user_key_id -> Nullable<Uuid>,
    }
}

table! {
    sso_csrf (csrf_key) {
        created_at -> Timestamptz,
        csrf_key -> Varchar,
        csrf_value -> Varchar,
        csrf_ttl -> Timestamptz,
        service_id -> Uuid,
    }
}

table! {
    sso_key (key_id) {
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        key_id -> Uuid,
        key_is_enabled -> Bool,
        key_is_revoked -> Bool,
        key_allow_key -> Bool,
        key_allow_token -> Bool,
        key_allow_totp -> Bool,
        key_name -> Varchar,
        key_value -> Varchar,
        service_id -> Nullable<Uuid>,
        user_id -> Nullable<Uuid>,
    }
}

table! {
    sso_service (service_id) {
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        service_id -> Uuid,
        service_is_enabled -> Bool,
        service_name -> Varchar,
        service_url -> Varchar,
        service_provider_local_url -> Nullable<Varchar>,
        service_provider_github_oauth2_url -> Nullable<Varchar>,
        service_provider_microsoft_oauth2_url -> Nullable<Varchar>,
    }
}

table! {
    sso_user (user_id) {
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        user_id -> Uuid,
        user_is_enabled -> Bool,
        user_name -> Varchar,
        user_email -> Varchar,
        user_locale -> Varchar,
        user_timezone -> Varchar,
        user_password_hash -> Nullable<Varchar>,
        user_password_update_required -> Nullable<Bool>,
    }
}

joinable!(sso_audit -> sso_service (service_id));
joinable!(sso_audit -> sso_user (user_id));
joinable!(sso_csrf -> sso_service (service_id));
joinable!(sso_key -> sso_service (service_id));
joinable!(sso_key -> sso_user (user_id));

allow_tables_to_appear_in_same_query!(sso_audit, sso_csrf, sso_key, sso_service, sso_user,);