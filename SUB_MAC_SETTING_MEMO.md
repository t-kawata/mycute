# FileVault を切る（ログイン前にfrpが起動できるようにするため）
    1. 「設定」
    2. 「プライバシーとセキュリティ」
    3. 「FileVault」 -> オフ

# VNCとリモートログインの設定 + ロックなし設定
    1. 「設定」>「一般」>「共有」
    2. 「画面共有」-> ON -> 「アクセス許可」-> 全てのユーザ
    3. 「リモートログイン」-> ON -> 「アクセス許可」-> 全てのユーザ
    4. 「設定」>「ロック画面」-> 「しない」に設定

# frp インストール

## ダウンロードと frpc 設定
- `remotePort` を適宜書き換えること
```
mkdir -m 755 -p ~/shyme
cd ~/shyme
wget https://github.com/fatedier/frp/releases/download/v0.67.0/frp_0.67.0_darwin_arm64.tar.gz
tar vxzf frp_0.67.0_darwin_arm64.tar.gz
rm frp_0.67.0_darwin_arm64.tar.gz
mv frp_0.67.0_darwin_arm64 frp
cd frp

cat <<EOF > ~/shyme/frp/frpc.toml
serverAddr = "p00-ap001a.shyme.net"
serverPort = 7000
loginFailExit = false

[[proxies]]
name = "ssh-$(hostname)"
type = "tcp"
localIP = "127.0.0.1"
localPort = 22
remotePort = 50002

[[proxies]]
name = "vnc-$(hostname)"
type = "tcp"
localIP = "127.0.0.1"
localPort = 5900
remotePort = 59902
EOF
```

## デーモン化
```
cat <<EOF | sudo tee /Library/LaunchDaemons/net.shyme.frp.plist
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>net.shyme.frp</string>
    <key>ProgramArguments</key>
    <array>
        <string>$HOME/shyme/frp/frpc</string>
        <string>-c</string>
        <string>$HOME/shyme/frp/frpc.toml</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>NetworkState</key>
        <true/>
    </dict>
    <key>WorkingDirectory</key>
    <string>$HOME/shyme/frp</string>
    <key>StandardOutPath</key>
    <string>/var/log/net.shyme.frp.out.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/net.shyme.frp.err.log</string>
</dict>
</plist>
EOF
sudo launchctl bootout system /Library/LaunchDaemons/net.shyme.frp.plist 2>/dev/null
sudo launchctl bootstrap system /Library/LaunchDaemons/net.shyme.frp.plist
sudo launchctl enable system/net.shyme.frp

sudo tee /etc/newsyslog.d/net.shyme.frp.conf >/dev/null <<'EOF'
# logfile                          mode  count  size  when  flags
/var/log/net.shyme.frp.out.log     640   7      *     @T00  J
/var/log/net.shyme.frp.err.log     640   7      *     @T00  J
EOF

cat <<'EOF' > ~/shyme/frp/restart.sh
#!/bin/bash
set -e
PLIST="/Library/LaunchDaemons/net.shyme.frp.plist"
LABEL="net.shyme.frp"
# 一旦落とす（存在しなくても無視）
sudo launchctl bootout system "$PLIST" 2>/dev/null || true
sleep 1
# 再登録して起動
sudo launchctl bootstrap system "$PLIST"
sudo launchctl enable system/"$LABEL"
sleep 1
# frpc がサーバに接続しにいっているか確認（7000 に対して）
netstat -an | grep 7000 || true
exit 0
EOF
chmod 755 ~/shyme/frp/restart.sh

sudo mkdir -p /usr/local/bin
cat <<EOF | sudo tee /usr/local/bin/frpremote && sudo chmod 755 /usr/local/bin/frpremote && source ~/.zshrc
#!/bin/bash
cat \${HOME}/shyme/frp/frpc.toml | grep 'serverAddr =' | awk -F\\" '{print \$2}'
EOF
```
