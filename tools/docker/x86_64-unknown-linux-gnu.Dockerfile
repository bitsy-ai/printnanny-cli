FROM ubuntu:22.04
ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install --assume-yes libssl-dev

COPY common.sh lib.sh /
RUN /common.sh

COPY cmake.sh /
RUN /cmake.sh

COPY xargo.sh /
RUN /xargo.sh

COPY qemu.sh /
RUN /qemu.sh x86_64 softmmu

COPY dropbear.sh /
RUN /dropbear.sh

COPY linux-image.sh /
RUN /linux-image.sh x86_64

COPY linux-runner /


ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="/linux-runner x86_64"
