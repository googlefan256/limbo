FROM debian:latest AS builder
COPY .out /out
RUN if [ "$(arch)" = "x86_64" ]; then \
        cp -r /out/x86_64-linux-gnu /limbo; \
    else \
        cp -r /out/aarch64-linux-gnu /limbo; \
    fi
RUN rm -rf /out
FROM gcr.io/distroless/cc-debian12:latest
COPY --from=builder /limbo /limbo
CMD ["/limbo"]