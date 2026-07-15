# CYD Video

Originally a video player for the esp32
["Cheap Yellow Display" (CYD)](https://github.com/witnessmenow/ESP32-Cheap-Yellow-Display) (`ESP32-2432S028R`), like [this](https://www.aliexpress.us/item/3256804785406072.html).

However JPEG decoding was too slow on that device, and it was difficult to support audio.

So this is now a video player for the esp32s3 [E32N28P/E32C28P](https://www.lcdwiki.com/2.8inch_ESP32-S3_Display), like [this](https://www.aliexpress.us/item/3256811919417733.html).

## Development

```sh-session
$ cargo install espup espflash
$ espup install --targets esp32s3
```

Configure environment variables, see the
[documentation](https://github.com/esp-rs/espup?tab=readme-ov-file#environment-variables-setup).
e.g. `. ~/export-esp.sh`.

The SD card must have an `AVI` directory containing 8.3 filename AVI videos encoded with `scripts/encode.sh`.

```sh-session
$ cd cyd-player
$ cargo run --release
```
or
```sh-session
$ cd cyd-player
$ cargo build --release
```

Encode and play back video (requires [ffmpeg/ffplay](https://ffmpeg.org)):

```sh-session
$ scripts/encode.sh <input.mp4> output.avi
```
