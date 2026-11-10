import * as THREE from 'three';

export function easeOutCubic(t: number) { return 1 - Math.pow(1 - t, 3); }
export function easeOutElastic(t: number) { const c4 = (2 * Math.PI) / 3; return t <= 0 ? 0 : t >= 1 ? 1 : Math.pow(2, -10 * t) * Math.sin((t * 10 - 0.75) * c4) + 1; }
export function gauss(): number { let u = 0, v = 0; while (u === 0) u = Math.random(); while (v === 0) v = Math.random(); return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v); }
export function primeHue(n: number, range = 6.0, max = 0.78) { return Math.min(1, Math.log10(Math.max(n, 2)) / range) * max; }
export function ordinal(n: number) { const s = n % 100; if (s > 10 && s < 20) return 'th'; return ['th', 'st', 'nd', 'rd'][n % 10] || 'th'; }

export const AXIS = [new THREE.Vector3(1,0,0), new THREE.Vector3(-1,0,0), new THREE.Vector3(0,1,0), new THREE.Vector3(0,-1,0), new THREE.Vector3(0,0,1), new THREE.Vector3(0,0,-1)];
export const FWD = [new THREE.Vector3(1,0,0), new THREE.Vector3(0,1,0), new THREE.Vector3(0,0,1), new THREE.Vector3(-1,0,0), new THREE.Vector3(0,-1,0), new THREE.Vector3(0,0,-1)];

export function buildOrientations() { const o = []; for (let u = 0; u < 4; u++) for (const f of FWD) { const ups = AXIS.filter(a => Math.abs(a.dot(f)) < 0.5); o.push({ fwd: f.clone(), up: ups[u].clone() }); } return o; }
export const ORIENTATIONS = buildOrientations();
export const FWD_NAMES = ORIENTATIONS.map(o => { const v = o.fwd; if (v.x > 0.5) return '+X'; if (v.x < -0.5) return '-X'; if (v.y > 0.5) return '+Y'; if (v.y < -0.5) return '-Y'; if (v.z > 0.5) return '+Z'; return '-Z'; });