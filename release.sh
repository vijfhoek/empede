#!/bin/sh
set -ex

cargo build --release

rm -rf release/empede-$1-x86_64-linux
mkdir -p release/empede-$1-x86_64-linux
cp -r target/release/empede static/ README.md release/empede-$1-x86_64-linux

if [ ! -z "$NIX_STORE" ]; then
  patchelf --set-interpreter /usr/lib64/ld-linux-x86-64.so.2 release/empede-$1-x86_64-linux/empede
fi
strip release/empede-$1-x86_64-linux/empede

cd release
tar czf empede-$1-x86_64-linux.tar.gz empede-$1-x86_64-linux
