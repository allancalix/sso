use crate::{grpc::method::auth::oauth2_login, prelude::*};

pub async fn oauth2_url(
    server: &GrpcServer,
    request: GrpcMethodRequest<()>,
) -> GrpcMethodResult<pb::AuthOauth2UrlReply> {
    let (audit_meta, auth, _) = request.into_inner();
    let driver = server.driver();
    let args = server.options().github_oauth2_args();

    blocking_method(move || {
        audit_result_err(
            driver.as_ref(),
            audit_meta,
            AuditType::AuthGithubOauth2Url,
            |driver, audit| provider_github::oauth2_url(driver, audit, &auth, &args),
        )
        .map_err(Into::into)
    })
    .await
    .map(|url| pb::AuthOauth2UrlReply { url })
}

impl validator::Validate for pb::AuthOauth2CallbackRequest {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        validate::wrap(|e| {
            validate::oauth2_token(e, "code", &self.code);
            validate::oauth2_token(e, "state", &self.state);
        })
    }
}

pub async fn oauth2_callback(
    server: &GrpcServer,
    request: GrpcMethodRequest<pb::AuthOauth2CallbackRequest>,
) -> GrpcMethodResult<pb::AuthTokenReply> {
    let (audit_meta, auth, req) = request.into_inner();

    let driver = server.driver();
    let args = server.options().github_oauth2_args();
    let audit_meta1 = audit_meta.clone();
    let (service, service_id, access_token) = blocking_method(move || {
        audit_result_err(
            driver.as_ref(),
            audit_meta1,
            AuditType::AuthGithubOauth2Callback,
            |driver, audit| provider_github::oauth2_callback(driver, audit, &auth, &args, &req),
        )
        .map_err(Into::into)
    })
    .await?;

    let client = server.client();
    let user_email = provider_github::api_user_email(&client, access_token)
        .await
        .map_err(GrpcMethodError::BadRequest)?;

    let driver = server.driver();
    let args = server.options().github_oauth2_args();
    blocking_method(move || {
        audit_result_err(
            driver.as_ref(),
            audit_meta,
            AuditType::AuthGithubOauth2Callback,
            |driver, audit| {
                oauth2_login(
                    driver,
                    audit,
                    &service,
                    service_id,
                    user_email.clone(),
                    args.access_token_expires,
                    args.refresh_token_expires,
                )
            },
        )
    })
    .await
    .map(|user_token| pb::AuthTokenReply {
        user: Some(user_token.user.clone().into()),
        access: Some(user_token.access_token()),
        refresh: Some(user_token.refresh_token()),
        audit: None,
    })
}

mod provider_github {
    use crate::{pattern::*, prelude::*};
    use oauth2::{
        basic::BasicClient, reqwest::http_client, AuthUrl, AuthorizationCode, ClientId,
        ClientSecret, CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl,
    };
    use reqwest::Client;

    pub(crate) fn oauth2_url(
        driver: &Postgres,
        audit: &mut AuditBuilder,
        auth: &HeaderAuth,
        args: &ServerProviderOauth2Args,
    ) -> GrpcMethodResult<String> {
        let service =
            key_service_authenticate(driver, audit, auth).map_err(GrpcMethodError::Unauthorised)?;

        // Generate the authorisation URL to which we'll redirect the user.
        let client = new_client(&service, &args.provider).map_err(GrpcMethodError::BadRequest)?;
        let (authorise_url, csrf_state) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("user:email".to_string()))
            .url();

        // Save the state and code verifier secrets as a CSRF key, value.
        let csrf_key = csrf_state.secret();
        let conn = driver.conn().map_err(GrpcMethodError::BadRequest)?;
        CsrfCreate::create(
            &conn,
            csrf_key,
            csrf_key,
            args.access_token_expires,
            service.id,
        )
        .map_err(GrpcMethodError::BadRequest)?;

        Ok(authorise_url.to_string())
    }

    pub(crate) fn oauth2_callback(
        driver: &Postgres,
        audit: &mut AuditBuilder,
        auth: &HeaderAuth,
        args: &ServerProviderOauth2Args,
        request: &pb::AuthOauth2CallbackRequest,
    ) -> GrpcMethodResult<(Service, Uuid, String)> {
        let service =
            key_service_authenticate(driver, audit, auth).map_err(GrpcMethodError::Unauthorised)?;

        // Read the CSRF key using state value, rebuild code verifier from value.
        let conn = driver.conn().map_err(GrpcMethodError::BadRequest)?;
        let csrf = CsrfRead::read(&conn, &request.state)
            .map_err(GrpcMethodError::BadRequest)?
            .ok_or_else(|| DriverError::CsrfNotFoundOrUsed)
            .map_err(GrpcMethodError::BadRequest)?;

        // Exchange the code with a token.
        let client = new_client(&service, &args.provider).map_err(GrpcMethodError::BadRequest)?;
        let code = AuthorizationCode::new(request.code.clone());
        let token = client
            .exchange_code(code)
            .request(http_client)
            .map_err(|err| DriverError::Oauth2Request(err.into()))
            .map_err(GrpcMethodError::BadRequest)?;

        // Return access token value.
        let (service_id, access_token) =
            (csrf.service_id(), token.access_token().secret().to_owned());

        Ok((service, service_id, access_token))
    }

    pub(crate) async fn api_user_email(
        client: &Client,
        access_token: String,
    ) -> DriverResult<String> {
        #[derive(Debug, Serialize, Deserialize)]
        struct GithubUser {
            email: String,
        }

        let authorisation = format!("token {}", access_token);
        let res = client
            .get("https://api.github.com/user")
            .header(header::AUTHORISATION, authorisation)
            .send()
            .await
            .map_err(DriverError::Reqwest)?;
        let res = res.error_for_status().map_err(DriverError::Reqwest)?;
        let res = res
            .json::<GithubUser>()
            .await
            .map_err(DriverError::Reqwest)?;
        Ok(res.email)
    }

    fn new_client(
        service: &Service,
        provider: &Option<GrpcServerOptionsProvider>,
    ) -> DriverResult<BasicClient> {
        let (provider_github_oauth2_url, provider) =
            match (&service.provider_github_oauth2_url, provider) {
                (Some(provider_github_oauth2_url), Some(provider)) => {
                    Ok((provider_github_oauth2_url, provider))
                }
                _ => Err(DriverError::ServiceProviderGithubOauth2Disabled),
            }?;

        let new_client_id = ClientId::new(provider.client_id.to_owned());
        let new_client_secret = ClientSecret::new(provider.client_secret.to_owned());

        let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
            .expect("Invalid authorisation endpoint URL");
        let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
            .expect("Invalid token endpoint URL");

        Ok(BasicClient::new(
            new_client_id,
            Some(new_client_secret),
            auth_url,
            Some(token_url),
        )
        .set_redirect_url(
            RedirectUrl::new(provider_github_oauth2_url.to_string()).expect("Invalid redirect URL"),
        ))
    }
}
