#!/bin/bash
export SDK_PATH=/opt/MacOSX11.3.sdk
export SDKROOT=$SDK_PATH
export RUSTFLAGS="-C link-arg=-isysroot -C link-arg=$SDK_PATH -C link-arg=-F -C link-arg=$SDK_PATH/System/Library/Frameworks -C link-arg=-L -C link-arg=$SDK_PATH/usr/lib"

cargo clean
cargo build --release
cargo build --target x86_64-pc-windows-gnu --release
cargo zigbuild --target aarch64-apple-darwin --release
cargo zigbuild --target x86_64-apple-darwin --release
