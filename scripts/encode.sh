#!/usr/bin/env sh

USAGE="Usage: $0 [-h] [-r framerate] <input-video> <output-video>.avi"
fps=15
while getopts "hr:" opt; do
    case $opt in
        h)
            echo $USAGE
            echo "  -h  Show this help"
            echo "  -r  Frames per second"
            exit 0
            ;;
        r)
            fps="$OPTARG"
            ;;
        ?)
            echo $USAGE >&2
            exit 1
            ;;
    esac
done

shift $(( OPTIND - 1 ))

# Scale to fit in 320x240, maintaining AR, and then pad width/height to multiple of 8
ffmpeg -hide_banner -i "$1" \
  -vf "framerate=${fps},scale=320x240:force_original_aspect_ratio=decrease:reset_sar=1:flags=lanczos,pad=ceil(iw/8)*8:ceil(ih/8)*8:(ow-iw)/2:(oh-ih)/2,format=yuv420p" \
  -c:a pcm_s16le -ar 16000 -ac 1 \
  -c:v mjpeg -q:v 10 \
  -f avi \
  -y "$2"
