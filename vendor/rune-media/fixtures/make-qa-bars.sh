#!/bin/sh
# Writes qa-bars-60fps.m4v: no B-frames and no edit lists, like Greyscreen's own recordings.
set -eu
out="$(cd "$(dirname "$0")" && pwd)/qa-bars-60fps.m4v"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
# The audio starts 1024 samples early: AAC adds that delay and the file has no edit list to hide it.
delay=0.021333333
ffmpeg -v error -y \
  -f lavfi -i "color=c=black:s=320x180:r=60:d=6,format=yuv420p,geq=lum='if(mod(floor(N/pow(2\,floor(X*10/W)))\,2)\,235\,16)':cb=128:cr=128" \
  -f lavfi -i "aevalsrc=exprs='0.1*sin(2*PI*220*(t+$delay))+0.8*sin(2*PI*1000*(t+$delay))*lt(mod(t+$delay\,1)\,0.05)':s=48000:c=stereo:d=6" \
  -c:v libx264 -profile:v high -pix_fmt yuv420p -g 60 -bf 0 -crf 30 -c:a aac -b:a 64k -shortest "$work/encoded.m4v"
ffmpeg -v error -y -i "$work/encoded.m4v" -c copy -use_editlist 0 -avoid_negative_ts disabled -movflags +faststart "$out"
