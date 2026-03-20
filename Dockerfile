FROM node:22-alpine AS frontend-builder
WORKDIR /build/front

COPY front/package.json front/package-lock.json ./
RUN npm ci

COPY front/ ./
RUN npm run build

FROM rust:1.89-alpine AS backend-builder
WORKDIR /build

RUN apk add --no-cache musl-dev build-base

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM alpine:3.22 AS runtime
WORKDIR /app

RUN apk add --no-cache ca-certificates libgcc libstdc++ \
    && mkdir -p /app/static /data

COPY --from=backend-builder /build/target/release/ruinacounter2 /usr/local/bin/ruinacounter2
COPY --from=frontend-builder /build/front/dist/ /app/static/

ENV BIND_ADDR=0.0.0.0:11488
ENV STATUS_FILE=/data/status.json

VOLUME ["/data"]
EXPOSE 11488

CMD ["ruinacounter2"]
