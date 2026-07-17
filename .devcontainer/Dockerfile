# See https://github.com/esp-rs/rust-build/blob/main/idf-rust.Containerfile
# https://hub.docker.com/layers/espressif/idf-rust/esp32s3_latest/images/sha256-4eadb6ba185332e0605b0c89287e8038288c4c7160d43c996da4a69d15cffca3

FROM debian:trixie-slim
ENV DEBIAN_FRONTEND=noninteractive
ENV LC_ALL=C.UTF-8
ENV LANG=C.UTF-8

RUN apt-get update \
    && apt-get install -y git curl llvm-dev libclang-dev clang unzip \
    libusb-1.0-0 libssl-dev libudev-dev pkg-config \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/* /tmp/library-scripts

RUN useradd -ms /bin/bash esp
USER esp
WORKDIR /home/esp

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
    --default-toolchain none -y --profile minimal
ENV PATH=${PATH}:/home/esp/.cargo/bin

RUN ARCH=$(/home/esp/.cargo/bin/rustup show | grep "Default host" | sed -e 's/.* //') && \
    curl -L "https://github.com/esp-rs/espup/releases/latest/download/espup-${ARCH}" -o "/home/esp/.cargo/bin/espup" && \
    chmod u+x "/home/esp/.cargo/bin/espup" && \
    curl -L "https://github.com/esp-rs/espflash/releases/latest/download/cargo-espflash-${ARCH}.zip" -o "/home/esp/.cargo/bin/cargo-espflash.zip" && \
    unzip "/home/esp/.cargo/bin/cargo-espflash.zip" -d "/home/esp/.cargo/bin/" && \
    rm "/home/esp/.cargo/bin/cargo-espflash.zip" && \
    chmod u+x "/home/esp/.cargo/bin/cargo-espflash" && \
    curl -L "https://github.com/esp-rs/espflash/releases/latest/download/espflash-${ARCH}.zip" -o "/home/esp/.cargo/bin/espflash.zip" && \
    unzip "/home/esp/.cargo/bin/espflash.zip" -d "/home/esp/.cargo/bin/" && \
    rm "/home/esp/.cargo/bin/espflash.zip" && \
    chmod u+x "/home/esp/.cargo/bin/espflash"

RUN espup install --targets esp32s3 --export-file /home/esp/export-esp.sh --toolchain-version 1.93.0.0
RUN echo "source /home/esp/export-esp.sh" >> ~/.bashrc

CMD [ "/bin/bash" ]
