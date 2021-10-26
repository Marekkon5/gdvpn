# GDVPN

VPN to proxy all requests thru Google Drive.
**WARNING: Unfinished and broken, testing**

### Setup

1. Create project at https://console.cloud.google.com
2. Enable Google Drive API
3. Credentials > Create Credentials > OAuth Client ID > Desktop Client
4. Download the .json file into `gdvpn_server/secret.json`
5. Credentials > Create Credentials > OAuth Client ID > Android Client
6. Either create release signing key or use the SHA-1 hash of the debug one located at `$HOME/.android/debug.keystore` 
7. Make sure the package name is same as this project (`com.marekkon5.gdvpn`)
8. Inside `gdvpn/app/src/main/java/com/marekkon5/gdvpn/GDVPN.java` on line 58 change IP and port to your server
9. Compile app and try if Login button works (and authenticate it)
10. Inside `gdvpn_server` copy `settings_example.json` into `settings.json` and change the settings inside (right now only required is `folder_id`)
11. Enable TUN passthru (replace `enp4s0` with eth device on your system)
```
su
echo 1 > /proc/sys/net/ipv4/ip_forward
iptables -t nat -A POSTROUTING -s 10.0.0.0/8 -o enp4s0 -j MASQUERADE
```
12. Compile and run gdvpn_server (`cargo build --release`) (NOTE: Run it as root because of TUN/TAP permissions - `sudo target/release/gdvpn_server`)
13. It will prompt you for Google Drive OAuth, do as it says
14. After all files have been created you can click `Connect` on Android.