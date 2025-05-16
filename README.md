## Localshare
Share your files by starting an HTTP server(using [dufs](https://github.com/sigoden/dufs)).
You can then access them from any device on your network using browser. Support authentication, searching and uploading.
Support Windows, Linux and Android thanks to [tauri](https://github.com/tauri-apps/tauri).

```zsh
pnpm tauri-build-win; pnpm tauri-build; pnpm tauri-build-apk
cd /f/localshare/
mkdir /tmp/localshare
VERSION=0.3.0
cp src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk  /tmp/localshare/io.github.fokx.localshare-${VERSION}.apk
cp src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/localshare_${VERSION}_x64-setup.exe  /tmp/localshare/
cp src-tauri/target/release/bundle/appimage/localshare_${VERSION}_amd64.AppImage /tmp/localshare/
#cp src-tauri/target/release/bundle/deb/localshare_${VERSION}_amd64.deb src-tauri/target/release/bundle/rpm/localshare-${VERSION}-1.x86_64.rpm  /tmp/localshare/

```

rsync /f/tauri-plugin-sharetarget hk:/f/ -av --exclude={"node_modules/*","src-tauri/target/*","src-tauri/gen/android/app/build/*","target/debug/*","target/release/*","android/.tauri/*"}
rsync /f/plugins-workspace hk:/f/ -av --exclude={"node_modules/*","src-tauri/target/*","src-tauri/gen/android/app/build/*","target/debug/*","target/release/*","android/.tauri/*"}
rsync /f/tauri-plugin-android-fs hk:/f/ -av --exclude={"node_modules/*","src-tauri/target/*","src-tauri/gen/android/app/build/*","target/debug/*","target/release/*","android/.tauri/*"}


### Note

currently, to send to a peer, the peer needs to disable encryption in LocalSend setting