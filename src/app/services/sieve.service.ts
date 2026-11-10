import { Injectable } from '@angular/core';
import { CONFIG } from '../config/config';

@Injectable({ providedIn: 'root' })
export class SieveService {
  private sieveLimit = 0;
  private sieve: Uint8Array | null = null;

  constructor() {
    this.buildSieve(CONFIG.INITIAL_SIEVE);
    this.sieveLimit = CONFIG.INITIAL_SIEVE;
  }

  private buildSieve(limit: number): Uint8Array {
    const s = new Uint8Array(limit + 1);
    s.fill(1, 2);
    s[0] = s[1] = 0;
    for (let i = 2; i * i <= limit; i++) {
      if (s[i]) {
        for (let j = i * i; j <= limit; j += i) s[j] = 0;
      }
    }
    return s;
  }

  private extendSieve(old: Uint8Array, oldLimit: number, newLimit: number): Uint8Array {
    const s = new Uint8Array(newLimit + 1);
    s.set(old);
    s.fill(1, oldLimit + 1);
    for (let i = 2; i * i <= newLimit; i++) {
      if (old[i] !== 1) continue;
      let start = Math.max(i * i, oldLimit + 1);
      const r = start % i;
      if (r !== 0) start += i - r;
      for (let j = start; j <= newLimit; j += i) s[j] = 0;
    }
    return s;
  }

  isPrime(n: number): boolean {
    if (n < 2) return false;
    while (n > this.sieveLimit) {
      const nl = this.sieveLimit * 2;
      this.sieve = this.extendSieve(this.sieve!, this.sieveLimit, nl);
      this.sieveLimit = nl;
    }
    return this.sieve![n] === 1;
  }
}