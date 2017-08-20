# Builder container
FROM buildpack-deps:xenial-curl AS builder
RUN apt-get update; apt-get upgrade; apt-get install rustup
WORKDIR /chefi
COPY . .
RUN cargo build --target="x86_64-unknown-linux-musl" --release

# Final container
# TODO: why can't this be scratch? Is it openssl?
FROM alpine:latest  
RUN apk --no-cache add ca-certificates
WORKDIR /chefi
COPY --from=builder /chefi .
CMD ["/chefi/chefi"]  
