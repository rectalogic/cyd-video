#!/usr/bin/env sh

ffmpeg -hide_banner -i "$1" \
  -an \
  -vf "framerate=15,scale=size=320x240:force_original_aspect_ratio=decrease:reset_sar=1:flags=lanczos" \
  -c:v mjpeg -q:v 10 \
  -f avi \
  -y "$2"
