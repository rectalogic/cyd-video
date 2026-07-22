# CYD Video

Originally a video player for the esp32
["Cheap Yellow Display" (CYD)](https://github.com/witnessmenow/ESP32-Cheap-Yellow-Display) (`ESP32-2432S028R`), like [this](https://www.aliexpress.us/item/3256804785406072.html).

However JPEG decoding was too slow on that device, and it was difficult to support audio.

So this is now a video player for the esp32s3 [E32N28P/E32C28P](https://www.lcdwiki.com/2.8inch_ESP32-S3_Display), like [this](https://www.aliexpress.us/item/3256811919417733.html).

## Development

Install [espflash](https://github.com/esp-rs/espflash/blob/main/espflash/README.md#installation) and [Docker](https://www.docker.com/products/docker-desktop/) (or [OrbStack](https://orbstack.dev))

Build and flash a connected device:
```sh-session
$ scripts/docker.sh cargo build --release
$ espflash flash --monitor --chip esp32s3 target/xtensa-esp32s3-none-elf/release/cyd-video
```

You can also build the firmware with an embedded video, if no SD card is detected the embedded video will play:
```sh-session
$ EMBED_VIDEO=test.avi scripts/docker.sh cargo build -F embed-video --release
```

The SD card must have an `AVI` directory containing 8.3 filename AVI videos encoded with `scripts/encode.sh`.

Encode and play back video (requires [ffmpeg/ffplay](https://ffmpeg.org)):

```sh-session
$ scripts/encode.sh <input.mp4> <output.avi>
```

You can also specify the framerate:
```sh-session
$ scripts/encode.sh -r 25 <input.mp4> <output.avi>
```
