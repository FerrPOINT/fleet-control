ARG RUST_IMAGE
ARG DOCKER_IMAGE
FROM ${DOCKER_IMAGE} AS cli
FROM ${RUST_IMAGE}
COPY --from=cli /usr/local/bin/docker /usr/local/bin/docker
COPY --from=cli /usr/local/libexec/docker/cli-plugins/docker-compose /usr/local/libexec/docker/cli-plugins/docker-compose
