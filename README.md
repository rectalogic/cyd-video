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

Build and run on esp32. You must specify a format feature, `mjpeg`, `yuv` or `rgb`.
The SD card must have a corresponding file either `video.mjp`, `video.yuv` or `video.rgb`.

```sh-session
$ cd cyd-player
$ cargo run -F yuv
```

Encode and play back video (requires [ffmpeg/ffplay](https://ffmpeg.org)):

```sh-session
$ cd cyd-encoder
$ cargo encode --format mjpeg --fps 25 <input.mp4> video.mjp
$ cargo preview --format mjpeg video.mjp
```

## Performance

The decoder is unuseably slow. The bottleneck is reading from the SD card.
`esp_hal` only supports `SPI` for the SD card, not the faster `SDIO`.
`SDIO` support is being work on in https://github.com/esp-rs/esp-hal/pull/3503

Also the `zune_jpeg` decoder needs `alloc` and decodes to a full RGB buffer.
This seems to consume all available RAM unless you constrain the frame size to something
very small (which is what this implementation does).
