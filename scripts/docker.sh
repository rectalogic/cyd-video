#!/usr/bin/env bash

root_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]:-$0}")" && pwd)/..

docker build -t cyd-video "$root_dir/.devcontainer" \
    && docker run --rm -it -e EMBED_VIDEO \
        -v "$root_dir:/home/esp/cyd-video" \
        -v cyd-video-cargo:/home/esp/.cargo/registry \
        -w /home/esp/cyd-video cyd-video \
        "$@"
