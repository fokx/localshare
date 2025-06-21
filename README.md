## Localshare
Share your files by starting an HTTP server(using [dufs](https://github.com/sigoden/dufs)).
You can then access them from any device on your network using browser. Support authentication, searching and uploading.
Support Windows, Linux and Android thanks to [tauri](https://github.com/tauri-apps/tauri).

```zsh
cd /f/localshare/
pnpm i
#rm -r ./src-tauri/gen/android/app/src/main/assets/    
#rm src-tauri/gen/android/app/build -r
#rm src-tauri/target -r
rm -r /tmp/localshare
mkdir /tmp/localshare
pnpm tauri-build-apk; 
VERSION=0.6.4
cp src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk  /tmp/localshare/io.github.fokx.localshare-${VERSION}.apk
pnpm tauri-build-win;
cp src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/localshare_${VERSION}_x64-setup.exe  /tmp/localshare/
pnpm tauri-build; 
cp src-tauri/target/release/bundle/deb/localshare_${VERSION}_amd64.deb  /tmp/localshare/
#cp src-tauri/target/release/bundle/appimage/localshare_${VERSION}_amd64.AppImage /tmp/localshare/
cp src-tauri/target/release/bundle/rpm/localshare-${VERSION}-1.x86_64.rpm   /tmp/localshare/

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

for ERR_CLEARTEXT_NOT_PERMITTED on Android, set 
```xml
    <application
        android:icon="@mipmap/ic_launcher"
        android:label="@string/app_name"
        android:theme="@style/Theme.localshare"
        android:usesCleartextTraffic="true">
    </application>
```
in  ./src-tauri/gen/android/app/src/main/AndroidManifest.xml

`cargo tauri android dev`
will show logging >= info!

`pnpm tauri android dev`
will not



Localsend calculates sha256 of cert's der bytes as fingerprint
`ip.dst_host==224.0.0.167` got fingerprint,
download https certificate pem from https share link, convert pem to der using:
https://www.sslshopper.com/ssl-converter.html
then sha256 *.der gives the same fingerprint
Do not sha256 pem text.


rsync /f/tuic/tuic-client/src/ /f/localshare/src-tauri/src/tuicc/ -av
cd /f/localshare/src-tauri/src/tuicc/

replace in directory: crate:: -> crate::tuicc::

mv main.rs mod.rs

modify mod.rs:
#[tokio::main]
async fn main() {
->
pub async fn main() {

remove:
LoggerBuilder
rustls::crypto::ring::default_provider().install_default()

cp /f/tuic/.env /f/localshare/src-tauri


rsync /f/tuic/tuic-client/src/ /f/localshare/src-tauri/src/tuicc/ -av --delete --exclude={"/mod.rs","/main.rs"}


listen on localhost:4810 for result,
user use distrust to do oauth, 
will callback localhost:4810,
should return username, email, etc.
(+what if it also returns Api-Key, which can be pre-generated for all users / at registration)

safety: 
* Is hardcoding client_secret in user-side app safe?

https://oidc.xjtu.app
http://192.168.174.93:3010


## what do I want to achieve in chat? (why use this app over existing IM app)
* visually prevent sending messages to the wrong person / group 
by automatically using different background colors for different chat
* support chatting with peers in the local network (without an external / central server)
* support chatting with peers via Bluetooth
* automatic message synchronization across devices
* export messages
* support sending audio messages and automatic transcription
* support P2P video / audio calling
* support group video / audio meeting
* support chat thread