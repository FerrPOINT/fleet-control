ARG RUST_IMAGE
ARG DOCKER_IMAGE
FROM ${DOCKER_IMAGE} AS cli
FROM ${RUST_IMAGE}
COPY recipes/inputs.json /build-inputs/inputs.json
COPY recipes/snapshot.sh /build-inputs/snapshot.sh
RUN bash /build-inputs/snapshot.sh && apt-get install -y --no-install-recommends \
    python3 git pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY recipes/fetch.py /build-inputs/fetch.py
RUN python3 -B /build-inputs/fetch.py /build-inputs && \
    rustup component add --toolchain 1.88.0 rustfmt clippy && \
    test "$(rustc --version | awk '{print $2}')" = 1.88.0 && \
    groupadd --gid 999 fleet-control && useradd --uid 999 --gid 999 --no-create-home --home-dir /tmp fleet-control
COPY --from=cli /usr/local/bin/docker /usr/local/bin/docker
COPY --from=cli /usr/local/libexec/docker/cli-plugins/docker-compose /usr/local/libexec/docker/cli-plugins/docker-compose
ENV RUSTUP_TOOLCHAIN=1.88.0 RUSTUP_AUTO_INSTALL=0 PYTHONDONTWRITEBYTECODE=1
# Original build services own fresh volumes as root; native/qualifier select999.
WORKDIR /tmp
CMD ["false"]
