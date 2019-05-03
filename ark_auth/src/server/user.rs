//! # User
use crate::{
    core,
    server::{
        auth::{password_meta, PasswordMeta},
        route_json_config, route_response_empty, route_response_json, validate_name,
        validate_password, validate_unsigned, Data, Error, ValidateFromValue,
    },
};
use actix_web::{middleware::identity::Identity, web, HttpResponse};
use futures::{future, Future};
use validator::Validate;

/// List query.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    #[validate(custom = "validate_unsigned")]
    pub gt: Option<i64>,
    #[validate(custom = "validate_unsigned")]
    pub lt: Option<i64>,
    #[validate(custom = "validate_unsigned")]
    pub limit: Option<i64>,
}

impl ValidateFromValue<ListQuery> for ListQuery {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMetaResponse {
    pub gt: Option<i64>,
    pub lt: Option<i64>,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    pub meta: ListMetaResponse,
    pub data: Vec<core::User>,
}

/// API version 1 user list route.
pub fn api_v1_list(
    data: web::Data<Data>,
    id: Identity,
    query: web::Query<serde_json::Value>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let id = id.identity();

    ListQuery::from_value(query.into_inner())
        .and_then(|query| list_inner(data, id, query))
        .then(|result| route_response_json(result))
}

fn list_inner(
    data: web::Data<Data>,
    id: Option<String>,
    query: ListQuery,
) -> impl Future<Item = ListResponse, Error = Error> {
    web::block(move || {
        core::service_authenticate(data.driver(), id)
            .and_then(|service| {
                let limit = query.limit.unwrap_or(10);
                let (gt, lt, users) = match query.lt {
                    Some(lt) => {
                        let users =
                            core::user_list_where_id_lt(data.driver(), &service, lt, limit)?;
                        (None, Some(lt), users)
                    }
                    None => {
                        let gt = query.gt.unwrap_or(0);
                        let users =
                            core::user_list_where_id_gt(data.driver(), &service, gt, limit)?;
                        (Some(gt), None, users)
                    }
                };

                Ok(ListResponse {
                    meta: ListMetaResponse { gt, lt, limit },
                    data: users,
                })
            })
            .map_err(Into::into)
    })
    .map_err(Into::into)
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateBody {
    #[validate(custom = "validate_name")]
    pub name: String,
    #[validate(email)]
    pub email: String,
    #[validate(custom = "validate_password")]
    pub password: Option<String>,
}

impl ValidateFromValue<CreateBody> for CreateBody {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateResponse {
    meta: PasswordMeta,
    data: core::User,
}

/// API version 1 user create route.
pub fn api_v1_create(
    data: web::Data<Data>,
    id: Identity,
    body: web::Json<serde_json::Value>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let id = id.identity();

    CreateBody::from_value(body.into_inner())
        .and_then(|body| create_inner(data, id, body))
        .then(|result| route_response_json(result))
}

fn create_inner(
    data: web::Data<Data>,
    id: Option<String>,
    body: CreateBody,
) -> impl Future<Item = CreateResponse, Error = Error> {
    let (data1, body1) = (data.clone(), body.clone());

    web::block(move || {
        core::service_authenticate(data.driver(), id)
            .and_then(|service| {
                core::user_create(
                    data.driver(),
                    &service,
                    &body.name,
                    &body.email,
                    body.password.as_ref().map(|x| &**x),
                )
            })
            .map_err(Into::into)
    })
    .map_err(Into::into)
    .and_then(move |user| {
        let password_meta = password_meta(&data1, body1.password.as_ref().map(|x| &**x));
        let user = future::ok(user);
        password_meta.join(user)
    })
    .map(|(meta, user)| CreateResponse {
        meta,
        data: user.into(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResponse {
    data: core::User,
}

/// API version 1 user read route.
pub fn api_v1_read(
    data: web::Data<Data>,
    id: Identity,
    path: web::Path<(i64,)>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let id = id.identity();

    read_inner(data, id, path.0).then(|result| route_response_json(result))
}

fn read_inner(
    data: web::Data<Data>,
    id: Option<String>,
    user_id: i64,
) -> impl Future<Item = ReadResponse, Error = Error> {
    web::block(move || {
        core::service_authenticate(data.driver(), id)
            .and_then(|service| core::user_read_by_id(data.driver(), &service, user_id))
            .map_err(Into::into)
    })
    .map_err(Into::into)
    .map(|user| ReadResponse { data: user })
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdateBody {
    #[validate(custom = "validate_name")]
    pub name: Option<String>,
}

impl ValidateFromValue<UpdateBody> for UpdateBody {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResponse {
    data: core::User,
}

/// API version 1 user update route.
pub fn api_v1_update(
    data: web::Data<Data>,
    id: Identity,
    path: web::Path<(i64,)>,
    body: web::Json<serde_json::Value>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let id = id.identity();

    UpdateBody::from_value(body.into_inner())
        .and_then(move |body| update_inner(data, id, path.0, body))
        .then(|result| route_response_json(result))
}

fn update_inner(
    data: web::Data<Data>,
    id: Option<String>,
    user_id: i64,
    body: UpdateBody,
) -> impl Future<Item = UpdateResponse, Error = Error> {
    web::block(move || {
        core::service_authenticate(data.driver(), id)
            .and_then(|service| {
                core::user_update_by_id(
                    data.driver(),
                    &service,
                    user_id,
                    body.name.as_ref().map(|x| &**x),
                )
            })
            .map_err(Into::into)
    })
    .map_err(Into::into)
    .map(|user| UpdateResponse { data: user })
}

/// API version 1 user delete route.
pub fn api_v1_delete(
    data: web::Data<Data>,
    id: Identity,
    path: web::Path<(i64,)>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let id = id.identity();

    delete_inner(data, id, path.0).then(|result| route_response_empty(result))
}

fn delete_inner(
    data: web::Data<Data>,
    id: Option<String>,
    user_id: i64,
) -> impl Future<Item = usize, Error = Error> {
    web::block(move || {
        core::service_authenticate(data.driver(), id)
            .and_then(|service| core::user_delete_by_id(data.driver(), &service, user_id))
            .map_err(Into::into)
    })
    .map_err(Into::into)
}

/// API version 1 service scope.
pub fn api_v1_scope() -> actix_web::Scope {
    web::scope("/user")
        .service(
            web::resource("")
                .route(web::get().to_async(api_v1_list))
                .route(
                    web::post()
                        .data(route_json_config())
                        .to_async(api_v1_create),
                ),
        )
        .service(
            web::resource("/{user_id}")
                .route(web::get().to_async(api_v1_read))
                .route(
                    web::patch()
                        .data(route_json_config())
                        .to_async(api_v1_update),
                )
                .route(web::delete().to_async(api_v1_delete)),
        )
}