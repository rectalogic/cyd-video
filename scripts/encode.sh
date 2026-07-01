#!/usr/bin/env sh

# Scale to fit in 320x240, maintaining AR, and then pad width/height to multiple of 8
ffmpeg -hide_banner -i "$1" \
  -an \
  -vf "framerate=15,scale=320x240:force_original_aspect_ratio=decrease:reset_sar=1:flags=lanczos,pad=ceil(iw/8)*8:ceil(ih/8)*8:(ow-iw)/2:(oh-ih)/2" \
  -c:v mjpeg -q:v 10 \
  -f avi \
  -y "$2"
