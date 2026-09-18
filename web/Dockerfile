# petrovich-web service image (multi-stage, Linux/amd64).
FROM rust:1.84-bookworm AS builder
WORKDIR /build
# Workspace layout: root crate (petrovich-core) + cli/ + web/.
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY src ./src
COPY cli ./cli
COPY web ./web
RUN cargo build --locked --release -p petrovich-web

FROM debian:bookworm-slim
RUN useradd --create-home --shell /usr/sbin/nologin petrovich
COPY --from=builder /build/target/release/petrovich-web /usr/local/bin/petrovich-web
USER petrovich
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/petrovich-web", "--bind", "0.0.0.0:8080"]
