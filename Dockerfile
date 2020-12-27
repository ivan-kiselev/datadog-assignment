FROM rust:1.48.0-alpine3.12 as build
WORKDIR /app
RUN apk add --no-cache musl-dev
COPY . /app
RUN cargo build --release


FROM alpine:3.12
COPY --from=build /app/target/release/clf-parser /bin/clf-parser
ENTRYPOINT ["/bin/clf-parser"]
CMD ["--help"]
