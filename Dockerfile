# Stage 1: compile
FROM rust:1.87-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build

# Cache dependencies before copying source
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src ./src
RUN touch src/main.rs && cargo build --release

# Stage 2: minimal runtime image
FROM scratch
COPY --from=builder /build/target/release/oko /usr/local/bin/oko
ENTRYPOINT ["/usr/local/bin/oko"]
