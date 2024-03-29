FROM rust:alpine AS builder
WORKDIR /usr/src/empede
RUN apk add --no-cache build-base
COPY ./src ./src
COPY ./templates ./templates
COPY ./Cargo.toml ./Cargo.lock ./
RUN cargo install --path .

FROM alpine:latest
WORKDIR /app
COPY --from=builder /usr/local/cargo/bin/empede ./
COPY ./static ./static

ARG MPD_HOST
ARG MPD_PORT
ARG EMPEDE_BIND

CMD ["./empede"]
