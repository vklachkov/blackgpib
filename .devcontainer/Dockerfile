FROM mcr.microsoft.com/devcontainers/rust:2-1-bullseye

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends build-essential curl python3 python3-distutils

WORKDIR /tmp
RUN git clone https://github.com/joan2937/pigpio.git
WORKDIR /tmp/pigpio
RUN make && sudo make install
WORKDIR /tmp
RUN rm -rf pigpio
