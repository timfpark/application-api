FROM rust:1.55 as build

COPY ./ ./

RUN cargo build --release

RUN mkdir -p /build
RUN cp target/release/application-api /build/

FROM ubuntu:18.04

RUN apt-get update && apt-get -y upgrade
RUN apt-get -y install openssl
RUN apt-get -y install ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

COPY --from=build /build/application-api /

CMD ["/application-api"]
