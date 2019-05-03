//! # GitHub
use crate::server::{Data, Error};
use actix_http::http::header::ContentType;
use actix_web::http::{header, StatusCode};
use actix_web::middleware::identity::Identity;
use actix_web::{web, HttpResponse};
use futures::{future, Future};
use oauth2::basic::BasicClient;
use oauth2::prelude::*;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct GithubUser {
    pub email: String,
}

pub fn v1(
    data: web::Data<Data>,
    id: Identity,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let id = id.identity();

    web::block(move || authenticate(&data, id).and_then(|s| github_authorise(&data, s)))
        .map_err(Into::into)
        .map(|x| HttpResponse::build(StatusCode::OK).json(UrlResponse { url: x }))
}

pub fn v1_callback(
    data: web::Data<Data>,
    query: web::Query<CallbackQuery>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    web::block(move || github_callback(data, &query.code, &query.state))
        .map_err(Into::into)
        .and_then(move |(data, access_token, service_id)| {
            github_api_user_email(data, &access_token, service_id)
        })
        .and_then(|(data, email, service_id)| {
            web::block(move || oauth2_login(&data, &email, service_id)).map_err(Into::into)
        })
        .map_err(Into::into)
        .map(|(token, service)| oauth2_redirect(token, service))
}

fn github_authorise(data: &web::Data<Data>, service: AuthService) -> Result<String, Error> {
    // Generate the authorization URL to which we'll redirect the user.
    let client = github_client(data.oauth2_github())?;
    let (authorize_url, state) = client.authorize_url(CsrfToken::new_random);

    // Save the state and code verifier secrets as a CSRF key, value.
    data.db
        .csrf_create(&state.secret(), &state.secret(), service.service_id)
        .map_err(Error::Db)?;

    Ok(authorize_url.to_string())
}

fn github_callback(
    data: web::Data<Data>,
    code: &str,
    state: &str,
) -> Result<(web::Data<Data>, String, i64), Error> {
    // Read the CSRF key using state value, rebuild code verifier from value.
    let csrf = data.db.csrf_read_by_key(&state).map_err(Error::Db)?;

    // Exchange the code with a token.
    let client = github_client(data.oauth2_github())?;
    let code = AuthorizationCode::new(code.to_owned());
    let token = client
        .exchange_code(code)
        .map_err(|_e| Error::Unwrap("failed to exchange code"))?;

    // Return access token value.
    Ok((
        data,
        token.access_token().secret().to_owned(),
        csrf.service_id,
    ))
}

fn github_api_user_email(
    data: web::Data<Data>,
    access_token: &str,
    service_id: i64,
) -> impl Future<Item = (web::Data<Data>, String, i64), Error = Error> {
    let client = actix_web::client::Client::new();
    let authorisation_header = format!("token {}", access_token);
    client
        .get("https://api.github.com/user")
        .header(header::AUTHORIZATION, authorisation_header)
        .header(header::CONTENT_TYPE, ContentType::json())
        .header(header::USER_AGENT, data.user_agent())
        .send()
        .map_err(|_e| Error::Unwrap("failed to client.request"))
        .and_then(|response| match response.status() {
            StatusCode::OK => future::ok(response),
            _ => future::err(Error::Unwrap("failed to receive ok response")),
        })
        .and_then(|mut response| {
            response
                .json::<GithubUser>()
                .map_err(|_e| Error::Unwrap("failed to parse json"))
        })
        .map(move |response| (data, response.email, service_id))
}

fn github_client(provider: Option<&ApiConfigOauth2Provider>) -> Result<BasicClient, Error> {
    let provider = provider.ok_or(Error::InvalidOauth2Provider)?;

    let github_client_id = ClientId::new(provider.client_id.to_owned());
    let github_client_secret = ClientSecret::new(provider.client_secret.to_owned());
    let auth_url = AuthUrl::new(Url::parse("https://github.com/login/oauth/authorize").unwrap());
    let token_url =
        TokenUrl::new(Url::parse("https://github.com/login/oauth/access_token").unwrap());

    Ok(BasicClient::new(
        github_client_id,
        Some(github_client_secret),
        auth_url,
        Some(token_url),
    )
    .add_scope(Scope::new("user:email".to_string()))
    .set_redirect_url(RedirectUrl::new(
        Url::parse(&provider.redirect_url).unwrap(),
    )))
}