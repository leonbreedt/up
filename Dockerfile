# stage 1: build
FROM rust:1.62.1 as build

RUN curl -fsSL https://deb.nodesource.com/setup_16.x | bash -
RUN apt-get update && apt-get install -y nodejs && rm -rf /var/lib/apt/lists/*

# stage 1.1: cache dependencies
WORKDIR /workspace
RUN cargo new --bin up-server
COPY ./Cargo.lock ./up-server/Cargo.lock
COPY ./up-server/Cargo.toml ./up-server/Cargo.toml
RUN cd up-server && cargo build --release && rm src/*.rs

# stage 1.2: build UI
COPY ./up-ui ./up-ui
RUN cd up-ui && npm run clean && npm install && npm run build

# stage 1.3: build server
COPY ./up-server/src ./up-server/src
COPY ./up-server/migrations ./up-server/migrations
RUN cd up-server && rm ./target/release/deps/up_server* && cargo build --release

# stage 2: runtime
FROM gcr.io/distroless/cc
COPY --from=build /workspace/up-server/target/release/up-server /
EXPOSE 8080

USER nonroot
CMD ["/up-server", "--listen-address", "0.0.0.0:8080"]
