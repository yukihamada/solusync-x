export enum NodeType {
  Master = 'master',
  Replica = 'replica',
  Client = 'client',
}

export enum NetworkQuality {
  Excellent = 'excellent',
  Good = 'good',
  Fair = 'fair',
  Poor = 'poor',
  Critical = 'critical',
}

export interface ClockSample {
  offset: number;
  rtt: number;
  timestamp: number;
}

export interface MediaFrame {
  data: ArrayBuffer;
  timestamp: number;
  duration: number;
  frameType: 'audio' | 'video' | 'video-keyframe';
  sequence: number;
}

export interface SoluSyncConfig {
  serverUrl: string;
  nodeType?: NodeType;
  capabilities?: string[];
  authToken?: string;
  iceServers?: RTCIceServer[];
  clockSyncInterval?: number;
  futureBufferMs?: number;
}

export interface MediaControlParams {
  volume?: number;
  loopCount?: number;
  fadeInMs?: number;
  fadeOutMs?: number;
  seekPosition?: number;
}

export type MediaAction = 'play' | 'pause' | 'stop' | 'seek' | 'load' | 'unload';

export interface Message {
  type: string;
  [key: string]: any;
}

export interface ClockSyncMessage extends Message {
  type: 'clock_sync';
  header: MessageHeader;
  t1: number;
}

export interface ClockSyncResponse extends Message {
  type: 'clock_sync_response';
  header: MessageHeader;
  t1: number;
  t2: number;
  t3: number;
}

export interface MediaControlMessage extends Message {
  type: 'media_control';
  header: MessageHeader;
  action: MediaAction;
  track_id: string;
  start_at: number;
  params: MediaControlParams;
}

export interface MessageHeader {
  id: string;
  timestamp: number;
  node_id: string;
  sequence: number;
}

export interface HelloMessage extends Message {
  type: 'hello';
  header: MessageHeader;
  protocol_version: string;
  capabilities: string[];
  node_type: NodeType;
  auth_token?: string;
}

export interface HeartbeatMessage extends Message {
  type: 'heartbeat';
  header: MessageHeader;
  client_time: number;
  server_time?: number;
}

export interface ErrorMessage extends Message {
  type: 'error';
  header: MessageHeader;
  code: number;
  message: string;
  details?: any;
}