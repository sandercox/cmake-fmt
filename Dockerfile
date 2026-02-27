FROM rust:latest AS builder
ARG REPO=https://github.com/sandercox/cmake-fmt.git
ARG REV=main

RUN cargo install cmake-fmt --git $REPO --rev $REV --locked

FROM debian:latest

RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    git-lfs \
    jq \
    && rm -rf /var/lib/apt/lists/*
COPY --from=0 /usr/local/cargo/bin/cmake-fmt /usr/local/bin/cmake-fmt