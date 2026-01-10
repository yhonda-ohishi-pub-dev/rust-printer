# ローカル CUPS インストールによる印刷テスト計画

## 目的
Docker 内 CUPS の問題か、プリンター本体の問題かを切り分けるため、ローカルに CUPS をインストールして直接印刷テストを行う。

## 前提
- プリンター: Canon LBP221 (IP: 172.18.21.60)
- Docker CUPS では LIPSLX ドライバーで `Broken pipe`、RAW で印刷不可
- Ubuntu ローカル環境

---

## 手順

### 1. ローカル CUPS インストール

```bash
sudo apt update
sudo apt install cups cups-client cups-bsd
```

### 2. CUPS サービス起動確認

```bash
sudo systemctl start cups
sudo systemctl status cups
```

### 3. プリンター登録（まずは汎用ドライバーで）

```bash
# ネットワーク到達性確認
ping -c 3 172.18.21.60

# socket プロトコルで登録（RAW）
sudo lpadmin -p Canon_LBP221_Test -E -v socket://172.18.21.60:9100 -m raw
sudo lpadmin -d Canon_LBP221_Test
```

### 4. 簡単な印刷テスト

```bash
# テストページ印刷
echo "Hello from Linux" | lp -d Canon_LBP221_Test

# または PostScript ファイルで
cat > /tmp/test.ps << 'EOF'
%!PS
/Helvetica findfont 24 scalefont setfont
100 700 moveto (Hello Printer Test) show
showpage
EOF
lp -d Canon_LBP221_Test /tmp/test.ps
```

### 5. 結果確認

```bash
# ジョブ状態
lpstat -t

# エラーログ
sudo tail -50 /var/log/cups/error_log
```

---

## 結果による次のアクション

| ローカル CUPS 結果 | 判断 | 次のアクション |
|-----------------|------|--------------|
| 印刷成功 | Docker CUPS に問題あり | Docker 設定見直し |
| 印刷失敗（同じエラー） | プリンター側の問題 | プリンター本体設定確認 |
| 接続エラー | ネットワーク問題 | ファイアウォール/ネットワーク確認 |

---

## 追加テスト（必要に応じて）

### Canon LIPSLX ドライバーをローカルにもインストール
```bash
# ダウンロード済みの場合
sudo dpkg -i /tmp/linux-lipslx-drv-v620-jp/x64/Debian/cnrdrvcups-lipslx_6.20-1.02_amd64.deb
sudo lpadmin -p Canon_LBP221_LIPSLX -E -v socket://172.18.21.60:9100 -m CNRCUPSLBP221ZJ.ppd
```

### IPP Everywhere 確認
```bash
# プリンターの IPP サービス検出
avahi-browse -art | grep -i canon
# または
ippfind
```
