## Localshare
Share your files by starting an HTTP server(using [dufs](https://github.com/sigoden/dufs)).
You can then access them from any device on your network using browser. Support authentication, searching and uploading.
Support Windows, Linux and Android thanks to [tauri](https://github.com/tauri-apps/tauri).

### build
```zsh
pnpm tauri-build-win; pnpm tauri-build; pnpm tauri-build-apk
mkdir /tmp/localshare
VERSION=0.2.0
cp src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk  /tmp/localshare/io.github.fokx.localshare-$VERSION.apk
cp src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/localshare_$VERSION_x64-setup.exe src-tauri/target/release/bundle/deb/localshare_$VERSION_amd64.deb src-tauri/target/release/bundle/rpm/localshare-$VERSION-1.x86_64.rpm src-tauri/target/release/bundle/appimage/localshare_$VERSION_amd64.AppImage /tmp/localshare

```