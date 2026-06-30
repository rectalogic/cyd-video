FROM docker.io/espressif/idf-rust:esp32_latest

USER root
RUN apt-get update \
    && apt-get install -y sudo \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/* /tmp/library-scripts
RUN usermod -aG sudo esp
RUN echo "esp ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers
USER esp
