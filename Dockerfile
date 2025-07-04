FROM rust:latest as builder
COPY . /work
WORKDIR /work
RUN cargo build --release
FROM gcr.io/distroless/cc-debian12:latest
COPY --from=builder /work/target/release/limbo /limbo
CMD ["/limbo"]