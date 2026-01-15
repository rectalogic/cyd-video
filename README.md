# CYD Video

Video player for the ["Cheap Yellow Display"](https://github.com/witnessmenow/ESP32-Cheap-Yellow-Display) (`ESP32-2432S028R` based on `ESP32-D0WDQ6`)

## Development

```sh-session
$ cargo install espup espflash esp-generate
$ espup install --targets esp32
```

Configure environment variables, see the
[documentation](https://github.com/esp-rs/espup?tab=readme-ov-file#environment-variables-setup).
e.g. `. ~/export-esp.sh`.

Build and run on esp32. You must can a format feature, `mjpeg`, `yuv` or `rgb`, `mjpeg` is default.
The other formats are very slow due to large file size and slow SD card.
The SD card must have a corresponding file either `video.mjp`, `video.yuv` or `video.rgb`.

```sh-session
$ cd cyd-player
$ cargo run
```

Encode and play back video (requires [ffmpeg/ffplay](https://ffmpeg.org)):

```sh-session
$ cd cyd-encoder
$ cargo encode --format mjpeg --fps 25 <input.mp4> video.mjp
$ cargo preview --format mjpeg video.mjp
```

## Performance

The decoder is unuseably slow for `yuv` and `rgb`.
The bottleneck is reading these large uncompressed files from the SD card.
`SDIO` support may help [eventually](https://github.com/esp-rs/esp-hal/pull/3503).

`mjpeg` via [tjpgdec_rs](https://docs.rs/tjpgdec-rs/0.4.0/tjpgdec_rs/index.html)
is acceptable for lower framerates.
