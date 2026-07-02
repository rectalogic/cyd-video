# CYD Video

Video player for the esp32 ["Cheap Yellow Display"](https://github.com/witnessmenow/ESP32-Cheap-Yellow-Display) (`ESP32-2432S028R` based on `ESP32-D0WDQ6`) and
[esp32-s3 display](https://www.lcdwiki.com/2.8inch_ESP32-S3_Display).

For example [esp32s3](https://www.aliexpress.us/item/3256811919417733.html),
[esp32](https://www.aliexpress.us/item/3256804785406072.html).

## Development

```sh-session
$ cargo install espup espflash esp-generate
$ espup install --targets esp32 esp32s3
```

Configure environment variables, see the
[documentation](https://github.com/esp-rs/espup?tab=readme-ov-file#environment-variables-setup).
e.g. `. ~/export-esp.sh`.

The SD card must have an `AVI` directory containing 8.3 filename AVI videos encoded with `scripts/encode.sh`.

```sh-session
$ cd cyd-player
$ cargo run run-esp32
$ cargo run run-esp32s3
```

Encode and play back video (requires [ffmpeg/ffplay](https://ffmpeg.org)):

```sh-session
$ scripts/encode.sh <input.mp4> output.avi
```

## Performance

On esp32 playback can nearly achieve 15fps.
