import * as THREE from 'three';

export function easeOutCubic(t: number): number {
  return 1 - Math.pow(1 - t, 3);
}

export function easeOutElastic(t: number): number {
  const c4 = (2 * Math.PI) / 3;
  return t <= 0 ? 0 : t >= 1 ? 1 : Math.pow(2, -10 * t) * Math.sin((t * 10 - 0.75) * c4) + 1;
}

export function gauss(): number {
  let u = 0, v = 0;
  while (u === 0) u = Math.random();
  while (v === 0) v = Math.random();
  return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v);
}

export function primeHue(n: number, colorLogRange = 6.0, hueMax = 0.78): number {
  const t = Math.min(1, Math.log10(Math.max(n, 2)) / colorLogRange);
  return t * hueMax;
}

export function labelCSS(n: number, colorLogRange = 6.0, hueMax = 0.78): string {
  return `hsl(${primeHue(n, colorLogRange, hueMax) * 360}, 100%, 62%)`;
}

export function ordinal(n: number): string {
  const s = n % 100;
  if (s > 10 && s < 20) return 'th';
  switch (n % 10) {
    case 1: return 'st';
    case 2: return 'nd';
    case 3: return 'rd';
    default: return 'th';
  }
}

export const AXIS_VECS = [
  new THREE.Vector3(1, 0, 0), new THREE.Vector3(-1, 0, 0),
  new THREE.Vector3(0, 1, 0), new THREE.Vector3(0, -1, 0),
  new THREE.Vector3(0, 0, 1), new THREE.Vector3(0, 0, -1),
];

export const FWD_ORDER = [
  new THREE.Vector3(1, 0, 0), new THREE.Vector3(0, 1, 0),
  new THREE.Vector3(0, 0, 1), new THREE.Vector3(-1, 0, 0),
  new THREE.Vector3(0, -1, 0), new THREE.Vector3(0, 0, -1),
];

export function buildOrientations(): { fwd: THREE.Vector3; up: THREE.Vector3 }[] {
  const orientations: { fwd: THREE.Vector3; up: THREE.Vector3 }[] = [];
  for (let upSlot = 0; upSlot < 4; upSlot++) {
    for (const fwd of FWD_ORDER) {
      const ups = AXIS_VECS.filter(a => Math.abs(a.dot(fwd)) < 0.5);
      orientations.push({ fwd: fwd.clone(), up: ups[upSlot].clone() });
    }
  }
  return orientations;
}

export function buildFwdNames(orientations: { fwd: THREE.Vector3; up: THREE.Vector3 }[]): string[] {
  return orientations.map(o => {
    const v = o.fwd;
    if (v.x > 0.5) return '+X';
    if (v.x < -0.5) return '-X';
    if (v.y > 0.5) return '+Y';
    if (v.y < -0.5) return '-Y';
    if (v.z > 0.5) return '+Z';
    return '-Z';
  });
}