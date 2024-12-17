FROM rust:1-alpine3.21 AS builder

WORKDIR /app

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    postgresql-dev \
    pkgconfig

ENV SQLX_OFFLINE true

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src ./src
COPY .sqlx ./.sqlx
RUN touch ./src/main.rs && cargo build --release --bin ucsd-student-evals

FROM alpine:3.21 AS runtime
WORKDIR /app

COPY --from=builder /app/target/release/ucsd-student-evals /app/ucsd-student-evals

EXPOSE 3000

ENTRYPOINT ["/app/ucsd-student-evals", "serve"]
