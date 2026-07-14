# Transitional source-build image. Normal registry publication consumes release
# descriptors and does not compile packages.
ARG KPM_BUILD_IMAGE=ghcr.io/bd452/kindle-kpm-build:v0.1.0
FROM ${KPM_BUILD_IMAGE}

WORKDIR /work
