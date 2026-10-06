# dprint-plugin-changelog is a stdin -> stdout filter, so the image only needs the binary itself.
# rust:alpine targets musl by default, which gives a fully static binary for scratch.
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked && strip target/release/dprint-plugin-changelog

FROM scratch

LABEL org.opencontainers.image.title="dprint-plugin-changelog" \
      org.opencontainers.image.description="dprint plugin for changelog files" \
      org.opencontainers.image.source="https://github.com/oriontvv/dprint-plugin-changelog" \
      org.opencontainers.image.licenses="Apache-2.0"

COPY --from=builder /src/target/release/dprint-plugin-changelog /dprint-plugin-changelog

ENTRYPOINT ["/dprint-plugin-changelog"]
