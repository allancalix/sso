# Verify Token [POST /v1/auth/token/verify]

Verify user token.

## Request

```json
{
  "token": "eyJ0e...Dlgu4"
}
```

- `token`: JWT authentication token for user (required).

## Response [200, OK]

```json
{
  "data": {
    "user_id": 3,
    "token": "eyJ0e...Dlgu4",
    "token_expires": 1555957164
  }
}
```

### Data

- `user_id`: User ID.
- `token`: JWT authentication token for user.
- `token_expires`: JWT expiry time, unix timestamp.

### Test

```rust,skt-verify-ok
let (service, service_key) = service_key_create(&client);
let user_email = user_email_create();

let url = server_url("/v1/user");
let request = user::CreateBody {
    name: "User Name".to_owned(),
    email: user_email.clone(),
    active: true,
    password: Some("guest".to_owned()),
};
let mut response = client
    .post(&url)
    .header("content-type", "application/json")
    .header("authorization", service_key.value.clone())
    .json(&request)
    .send()
    .unwrap();
let body = response.json::<user::CreateResponse>().unwrap();
let user = body.data;
let status = response.status();
let content_type = header_get(&response, "content-type");
assert_eq!(status, 200);
assert_eq!(content_type, "application/json");
assert!(user.id > 0);
assert_eq!(user.name, "User Name");
assert_eq!(user.email, user_email);
assert!(user.password_hash.is_none());
assert!(user.password_revision.is_none());

let url = server_url("/v1/key");
let request = key::CreateBody {
    name: "Key Name".to_owned(),
    service_id: None,
    user_id: Some(user.id),
};
let mut response = client
    .post(&url)
    .header("content-type", "application/json")
    .header("authorization", service_key.value.clone())
    .json(&request)
    .send()
    .unwrap();
let body = response.json::<key::CreateResponse>().unwrap();
let user_key = body.data;
let status = response.status();
let content_type = header_get(&response, "content-type");
assert_eq!(status, 200);
assert_eq!(content_type, "application/json");
assert_eq!(user_key.name, "Key Name");
assert_eq!(user_key.service_id.unwrap(), service.id);
assert_eq!(user_key.user_id.unwrap(), user.id);

let url = server_url("/v1/auth/login");
let request = auth::LoginBody {
    email: user_email.clone(),
    password: "guest".to_owned(),
};
let mut response = client
    .post(&url)
    .header("content-type", "application/json")
    .header("authorization", service_key.value.clone())
    .json(&request)
    .send()
    .unwrap();
let body = response.json::<auth::LoginResponse>().unwrap();
let user_token = body.data;
let status = response.status();
let content_type = header_get(&response, "content-type");
assert_eq!(status, 200);
assert_eq!(content_type, "application/json");
assert_eq!(user_token.user_id, user.id);

// Service 2 cannot verify token.
let url = server_url("/v1/auth/token/verify");
let (_service2, service2_key) = service_key_create(&client);
let request = auth::TokenBody {
    token: user_token.token.clone(),
};
let response = client
    .post(&url)
    .header("content-type", "application/json")
    .header("authorization", service2_key.value.clone())
    .json(&request)
    .send()
    .unwrap();
let status = response.status();
let content_length = header_get(&response, "content-length");
assert_eq!(status, 400);
assert_eq!(content_length, "0");

// Service verifies token.
let request = auth::TokenBody {
    token: user_token.token.clone(),
};
let mut response = client
    .post(&url)
    .header("content-type", "application/json")
    .header("authorization", service_key.value.clone())
    .json(&request)
    .send()
    .unwrap();
let body = response.json::<auth::TokenResponse>().unwrap();
let user_token_verify = body.data;
let status = response.status();
let content_type = header_get(&response, "content-type");
assert_eq!(status, 200);
assert_eq!(content_type, "application/json");
assert_eq!(user_token_verify.user_id, user_token.user_id);
assert_eq!(user_token_verify.token, user_token.token);
assert_eq!(user_token_verify.token_expires, user_token.token_expires);
```

## Response [400, Bad Request]

- Request body is invalid.
- Token is invalid.
- Token is not for authorised service.

### Test

```rust,skt-verify-bad-request
let (_service, service_key) = service_key_create(&client);
let url = server_url("/v1/auth/token/verify");

// Invalid body (missing token property).
let request = json_value(r#"{}"#);
let response = client
    .post(&url)
    .header("content-type", "application/json")
    .header("authorization", service_key.value.clone())
    .json(&request)
    .send()
    .unwrap();
let status = response.status();
let content_length = header_get(&response, "content-length");
assert_eq!(status, 400);
assert_eq!(content_length, "0");

// Invalid body (invalid token property).
let request = json_value(r#"{ "token": "" }"#);
let response = client
    .post(&url)
    .header("content-type", "application/json")
    .header("authorization", service_key.value.clone())
    .json(&request)
    .send()
    .unwrap();
let status = response.status();
let content_length = header_get(&response, "content-length");
assert_eq!(status, 400);
assert_eq!(content_length, "0");
```

## Response [403, Forbidden]

- Authorisation header is invalid.

### Test

```rust,skt-verify-forbidden
let url = server_url("/v1/auth/token/verify");
let request = auth::TokenBody {
    token: "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9".to_owned(),
};

let response = client
    .post(&url)
    .header("content-type", "application/json")
    .json(&request)
    .send()
    .unwrap();
let status = response.status();
let content_length = header_get(&response, "content-length");
assert_eq!(status, 403);
assert_eq!(content_length, "0");

let response = client
    .post(&url)
    .header("content-type", "application/json")
    .header("authorization", "some-invalid-key")
    .json(&request)
    .send()
    .unwrap();
let status = response.status();
let content_length = header_get(&response, "content-length");
assert_eq!(status, 403);
assert_eq!(content_length, "0");
```