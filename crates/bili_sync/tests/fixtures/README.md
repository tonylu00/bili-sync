`shared-media.mp4` is an original test fixture: one second of black, 16 by 16 pixels, H.264, no audio.
It contains no third-party media. Generate with:

```sh
ffmpeg -v error -f lavfi -i color=c=black:s=16x16:r=1 -t 1 -c:v libx264 -pix_fmt yuv420p -movflags +faststart -y shared-media.mp4
```

The shared-media integration tests require `ffprobe` on PATH. CI installs FFmpeg before testing.
