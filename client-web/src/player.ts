import { ClockSync } from './clock';
import { MediaFrame } from './types';

export class FutureAudioPlayer {
  private audioContext?: AudioContext;
  private clockSync: ClockSync;
  private futureBufferMs: number;
  private scheduledSources: Map<number, AudioBufferSourceNode> = new Map();
  private nextSequence: number = 0;

  constructor(clockSync: ClockSync, futureBufferMs: number = 80) {
    this.clockSync = clockSync;
    this.futureBufferMs = futureBufferMs;
  }

  async init(): Promise<void> {
    if (!this.audioContext) {
      this.audioContext = new AudioContext();
    }
  }

  async scheduleFrame(frame: MediaFrame): Promise<void> {
    if (!this.audioContext) {
      await this.init();
    }

    const now = this.clockSync.now();
    const playTime = frame.timestamp;
    const delay = playTime - now;

    if (delay < 0) {
      // Frame is late, drop it
      console.warn(`Dropping late frame: ${-delay * 1000}ms late`);
      return;
    }

    if (delay > 10) {
      // Frame is too far in the future
      console.warn(`Frame too far in future: ${delay * 1000}ms`);
      return;
    }

    // Decode audio data
    const audioBuffer = await this.decodeAudioData(frame.data);
    
    // Schedule playback
    const source = this.audioContext!.createBufferSource();
    source.buffer = audioBuffer;
    source.connect(this.audioContext!.destination);
    
    const contextTime = this.audioContext!.currentTime + delay;
    source.start(contextTime);
    
    // Store reference for cleanup
    this.scheduledSources.set(frame.sequence, source);
    
    // Cleanup after playback
    source.onended = () => {
      this.scheduledSources.delete(frame.sequence);
    };
  }

  async playBuffer(
    data: ArrayBuffer,
    startAt: number,
    volume: number = 1.0
  ): Promise<void> {
    if (!this.audioContext) {
      await this.init();
    }

    const audioBuffer = await this.audioContext!.decodeAudioData(data);
    const source = this.audioContext!.createBufferSource();
    const gainNode = this.audioContext!.createGain();

    source.buffer = audioBuffer;
    source.connect(gainNode);
    gainNode.connect(this.audioContext!.destination);
    gainNode.gain.value = volume;

    const now = this.clockSync.now();
    const delay = Math.max(0, startAt - now);
    const contextTime = this.audioContext!.currentTime + delay;

    source.start(contextTime);
    
    const sequence = this.nextSequence++;
    this.scheduledSources.set(sequence, source);
    
    source.onended = () => {
      this.scheduledSources.delete(sequence);
    };
  }

  stop(): void {
    // Stop all scheduled sources
    for (const source of this.scheduledSources.values()) {
      try {
        source.stop();
      } catch (e) {
        // Source might have already stopped
      }
    }
    this.scheduledSources.clear();
  }

  setFutureBufferMs(ms: number): void {
    this.futureBufferMs = ms;
  }

  getFutureBufferMs(): number {
    return this.futureBufferMs;
  }

  private async decodeAudioData(data: ArrayBuffer): Promise<AudioBuffer> {
    if (!this.audioContext) {
      throw new Error('Audio context not initialized');
    }

    // For raw PCM data, we need to create an AudioBuffer manually
    // This assumes 48kHz, 16-bit PCM, stereo
    const sampleRate = 48000;
    const channels = 2;
    const bitsPerSample = 16;
    
    const samples = data.byteLength / (channels * bitsPerSample / 8);
    const audioBuffer = this.audioContext.createBuffer(
      channels,
      samples,
      sampleRate
    );

    // Convert PCM16 to float32
    const dataView = new DataView(data);
    for (let channel = 0; channel < channels; channel++) {
      const channelData = audioBuffer.getChannelData(channel);
      for (let i = 0; i < samples; i++) {
        const offset = (i * channels + channel) * 2;
        const sample = dataView.getInt16(offset, true) / 32768;
        channelData[i] = sample;
      }
    }

    return audioBuffer;
  }

  // Utility method for Opus decoding (requires additional library)
  async decodeOpus(_data: ArrayBuffer): Promise<AudioBuffer> {
    // This would require an Opus decoder library like libopus.js
    throw new Error('Opus decoding not implemented yet');
  }
}