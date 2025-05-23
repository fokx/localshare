## Localshare
Share your files by starting an HTTP server(using [dufs](https://github.com/sigoden/dufs)).
You can then access them from any device on your network using browser. Support authentication, searching and uploading.
Support Windows, Linux and Android thanks to [tauri](https://github.com/tauri-apps/tauri).

```zsh
pnpm tauri-build-apk; pnpm tauri-build-win; pnpm tauri-build; 
cd /f/localshare/
rm -r /tmp/localshare
mkdir /tmp/localshare
VERSION=0.5.4
cp src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk  /tmp/localshare/io.github.fokx.localshare-${VERSION}.apk
cp src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/localshare_${VERSION}_x64-setup.exe  /tmp/localshare/
cp src-tauri/target/release/bundle/appimage/localshare_${VERSION}_amd64.AppImage /tmp/localshare/
cp src-tauri/target/release/bundle/deb/localshare_${VERSION}_amd64.deb src-tauri/target/release/bundle/rpm/localshare-${VERSION}-1.x86_64.rpm  /tmp/localshare/

```

rsync /f/tauri-plugin-sharetarget hk:/f/ -av --exclude={"node_modules/*","src-tauri/target/*","src-tauri/gen/android/app/build/*","target/debug/*","target/release/*","android/.tauri/*"}
rsync /f/plugins-workspace hk:/f/ -av --exclude={"node_modules/*","src-tauri/target/*","src-tauri/gen/android/app/build/*","target/debug/*","target/release/*","android/.tauri/*"}
rsync /f/tauri-plugin-android-fs hk:/f/ -av --exclude={"node_modules/*","src-tauri/target/*","src-tauri/gen/android/app/build/*","target/debug/*","target/release/*","android/.tauri/*"}


## Notes
### Cannot use dotenvy to hide credentials in `.env` file
```sh
7z x win-exe-installer
7z x tauri-app.exe
# 3d91 is the start of the password string
grep --text 3d91 .rdata
# this is definitely not expected!
# so hardcode it in the code!
```

.rdata
### Android puple notification bar

To remove the notification bar color in a Tauri Android app, need to modify the Android-specific configuration in the AndroidManifest.xml file and the styles in the res/values/styles.xml file.  
Modify `./gen/android/app/src/main/AndroidManifest.xml`: Ensure that the theme is set correctly for activity.

```
<activity
android:name=".MainActivity"
android:theme="@style/Theme.AppCompat.DayNight.NoActionBar">
<!-- other configurations -->
</activity>
```
alternatives to "Theme.AppCompat.DayNight.NoActionBar":
"Theme.AppCompat.Light.NoActionBar"
"Theme.AppCompat.DayNight"
"Theme.MaterialComponents.DayNight"

see:
https://developer.android.com/develop/ui/views/theming/darktheme


## Troubleshooting
### the proxy client cannot connect to the Internet
The proxy client will prefer IPv6 on the server. Make sure IPv6 works on the server, or disable IPv6.


    "svelte": "https://pkg.pr.new/svelte@async",
    "flowbite-svelte": "^1.4.3",
    "flowbite-svelte": "link:../flowbite-svelte",
