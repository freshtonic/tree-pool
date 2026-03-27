#!/bin/bash
# Record the demo animation. Run from the repo root: ./demo/record.sh
set -e
cd "$(dirname "$0")"
vhs demo.tape
mv demo.webp ../demo.webp
echo "done: demo.webp"
