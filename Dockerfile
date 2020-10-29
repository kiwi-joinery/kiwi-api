FROM rust:latest AS build
WORKDIR /build/
# Cache the dependencies specified in Cargo.toml
RUN env USER=root cargo init . --name temp
COPY Cargo.toml .
RUN cargo check --release
# Do the real build
COPY ./src ./src
COPY ./migrations ./migrations
RUN cargo build --release

FROM ubuntu:latest
RUN apt update && apt-get --assume-yes install libssl-dev libpq-dev ca-certificates
COPY --from=build /build/target/release/kiwi_api /opt/kiwi/
WORKDIR /kiwi/
ENV APP.PORT 9000
ENV APP.STORAGE_FOLDER /kiwi/storage
EXPOSE 9000
ENTRYPOINT ["/opt/kiwi/kiwi_api", "-c", "./config.toml"]
