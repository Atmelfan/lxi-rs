# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.195.0/containers/rust/.devcontainer/base.Dockerfile
# [Choice] Debian OS version (use bullseye on local arm64/Apple Silicon): buster, bullseye
ARG VARIANT="bullseye"
FROM mcr.microsoft.com/devcontainers/rust:1-${VARIANT}

# [Optional] Uncomment this section to install additional packages.
# rpcbind required for VXI-11
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install rpcbind

# To generate coverage reports
RUN cargo +stable install cargo-llvm-cov
