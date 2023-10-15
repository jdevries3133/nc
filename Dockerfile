FROM alpine:3.18
COPY ./target/x86_64-unknown-linux-musl/release/nc nc
ENTRYPOINT [ "./nc" ]
