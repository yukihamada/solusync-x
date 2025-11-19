# SOLUSync-X Protocol Specification v0.1

## 概要

SOLUSync-Xは、音楽フェスやライブイベントにおいて、数百〜数千台の端末間で音声・映像・照明・UI要素を完全に同期させるための次世代プロトコルです。

### 主要特性

- **超低遅延同期**: ±0.5ms以内の音声同期精度
- **スケーラビリティ**: 1000台以上の端末に対応
- **適応型バッファリング**: ネットワーク状態に応じた動的調整
- **自己修復型クラスタ**: マスターノード障害時の自動フェイルオーバー
- **マルチメディア対応**: 音声・映像・照明・スマートフォンUIの統合制御

## アーキテクチャ

### ノードタイプ

1. **Master Node**: 時刻基準とメディア配信を管理
2. **Replica Node**: Masterのバックアップ（自動昇格可能）
3. **Client Node**: エンドユーザー端末

### 通信チャネル

- **メディアチャネル**: WebRTC (UDP/SRTP)
- **制御チャネル**: WebSocket (TLS)
- **時刻同期チャネル**: QUIC または WebSocket

## プロトコルメッセージ

### メッセージ構造

全てのメッセージは以下のヘッダーを持ちます：

```json
{
  "header": {
    "id": "uuid-v4",
    "timestamp": 123456.789,  // UNIX時刻（マイクロ秒精度）
    "node_id": "uuid-v4",
    "sequence": 12345
  },
  "type": "message_type",
  ...
}
```

### 1. 接続確立

#### Hello (Client → Server)

```json
{
  "type": "hello",
  "header": {...},
  "protocol_version": "0.1.0",
  "capabilities": ["audio", "video", "clock_sync"],
  "node_type": "client",
  "auth_token": "optional-jwt-token"
}
```

#### Hello Response (Server → Client)

```json
{
  "type": "hello",
  "header": {...},
  "protocol_version": "0.1.0",
  "capabilities": ["audio", "video", "clock_sync", "cluster"],
  "node_type": "master",
  "cluster_info": {
    "master_id": "uuid",
    "replica_ids": ["uuid1", "uuid2"]
  }
}
```

### 2. 時刻同期

PTP/NTPアルゴリズムに基づく4段階同期：

#### Clock Sync Request (Client → Server)

```json
{
  "type": "clock_sync",
  "header": {...},
  "t1": 123456.789  // クライアント送信時刻
}
```

#### Clock Sync Response (Server → Client)

```json
{
  "type": "clock_sync_response",
  "header": {...},
  "t1": 123456.789,  // 元のクライアント時刻
  "t2": 123456.890,  // サーバー受信時刻
  "t3": 123456.891   // サーバー送信時刻
}
```

クライアントは受信時刻t4を記録し、以下の計算を行います：
- RTT = (t4 - t1) - (t3 - t2)
- offset = ((t2 - t1) + (t3 - t4)) / 2

### 3. メディア制御

#### Media Control (Client → Server or Server → Client)

```json
{
  "type": "media_control",
  "header": {...},
  "action": "play",  // play, pause, stop, seek, load, unload
  "track_id": "track_001",
  "start_at": 234567.000,  // ネットワーク時刻での開始時間
  "params": {
    "volume": 0.8,
    "loop_count": 1,
    "fade_in_ms": 100,
    "fade_out_ms": 200
  }
}
```

### 4. メディアデータ

WebRTC DataChannelまたはMediaStreamで送信：

```json
{
  "type": "media_data",
  "header": {...},
  "track_id": "track_001",
  "chunk_index": 42,
  "timestamp": 234567.100,  // プレゼンテーションタイムスタンプ
  "duration": 0.020,        // 20ms
  "codec": "opus",          // opus, pcm16, h264, vp9
  "data": "base64_encoded_data",
  "is_keyframe": false
}
```

### 5. クラスタ管理

#### Node Status (定期的にブロードキャスト)

```json
{
  "type": "node_status",
  "header": {...},
  "node_type": "master",
  "connected_clients": 245,
  "cpu_usage": 0.45,
  "memory_usage": 0.62,
  "battery_level": 0.88,  // モバイル端末の場合
  "network_quality": "good",
  "avg_rtt_ms": 23.5,
  "packet_loss_percent": 0.02
}
```

#### Master Election (障害時の自動選出)

```json
{
  "type": "master_election",
  "header": {...},
  "election_id": "uuid",
  "candidate_score": 0.95,  // 適性スコア
  "current_master": "uuid-or-null"
}
```

## 動的バッファ管理

### ネットワーク品質レベル

| Quality | RTT | Packet Loss | Buffer Size |
|---------|-----|-------------|-------------|
| Excellent | < 10ms | 0% | 30ms |
| Good | < 50ms | < 0.1% | 80ms |
| Fair | < 100ms | < 1% | 120ms |
| Poor | < 200ms | < 5% | 180ms |
| Critical | > 200ms | > 5% | 250ms |

### 適応アルゴリズム

1. RTTとパケットロスを200ms間隔で測定
2. カルマンフィルタで平滑化
3. バッファサイズを段階的に調整（10%/秒の変化率）
4. アンダーラン検出時は即座に20%増加
5. 安定期間が続けば徐々に減少

## セキュリティ

### 暗号化

- 制御チャネル: TLS 1.3
- メディアチャネル: DTLS-SRTP
- 認証: JWT (RS256)

### レート制限

- クロック同期: 最大10回/秒
- メディア制御: 最大100回/秒
- 接続数: IPあたり最大10接続

## 実装要件

### サーバー要件

- CPU: 4コア以上
- メモリ: 8GB以上
- ネットワーク: 1Gbps以上
- OS: Linux (推奨) / macOS / Windows

### クライアント要件

- ブラウザ: Chrome 90+, Safari 14+, Firefox 88+
- モバイル: iOS 14+, Android 8+
- Web Audio API対応
- WebRTC対応

## 今後の拡張予定 (v0.2以降)

- QUIC transport実装
- マルチキャストサポート
- E2E暗号化オプション
- 録画・再生機能
- AIによる遅延予測と事前補正