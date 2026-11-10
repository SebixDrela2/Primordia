import { Injectable } from '@angular/core';
import { CONFIG } from './config';

@Injectable({ providedIn: 'root' })
export class SieveService {
  private limit = 0;
  private sieve: Uint8Array | null = null;
  constructor() { this.build(CONFIG.INITIAL_SIEVE); this.limit = CONFIG.INITIAL_SIEVE; }
  private build(n: number) { const s = new Uint8Array(n + 1); s.fill(1, 2); s[0] = s[1] = 0; for (let i = 2; i * i <= n; i++) if (s[i]) for (let j = i * i; j <= n; j += i) s[j] = 0; return s; }
  private extend(old: Uint8Array, oLim: number, nLim: number) { const s = new Uint8Array(nLim + 1); s.set(old); s.fill(1, oLim + 1); for (let i = 2; i * i <= nLim; i++) { if (old[i] !== 1) continue; let st = Math.max(i * i, oLim + 1); const r = st % i; if (r) st += i - r; for (let j = st; j <= nLim; j += i) s[j] = 0; } return s; }
  isPrime(n: number) { if (n < 2) return false; while (n > this.limit) { this.sieve = this.extend(this.sieve!, this.limit, this.limit * 2); this.limit *= 2; } return this.sieve![n] === 1; }
}