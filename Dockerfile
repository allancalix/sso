FROM debian:10.2
ENV DEBIAN_FRONTEND="noninteractive"

# Install dependencies.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    wget unzip ca-certificates build-essential libpq-dev libssl-dev pkg-config git \
    && rm -rf /var/lib/apt/lists/*;

# Environment.
ENV HOME="/root"

# Rust environment.
ENV RUSTUP_HOME="/usr/local/rustup" \
    CARGO_HOME="/usr/local/cargo" \
    PATH="/usr/local/cargo/bin:$PATH" \
    RUST_VERSION="1.40.0" \
    RUSTUP_URL="https://static.rust-lang.org/rustup/archive/1.20.2/x86_64-unknown-linux-gnu/rustup-init"

# Install Rust toolchain.
# <https://github.com/rust-lang/docker-rust>
RUN wget -q "$RUSTUP_URL" \
    && chmod +x rustup-init \
    && ./rustup-init -y --no-modify-path --profile default --default-toolchain $RUST_VERSION \
    && rm rustup-init \
    && chmod -R a+w $RUSTUP_HOME $CARGO_HOME \
    && chmod 777 -R $HOME;

# Install Rust tools.
RUN cargo install --force cargo-make \
    && cargo install --force diesel_cli --no-default-features --features "postgres" \
    && cargo install --force cargo-audit;

# Go environment.
ENV PATH="/usr/local/go/bin:/root/go/bin:$PATH" \
    GOLANG_URL="https://golang.org/dl/go1.13.5.linux-amd64.tar.gz" \
    PROTOC_URL="https://github.com/protocolbuffers/protobuf/releases/download/v3.11.1/protoc-3.11.1-linux-x86_64.zip"

# Install Go toolchain.
# <https://github.com/docker-library/golang>
RUN wget -O go.tgz -q "$GOLANG_URL" \
    && tar -C /usr/local -xzf go.tgz \
    && rm go.tgz \
    && wget -O protoc.zip -q "$PROTOC_URL" \
    && unzip -o protoc.zip -d /usr/local bin/protoc \
    && unzip -o protoc.zip -d /usr/local 'include/*' \
    && chmod -R 777 /usr/local/bin/protoc \
    && chmod -R 777 /usr/local/include/google \
    && rm protoc.zip;

# Install Go tools.
# <https://github.com/grpc-ecosystem/grpc-gateway>
# <https://grpc-ecosystem.github.io/grpc-gateway/>
RUN go get -u github.com/grpc-ecosystem/grpc-gateway/protoc-gen-grpc-gateway \
    && go get -u github.com/grpc-ecosystem/grpc-gateway/protoc-gen-swagger \
    && go get -u github.com/golang/protobuf/protoc-gen-go \
    && go get -u google.golang.org/grpc;

# Pandoc environment.
ENV PANDOC_URL="https://github.com/jgm/pandoc/releases/download/2.9/pandoc-2.9-1-amd64.deb"

# Install Pandoc.
# <https://pandoc.org/installing.html>
RUN wget -O pandoc.deb -q "$PANDOC_URL" \
    && dpkg -i pandoc.deb \
    && rm pandoc.deb;

# -----------------------
# Development Environment
# -----------------------
# This file is checked into Git and must not contain secrets!

# sso
# # Sentry URL for logging integration.
# ENV SSO_SENTRY_URL=""
# Database connection URL.
ENV SSO_DATABASE_URL="postgres://guest:guest@localhost:5432/sso"
# Database number of connections.
ENV SSO_DATABASE_CONNECTIONS="10"
# Server bind addresses (gRPC and HTTP).
ENV SSO_BIND="0.0.0.0:7000" \
    SSO_HTTP_BIND="0.0.0.0:7010"
# # Server TLS configuration.
# ENV SSO_TLS_CERT_PEM="" \
#     SSO_TLS_KEY_PEM="" \
#     SSO_TLS_CLIENT_PEM=""
# # SMTP server transport configuration.
# ENV SSO_SMTP_HOST="" \
#     SSO_SMTP_PORT="" \
#     SSO_SMTP_USER="" \
#     SSO_SMTP_PASSWORD=""
# SMTP file transport configuration.
ENV SSO_SMTP_FILE="/sso/target/smtp"
# Password pwned integration enabled.
ENV SSO_PASSWORD_PWNED="true"
# # Github OAuth2 support.
# ENV SSO_GITHUB_CLIENT_ID="" \
#     SSO_GITHUB_CLIENT_SECRET=""
# # Microsoft OAuth2 support.
# ENV SSO_MICROSOFT_CLIENT_ID="" \
#     SSO_MICROSOFT_CLIENT_SECRET=""
# OpenAPI gateway configuration.
ENV SSO_OPENAPI_GRPC="localhost:7000" \
    SSO_OPENAPI_BIND="localhost:8000"
# Integration test variables.
ENV SSO_TEST_URL="http://localhost:7000" \
    SSO_TEST_KEY="E2GGVLXZJ6NPEV6SJKAHYISSGPNL23LTJE"

# Copy project files and set working directory.
# These are required for docker-compose service builds.
ADD . /sso
ADD ./docker/build/Cargo.toml /sso/Cargo.toml
WORKDIR /sso

# Set cargo cache directory in volume.
# This prevents having to download dependencies in development builds.
ENV CARGO_HOME="/sso/.cargo"

ADD ./docker/build/versions.sh /versions.sh
RUN chmod +x /versions.sh
CMD ["/versions.sh"]
