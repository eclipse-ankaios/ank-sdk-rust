FROM docker.io/alpine:3.18.4 as compile
ENV PATH="/root/.cargo/bin:${PATH}"

ARG WORKSPACE_DIR=/workspaces/build

RUN apk update && apk add --update-cache \
    curl \
    # Development tools
    protoc \
    build-base \
    && rm -rf /var/cache/apk/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y > /dev/null

COPY . ${WORKSPACE_DIR}/ankaios_sdk
COPY examples/app/ ${WORKSPACE_DIR}
WORKDIR ${WORKSPACE_DIR}
RUN --mount=type=cache,target=${WORKSPACE_DIR}/target/release cargo build --release \
    && cp ${WORKSPACE_DIR}/target/release/app /usr/local/bin/

ENV RUST_BACKTRACE=full
ENV RUST_LOG=trace

ENTRYPOINT ["/usr/local/bin/app"]
