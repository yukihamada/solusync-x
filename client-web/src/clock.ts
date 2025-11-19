import { ClockSample, ClockSyncMessage } from './types';

export class ClockSync {
  private offset: number = 0;
  private drift: number = 0;
  private samples: ClockSample[] = [];
  private maxSamples: number = 20;
  private lastSyncTime: number = 0;
  private filterWeight: number = 0.1;

  constructor() {}

  now(): number {
    const localTime = Date.now() / 1000;
    return localTime + this.offset + this.drift * (localTime - this.lastSyncTime);
  }

  getOffset(): number {
    return this.offset;
  }

  getLastRTT(): number {
    if (this.samples.length === 0) return 0;
    return this.samples[this.samples.length - 1].rtt * 1000; // Convert to ms
  }

  sendSync(client: any): void {
    const t1 = Date.now() / 1000;
    
    const message: ClockSyncMessage = {
      type: 'clock_sync',
      header: {
        id: Math.random().toString(36).substr(2, 9),
        timestamp: t1,
        node_id: 'client',
        sequence: 0,
      },
      t1,
    };
    
    // Store t1 for later use
    (window as any).__lastSyncT1 = t1;
    
    client._send(message);
  }

  handleResponse(response: any): void {
    const t1 = (window as any).__lastSyncT1 || response.t1;
    const t4 = Date.now() / 1000;
    
    // Calculate offset and RTT using NTP algorithm
    const rtt = (t4 - t1) - (response.t3 - response.t2);
    const offset = ((response.t2 - t1) + (response.t3 - t4)) / 2;
    
    const sample: ClockSample = {
      offset,
      rtt,
      timestamp: t4,
    };
    
    this.addSample(sample);
  }

  updateQuickSample(offset: number, rtt: number): void {
    const sample: ClockSample = {
      offset,
      rtt,
      timestamp: Date.now() / 1000,
    };
    
    this.addSample(sample);
  }

  private addSample(sample: ClockSample): void {
    this.samples.push(sample);
    
    // Keep only recent samples
    if (this.samples.length > this.maxSamples) {
      this.samples = this.samples.slice(-this.maxSamples);
    }
    
    // Update offset using exponential moving average
    this.offset = this.offset * (1 - this.filterWeight) + sample.offset * this.filterWeight;
    
    // Calculate drift if we have enough samples
    if (this.samples.length >= 3) {
      this.calculateDrift();
    }
    
    this.lastSyncTime = sample.timestamp;
  }

  private calculateDrift(): void {
    if (this.samples.length < 3) return;
    
    // Simple linear regression to estimate drift
    const n = Math.min(10, this.samples.length);
    const recent = this.samples.slice(-n);
    
    let sumX = 0, sumY = 0, sumXY = 0, sumXX = 0;
    const t0 = recent[0].timestamp;
    
    for (const sample of recent) {
      const x = sample.timestamp - t0;
      const y = sample.offset;
      sumX += x;
      sumY += y;
      sumXY += x * y;
      sumXX += x * x;
    }
    
    const denom = n * sumXX - sumX * sumX;
    if (Math.abs(denom) > 0.0001) {
      this.drift = (n * sumXY - sumX * sumY) / denom;
    }
  }

  reset(): void {
    this.offset = 0;
    this.drift = 0;
    this.samples = [];
    this.lastSyncTime = 0;
  }
}