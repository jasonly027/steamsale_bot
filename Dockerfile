FROM rust:1.89-alpine3.20 as builder

RUN apk add musl-dev openssl-dev openssl-libs-static ca-certificates

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./

RUN cargo fetch

COPY . .

RUN cargo install --path .

FROM scratch

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

COPY --from=builder /usr/local/cargo/bin/steamsale_bot .

CMD [ "./steamsale_bot" ]
