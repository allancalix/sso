CREATE TABLE auth_service (
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    service_id UUID NOT NULL,
    service_is_enabled BOOLEAN NOT NULL,
    service_name VARCHAR NOT NULL,
    service_url VARCHAR NOT NULL,
    service_provider_local_url VARCHAR,
    service_provider_github_oauth2_url VARCHAR,
    service_provider_microsoft_oauth2_url VARCHAR,
    PRIMARY KEY (service_id)
);

CREATE TABLE auth_user (
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    user_id UUID NOT NULL,
    user_is_enabled BOOLEAN NOT NULL,
    user_name VARCHAR NOT NULL,
    user_email VARCHAR NOT NULL,
    user_locale VARCHAR NOT NULL,
    user_timezone VARCHAR NOT NULL,
    user_password_hash VARCHAR,
    user_password_update_required BOOLEAN,
    PRIMARY KEY (user_id),
    CONSTRAINT uq_auth_user_email UNIQUE(user_email)
);

CREATE TABLE auth_key (
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    key_id UUID NOT NULL,
    key_is_enabled BOOLEAN NOT NULL,
    key_is_revoked BOOLEAN NOT NULL,
    key_allow_key BOOLEAN NOT NULL,
    key_allow_token BOOLEAN NOT NULL,
    key_allow_totp BOOLEAN NOT NULL,
    key_name VARCHAR NOT NULL,
    key_value VARCHAR NOT NULL,
    service_id UUID,
    user_id UUID,
    PRIMARY KEY (key_id),
    CONSTRAINT uq_auth_key_value UNIQUE(key_value),
    CONSTRAINT fk_auth_key_service
        FOREIGN KEY (service_id)
        REFERENCES auth_service(service_id)
        ON DELETE RESTRICT,
    CONSTRAINT fk_auth_key_user
        FOREIGN KEY (user_id)
        REFERENCES auth_user(user_id)
        ON DELETE RESTRICT
);
CREATE UNIQUE INDEX idx_auth_key_allow_token ON auth_key (service_id, user_id)
    WHERE key_is_enabled IS TRUE AND key_allow_token IS TRUE;
CREATE UNIQUE INDEX idx_auth_key_allow_totp ON auth_key (service_id, user_id)
    WHERE key_is_enabled IS TRUE AND key_allow_totp IS TRUE;

CREATE TABLE auth_csrf (
    created_at TIMESTAMPTZ NOT NULL,
    csrf_key VARCHAR NOT NULL,
    csrf_value VARCHAR NOT NULL,
    csrf_ttl TIMESTAMPTZ NOT NULL,
    service_id UUID NOT NULL,
    PRIMARY KEY (csrf_key),
    CONSTRAINT fk_auth_csrf_service
        FOREIGN KEY (service_id)
        REFERENCES auth_service(service_id)
        ON DELETE CASCADE
);

CREATE TABLE auth_audit (
    created_at TIMESTAMPTZ NOT NULL,
    audit_id UUID NOT NULL,
    audit_user_agent VARCHAR NOT NULL,
    audit_remote VARCHAR NOT NULL,
    audit_forwarded VARCHAR,
    audit_type VARCHAR NOT NULL,
    audit_data JSONB NOT NULL,
    key_id UUID,
    service_id UUID,
    user_id UUID,
    user_key_id UUID,
    PRIMARY KEY (audit_id),
    CONSTRAINT fk_auth_audit_key
        FOREIGN KEY (key_id)
        REFERENCES auth_key(key_id)
        ON DELETE RESTRICT,
    CONSTRAINT fk_auth_audit_service
        FOREIGN KEY (service_id)
        REFERENCES auth_service(service_id)
        ON DELETE RESTRICT,
    CONSTRAINT fk_auth_audit_user
        FOREIGN KEY (user_id)
        REFERENCES auth_user(user_id)
        ON DELETE RESTRICT,
    CONSTRAINT fk_auth_audit_user_key
        FOREIGN KEY (user_key_id)
        REFERENCES auth_key(key_id)
        ON DELETE RESTRICT
);
CREATE INDEX idx_auth_audit_created_at ON auth_audit(created_at DESC, audit_type);