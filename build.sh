cargo build --target aarch64-linux-android -p aeon_eye --release
cp ./target/aarch64-linux-android/release/aeon_eye ./mode/system/bin/aeon_eye
cd ./mode/
zip -r ../build/aeon_eye.zip *
