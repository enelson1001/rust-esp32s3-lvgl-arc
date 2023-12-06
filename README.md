# Rust ESP32S3 Lvgl Arc

The purpose of this demo is to use lv-binding-rust (Lvgl) and show arc with the foreground arc being updated to simulate software being loaded.

## Development Board
Aliexpress ESP32-8048S070 - 7 inch 800x400 TN RGB with ESP32S3, 8M PSRAM, 16M Flash, 512KB SRAM

## Overview
This application uses the lv-binding-rust crate on a ESP32S3 device.  The program will display a arc with a white background arc and a blue foreground arc that is constantly updated to show progress of a simulate software being loaded.  The progress is depicted by the blue arc moving from start angle to the end angle.

The program will output lvgl memory information to the terminal.
The following shows the lvgl memory right after lvgl_init and before the display is created, the touchscreen is created and before widgets are created.
```
rust_esp32s3_lvgl_arc: meminfo init: lv_mem_monitor_t { total_size: 49152, free_cnt: 2, free_size: 47676, free_biggest_size: 47664, used_cnt: 4, max_used: 60, used_pct: 4, frag_pct: 1 }
```

The following shows the lvgl memory information when the program is running.
```
rust_esp32s3_lvgl_arc: meminfo running: lv_mem_monitor_t { total_size: 49152, free_cnt: 2, free_size: 24364, free_biggest_size: 24352, used_cnt: 94, max_used: 22634, used_pct: 51, frag_pct: 1 }
```

## Comment
See rust-esp32s3-lvgl-clickme project for details on individual folders and files.
The following statement did not do what I expected, that is I assumed it would place the loading label above and in the middle of the arc.  But it placed the label in top left corner of the display.
```
loading_lbl.set_align(Align::OutTopMid, 0, 0)?;
```

I also notice that align_to is not supported by lv-binding-rust.  So I ended up just placing the loading label in the center of the arc.

## Flashing the ESP32S3 device
I used the following command to flash the ESP32S3 device.
```
$ cargo espflash flash --partition-table=partition-table/partitions.csv --monitor
```

## Picture of Aliexpress ESP32S3 running arc app
![esp32s3-arc](photos/arc.jpg)


# Versions
### v1.0 : 
- initial release
