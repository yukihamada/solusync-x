# SOLUSync-X

次世代音楽フェス向け超低遅延同期プロトコル

## 🌕 概要

SOLUSync-Xは、音楽フェスやライブイベントで数百〜数千台の端末を**±0.5ms以内**で同期させる革新的なプロトコルです。音声だけでなく、映像・照明・スマートフォンUIまで、すべてを1つの時間軸で制御します。

### ✨ 特徴

- **超低遅延同期**: PTP風アルゴリズム + カルマンフィルタで±0.5ms精度
- **動的バッファ管理**: ネットワーク状態に応じて30〜250msで自動調整
- **自己修復クラスタ**: マスター障害時の自動フェイルオーバー
- **マルチメディア対応**: 音声(Opus/PCM)、映像(H.264/VP9)、DMX照明制御
- **スケーラビリティ**: 1000台以上の同時接続に対応

## 🚀 クイックスタート

### サーバー起動（Rust）

```bash
cd server
cargo build --release
cargo run --release
```

サーバーは`http://localhost:8080`で起動します。

### Webクライアント（TypeScript）

```bash
cd client-web
npm install
npm run build
```

### 使用例

```typescript
import { SoluSyncClient } from 'solusync-x-client';

// クライアント作成
const client = new SoluSyncClient({
  serverUrl: 'http://localhost:8080',
  futureBufferMs: 80
});

// 接続
await client.connect();

// 時刻同期確認
console.log('Current time:', client.getCurrentTime());
console.log('Clock offset:', client.getClockOffset(), 'ms');

// 音声再生（全端末で同時）
await client.play('opening_track', client.getCurrentTime() + 1.0);
```

## 📡 アーキテクチャ

```
┌─────────────────┐     WebSocket/QUIC    ┌─────────────────┐
│  Master Server  │◄─────────────────────►│     Client      │
│  (Rust/Axum)    │                       │ (Web/iOS/Android)│
├─────────────────┤     WebRTC Media      ├─────────────────┤
│ • Clock Sync    │◄─────────────────────►│ • Future Buffer │
│ • Media Server  │                       │ • Clock Sync    │
│ • Cluster Mgmt  │      UDP/SRTP        │ • Audio Player  │
└─────────────────┘                       └─────────────────┘
```

### コンポーネント

1. **Clock Manager**: PTPライクな時刻同期
2. **Media Server**: WebRTC SFUによるストリーミング
3. **Control Server**: WebSocketによる制御API
4. **Future Buffer**: 動的遅延調整バッファ

## 🎯 ユースケース

### 音楽フェス
- 数千人が持つスマホから同時に音を鳴らす
- ステージ照明とスマホ画面を完全同期
- 遅延なしのサイレントディスコ

### ゴルフ場イベント
- 各カートのスピーカーを同期再生
- プレイヤー位置に応じた音響効果
- リアルタイム実況の同時配信

### 森林イベント
- 電波が弱い環境でも安定動作
- バッテリー効率を考慮した制御
- オフライングレースフル対応

## 🔧 技術仕様

### サーバー要件
- CPU: 4コア以上
- メモリ: 8GB以上
- OS: Linux推奨（macOS/Windows対応）

### プロトコル
- 時刻同期: PTP over WebSocket/QUIC
- メディア: WebRTC (Opus 48kHz)
- 制御: JSON over WebSocket
- セキュリティ: TLS 1.3, DTLS

### パフォーマンス
- 同期精度: ±0.5ms (LAN), ±2ms (WiFi)
- 遅延: 30〜250ms（自動調整）
- 同時接続: 1000+ クライアント

## 📚 API仕様

詳細は[プロトコル仕様書](docs/SPEC_v0.1.md)を参照してください。

### 主要API

```typescript
// 再生制御
client.play(trackId: string, startAt?: number)
client.pause(trackId: string)
client.stop(trackId: string)

// 時刻同期
client.getCurrentTime(): number
client.getClockOffset(): number

// ネットワーク状態
client.getNetworkQuality(): NetworkQuality
```

## 🛠 開発

### ビルド

```bash
# サーバー
cd server
cargo build

# クライアント
cd client-web
npm run build
```

### テスト

```bash
# サーバー
cargo test

# クライアント
npm test
```

## 📈 今後の展開

- v0.2: QUIC完全対応、マルチキャスト
- v0.3: 映像ストリーミング、DMX照明制御
- v0.4: E2E暗号化、録画再生機能
- v1.0: AIによる遅延予測と事前補正

## 📄 ライセンス

MIT License

---

**SOLUSync-X** - 音楽フェスの体験を次のレベルへ 🎵🌕