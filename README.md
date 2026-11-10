# Prime Pipes - Angular WebGL Visualization

A stunning 3D WebGL visualization of prime numbers as a growing spiral of neon pipes. Built with Angular 17+ and Three.js.

## Features

- **Prime Spiral Generation**: 3D spiral that grows at each step, turns at prime numbers
- **Neon Pipe Shaders**: Custom GLSL shaders with emissive cores, Fresnel rims, and breathing pulses
- **Inner Core Glow**: Thinner glowing cylinder inside each pipe for authentic neon tube look
- **Prime Milestones**: Celebration toasts at 1st, 10th, 100th, 1000th, 10000th, 100000th primes
- **Energy Arcs**: Connecting lines between consecutive prime joints
- **Traveling Pulses**: Glowing orbs that race along pipes from each prime junction
- **Burst Particles**: Explosion effects at milestone primes
- **Cinematic Post-Processing**: Color grading, motion blur, chromatic aberration, film grain
- **Dynamic Background**: Stars, Milky Way, procedural galaxy, nebulae, central energy core
- **Responsive Controls**: WASD + Space/Q navigation, Shift boost, P pause, F fit, R reset, L labels

## Controls

| Key | Action |
|-----|--------|
| WASD | Fly |
| Space / E | Ascend |
| Q | Descend |
| Shift | Boost |
| P | Pause |
| F | Fit camera |
| R | Restart |
| L | Toggle labels |

## Quick Start

```bash
npm install
npm start
```

Then open http://localhost:4200

## Build

```bash
npm run build
```

Output in `dist/prime-pipes/`

## Tech Stack

- Angular 17+ (standalone components)
- Three.js (WebGL)
- Custom GLSL shaders
- CSS2D for labels
- Zone.js for smooth animations