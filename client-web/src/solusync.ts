import { EventEmitter } from 'eventemitter3';
import { ClockSync } from './clock';
import { FutureAudioPlayer } from './player';
import {
  SoluSyncConfig,
  NodeType,
  NetworkQuality,
  Message,
  HelloMessage,
  HeartbeatMessage,
  MediaControlMessage,
  MediaAction,
  MediaControlParams,
} from './types';

export class SoluSyncClient extends EventEmitter {
  private config: Required<SoluSyncConfig>;
  private ws?: WebSocket;
  private pc?: RTCPeerConnection;
  private clockSync: ClockSync;
  private audioPlayer: FutureAudioPlayer;
  private nodeId: string;
  private sequence: number = 0;
  private heartbeatInterval?: number;
  private clockSyncInterval?: number;
  private connected: boolean = false;

  constructor(config: SoluSyncConfig) {
    super();
    
    this.config = {
      nodeType: NodeType.Client,
      capabilities: ['audio', 'clock_sync'],
      iceServers: [{ urls: 'stun:stun.l.google.com:19302' }],
      clockSyncInterval: 1000,
      futureBufferMs: 80,
      ...config,
    };
    
    this.nodeId = this.generateNodeId();
    this.clockSync = new ClockSync();
    this.audioPlayer = new FutureAudioPlayer(
      this.clockSync,
      this.config.futureBufferMs
    );
  }

  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      const wsUrl = this.config.serverUrl.replace('http', 'ws') + '/ws';
      this.ws = new WebSocket(wsUrl);
      
      this.ws.onopen = () => {
        this.connected = true;
        this.sendHello();
        this.startHeartbeat();
        this.startClockSync();
        this.emit('connected');
        resolve();
      };
      
      this.ws.onmessage = (event) => {
        this.handleMessage(event.data);
      };
      
      this.ws.onerror = (error) => {
        this.emit('error', error);
        reject(error);
      };
      
      this.ws.onclose = () => {
        this.connected = false;
        this.stopHeartbeat();
        this.stopClockSync();
        this.emit('disconnected');
      };
    });
  }

  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = undefined;
    }
    if (this.pc) {
      this.pc.close();
      this.pc = undefined;
    }
    this.connected = false;
  }

  async play(
    trackId: string,
    startAt?: number,
    params: MediaControlParams = {}
  ): Promise<void> {
    const message: MediaControlMessage = {
      type: 'media_control',
      header: this.createHeader(),
      action: 'play',
      track_id: trackId,
      start_at: startAt || this.clockSync.now() + 0.1,
      params,
    };
    
    this.send(message);
  }

  async pause(trackId: string): Promise<void> {
    const message: MediaControlMessage = {
      type: 'media_control',
      header: this.createHeader(),
      action: 'pause',
      track_id: trackId,
      start_at: this.clockSync.now(),
      params: {},
    };
    
    this.send(message);
  }

  async stop(trackId: string): Promise<void> {
    const message: MediaControlMessage = {
      type: 'media_control',
      header: this.createHeader(),
      action: 'stop',
      track_id: trackId,
      start_at: this.clockSync.now(),
      params: {},
    };
    
    this.send(message);
  }

  getCurrentTime(): number {
    return this.clockSync.now();
  }

  getClockOffset(): number {
    return this.clockSync.getOffset();
  }

  getNetworkQuality(): NetworkQuality {
    const rtt = this.clockSync.getLastRTT();
    if (rtt < 10) return NetworkQuality.Excellent;
    if (rtt < 50) return NetworkQuality.Good;
    if (rtt < 100) return NetworkQuality.Fair;
    if (rtt < 200) return NetworkQuality.Poor;
    return NetworkQuality.Critical;
  }

  private handleMessage(data: string): void {
    try {
      const message: Message = JSON.parse(data);
      
      switch (message.type) {
        case 'hello':
          this.handleHello(message as HelloMessage);
          break;
          
        case 'clock_sync_response':
          this.clockSync.handleResponse(message);
          break;
          
        case 'heartbeat':
          this.handleHeartbeat(message as HeartbeatMessage);
          break;
          
        case 'error':
          this.emit('error', message);
          break;
          
        default:
          this.emit('message', message);
      }
    } catch (error) {
      console.error('Failed to handle message:', error);
    }
  }

  private sendHello(): void {
    const message: HelloMessage = {
      type: 'hello',
      header: this.createHeader(),
      protocol_version: '0.1.0',
      capabilities: this.config.capabilities!,
      node_type: this.config.nodeType!,
      auth_token: this.config.authToken,
    };
    
    this.send(message);
  }

  private handleHello(message: HelloMessage): void {
    console.log('Server hello received:', message);
    this.emit('ready');
  }

  private handleHeartbeat(message: HeartbeatMessage): void {
    if (message.server_time) {
      // Update clock offset estimate
      const rtt = Date.now() / 1000 - message.client_time;
      const offset = message.server_time - Date.now() / 1000 + rtt / 2;
      this.clockSync.updateQuickSample(offset, rtt);
    }
  }

  private startHeartbeat(): void {
    this.heartbeatInterval = window.setInterval(() => {
      if (this.connected) {
        const message: HeartbeatMessage = {
          type: 'heartbeat',
          header: this.createHeader(),
          client_time: Date.now() / 1000,
        };
        this.send(message);
      }
    }, 5000);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = undefined;
    }
  }

  private startClockSync(): void {
    this.clockSyncInterval = window.setInterval(() => {
      if (this.connected) {
        this.clockSync.sendSync(this);
      }
    }, this.config.clockSyncInterval);
  }

  private stopClockSync(): void {
    if (this.clockSyncInterval) {
      clearInterval(this.clockSyncInterval);
      this.clockSyncInterval = undefined;
    }
  }

  private send(message: Message): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }

  public _send(message: Message): void {
    this.send(message);
  }

  private createHeader() {
    return {
      id: this.generateId(),
      timestamp: Date.now() / 1000,
      node_id: this.nodeId,
      sequence: this.sequence++,
    };
  }

  private generateNodeId(): string {
    return 'client-' + Math.random().toString(36).substr(2, 9);
  }

  private generateId(): string {
    return Math.random().toString(36).substr(2, 9);
  }
}