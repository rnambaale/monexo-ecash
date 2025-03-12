# build backend
FROM rust:1.83.0-slim-bullseye as rust-builder
RUN apt update && apt install -y make clang pkg-config protobuf-compiler curl

WORKDIR /rust-app
COPY . /rust-app
RUN cargo build  --package monexo-mint --release


FROM bitnami/minideb:bullseye
COPY --from=rust-builder /rust-app/target/release/monexo-mint /app/

COPY --chmod=755 ./entrypoint.sh /app/entrypoint.sh

USER 1000
WORKDIR /app
ENTRYPOINT ["/app/entrypoint.sh"]

ARG BUILDTIME
ARG COMMITHASH
ENV BUILDTIME ${BUILDTIME}
ENV COMMITHASH ${COMMITHASH}

CMD ["/app/monexo-mint"]
