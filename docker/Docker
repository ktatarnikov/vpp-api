FROM ubuntu:latest

# Essential
RUN apt-get update && \
    apt-get install -y \
        curl \
        vim \
        procps \
        build-essential \
        clang \
        git

# Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --no-modify-path
RUN echo '. "$HOME/.cargo/env"' >> ~/.bashrc 

# fd.io debian packages
RUN curl -s https://packagecloud.io/install/repositories/fdio/release/script.deb.sh | bash

RUN mkdir -p /var/log/vpp/vpp.log

WORKDIR /home/ubuntu

CMD ["/bin/bash"]

# apt-get install -y libvppinfra=25.10-release vpp=25.10-release vpp-plugin-core=25.10-release
