# Login

Create service with key.

```bash
sso-cli create-service-with-key $service_name $service_url \
    --local-url $service_local_url
```

Service creates a user with password.

```bash
curl --header "Content-Type: application/json" \
  --header "Authorization: $service_key" \
  --request POST \
  --data '{"is_enabled":true,"name":"$user_name","email":"$user_email","locale":"en","timezone":"Etc/UTC","password_allow_reset":true,"password_require_update":false,"password":"$user_password"}' \
  sso.localhost/api/v1/user
```

Service creates a key for user.

```bash
curl --header "Content-Type: application/json" \
  --header "Authorization: $service_key" \
  --request POST \
  --data '{"is_enabled":true,"type":"TOKEN","name":"$key_name","user_id":"$user_id"}' \
  sso.localhost/api/v1/key
```

User makes login request to service, service makes a login request.

```bash
curl --header "Content-Type: application/json" \
  --header "Authorization: $service_key" \
  --request POST \
  --data '{"email":"$user_email","password":"$user_password"}' \
  sso.localhost/api/v1/auth/provider/local/login
```

Service receives token response, access token can be verified to authenticate requests.

```bash
curl --header "Content-Type: application/json" \
  --header "Authorization: $service_key" \
  --request POST \
  --data '{"token":"$access_token"}' \
  sso.localhost/api/v1/auth/token/verify
```

Refresh token can be used to refresh token.

```bash
curl --header "Content-Type: application/json" \
  --header "Authorization: $service_key" \
  --request POST \
  --data '{"token":"$refresh_token"}' \
  sso.localhost/api/v1/auth/token/refresh
```

Access or refresh token can be revoked, this will disable the key created earlier and prevent verify and refresh.

```bash
curl --header "Content-Type: application/json" \
  --header "Authorization: $service_key" \
  --request POST \
  --data '{"token":"$token"}' \
  sso.localhost/api/v1/auth/token/revoke
```
