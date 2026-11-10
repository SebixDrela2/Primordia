import { CONFIG } from '../config/config';

export const CHROMATIC_GRAIN_SHADER = {
  uniforms: {
    tDiffuse: { value: null },
    uAmount: { value: 0.0030 },
    uTime: { value: 0 },
    uGrain: { value: 0.018 },
  },
  vertexShader: `varying vec2 vUv;
    void main(){vUv=uv;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.0);}`,
  fragmentShader: `
    uniform sampler2D tDiffuse;
    uniform float uAmount,uTime,uGrain;
    varying vec2 vUv;
    float rand(vec2 co){return fract(sin(dot(co,vec2(12.9898,78.233)))*43758.5453);}
    void main(){
      vec2 dir=vUv-vec2(0.5); float dist=length(dir);
      float amt=uAmount*dist*dist; vec2 rOff=dir*amt;
      vec4 r=texture2D(tDiffuse,vUv+rOff);
      vec4 g=texture2D(tDiffuse,vUv);
      vec4 b=texture2D(tDiffuse,vUv-rOff);
      vec3 col=vec3(r.r,g.g,b.b);
      float v=smoothstep(1.0,0.4,dist); col*=mix(0.72,1.0,v);
      float n=rand(vUv*vec2(uTime*0.01,uTime*0.013));
      col+=(n-0.5)*uGrain;
      gl_FragColor=vec4(col,1.0);
    }`,
};

export const COLOR_GRADE_SHADER = {
  uniforms: {
    tDiffuse: { value: null },
    uExposure: { value: 1.1 },
    uContrast: { value: 1.15 },
    uSaturation: { value: 1.15 },
    uLift: { value: 0.02 },
    uGamma: { value: 0.95 },
    uVibrance: { value: 0.15 },
  },
  vertexShader: `varying vec2 vUv;
    void main(){vUv=uv;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.0);}`,
  fragmentShader: `
    uniform sampler2D tDiffuse;
    uniform float uExposure,uContrast,uSaturation,uLift,uGamma,uVibrance;
    varying vec2 vUv;
    void main(){
      vec4 col=texture2D(tDiffuse,vUv);
      col.rgb*=uExposure;
      col.rgb=uLift+col.rgb*(1.0-uLift);
      col.rgb=pow(col.rgb,vec3(uGamma));
      float avg=dot(col.rgb,vec3(0.299,0.587,0.114));
      col.rgb=avg+(col.rgb-avg)*uContrast;
      col.rgb= mix(col.rgb, vec3(avg), 1.0-uSaturation);
      float maxC=max(col.r,max(col.g,col.b));
      float minC=min(col.r,min(col.g,col.b));
      float intensity=maxC-minC;
      float factor=(maxC+1.0e-7);
      col.rgb+= (col.rgb-avg)*uVibrance*intensity/factor;
      gl_FragColor=vec4(col.rgb,col.a);
    }`,
};

export const MOTION_BLUR_SHADER = {
  uniforms: {
    tDiffuse: { value: null },
    uDamp: { value: 0.92 },
  },
  vertexShader: `varying vec2 vUv;
    void main(){vUv=uv;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.0);}`,
  fragmentShader: `
    uniform sampler2D tDiffuse;
    uniform float uDamp;
    varying vec2 vUv;
    void main(){
      vec4 col=texture2D(tDiffuse,vUv);
      col.rgb*=uDamp;
      gl_FragColor=vec4(col.rgb,col.a);
    }`,
};

export const PIPE_SHADER = {
  uniforms: {
    uTime: { value: 0 },
    uGlow: { value: 1.4 },
    uRim: { value: CONFIG.PIPE_RIM },
    uCore: { value: CONFIG.PIPE_EMISSIVE },
    uSpec: { value: 0.3 },
  },
  vertexShader: `
    varying vec3 vNormal;
    varying vec3 vViewDir;
    varying float vFresnel;
    varying vec3 vColor;
    void main() {
      vNormal = normal;
      vColor = instanceColor;
      vec4 mvPos = modelViewMatrix * vec4(position, 1.0);
      vViewDir = normalize(-mvPos.xyz);
      vFresnel = pow(1.0 - clamp(dot(normalize(normal), vViewDir), 0.0, 1.0), 3.0);
      gl_Position = projectionMatrix * mvPos;
    }
  `,
  fragmentShader: `
    uniform float uTime, uGlow, uRim, uCore, uSpec;
    varying vec3 vNormal;
    varying vec3 vViewDir;
    varying float vFresnel;
    varying vec3 vColor;
    void main() {
      vec3 color = vColor;
      vec3 core = color * uCore;
      vec3 rim = color * vFresnel * uRim;
      vec3 reflectDir = reflect(-vViewDir, normalize(vNormal));
      float spec = pow(max(dot(reflectDir, vViewDir), 0.0), 32.0);
      vec3 shine = vec3(1.0) * spec * uSpec * 0.3;
      float pulse = 0.95 + 0.05 * sin(uTime * 1.5);
      vec3 outCol = (core + rim + shine) * pulse;
      gl_FragColor = vec4(outCol, 1.0);
    }
  `,
};

export const JOINT_SHADER = {
  uniforms: {
    uTime: { value: 0 },
    uGlow: { value: 1.6 },
    uRim: { value: CONFIG.JOINT_RIM },
    uCore: { value: CONFIG.JOINT_EMISSIVE },
    uSpec: { value: 0.4 },
  },
  vertexShader: `
    varying vec3 vNormal;
    varying vec3 vViewDir;
    varying float vFresnel;
    varying vec3 vColor;
    void main() {
      vNormal = normal;
      vColor = instanceColor;
      vec4 mvPos = modelViewMatrix * vec4(position, 1.0);
      vViewDir = normalize(-mvPos.xyz);
      vFresnel = pow(1.0 - clamp(dot(normalize(normal), vViewDir), 0.0, 1.0), 4.0);
      gl_Position = projectionMatrix * mvPos;
    }
  `,
  fragmentShader: `
    uniform float uTime, uGlow, uRim, uCore, uSpec;
    varying vec3 vNormal;
    varying vec3 vViewDir;
    varying float vFresnel;
    varying vec3 vColor;
    void main() {
      vec3 color = vColor;
      vec3 core = color * uCore;
      vec3 rim = color * vFresnel * uRim;
      vec3 reflectDir = reflect(-vViewDir, normalize(vNormal));
      float spec = pow(max(dot(reflectDir, vViewDir), 0.0), 32.0);
      vec3 shine = vec3(1.0) * spec * uSpec * 0.4;
      float pulse = 0.9 + 0.1 * sin(uTime * 2.0);
      vec3 outCol = (core + rim + shine) * pulse;
      gl_FragColor = vec4(outCol, 1.0);
    }
  `,
};

export const INNER_CORE_SHADER = {
  uniforms: {
    uTime: { value: 0 },
    uCore: { value: 1.0 },
  },
  vertexShader: `
    varying vec3 vColor;
    void main() {
      vColor = instanceColor;
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
    }
  `,
  fragmentShader: `
    uniform float uTime, uCore;
    varying vec3 vColor;
    void main() {
      float pulse = 0.95 + 0.05 * sin(uTime * 2.0);
      gl_FragColor = vec4(vColor * uCore * pulse, 1.0);
    }
  `,
};

export const TRAIL_SHADER = {
  uniforms: {
    uTime: { value: 0 },
    uLifetime: { value: 1.6 },
    uSize: { value: 22.0 },
  },
  vertexShader: `
    uniform float uTime, uLifetime, uSize;
    attribute vec3 velocity;
    attribute vec3 color;
    attribute float spawnTime;
    varying vec3 vColor;
    varying float vAlpha;
    void main() {
      float age = uTime - spawnTime;
      if (age < 0.0 || age > uLifetime) {
        gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
        return;
      }
      float t = age / uLifetime;
      vec3 p = position + velocity * age;
      p.z += age * age * 3.0;
      vec4 mv = modelViewMatrix * vec4(p, 1.0);
      gl_PointSize = uSize * (1.0 - t) / -mv.z * 300.0;
      gl_Position = projectionMatrix * mv;
      vAlpha = 1.0 - t;
      vColor = color;
    }
  `,
  fragmentShader: `
    varying vec3 vColor;
    varying float vAlpha;
    void main() {
      vec2 c = gl_PointCoord - vec2(0.5);
      float d = length(c);
      if (d > 0.5) discard;
      float a = (1.0 - d * 2.0) * vAlpha;
      gl_FragColor = vec4(vColor, a);
    }
  `,
};

export const DUST_SHADER = {
  uniforms: { uTime: { value: 0 }, uSize: { value: 6.0 } },
  vertexShader: `
    uniform float uTime, uSize;
    attribute float phase;
    varying float vAlpha;
    void main() {
      vec3 p = position;
      p.x += sin(uTime * 0.18 + phase) * 4.0;
      p.y += cos(uTime * 0.14 + phase * 1.3) * 4.0;
      p.z += sin(uTime * 0.11 + phase * 0.7) * 2.5;
      vec4 mv = modelViewMatrix * vec4(p, 1.0);
      gl_PointSize = uSize / -mv.z * 240.0;
      gl_Position = projectionMatrix * mv;
      vAlpha = 0.25 + 0.35 * sin(uTime * 0.6 + phase);
    }
  `,
  fragmentShader: `
    varying float vAlpha;
    void main() {
      vec2 c = gl_PointCoord - vec2(0.5);
      float d = length(c);
      if (d > 0.5) discard;
      float a = (1.0 - d * 2.0) * vAlpha;
      gl_FragColor = vec4(0.65, 0.82, 1.0, a);
    }
  `,
};

export const STAR_SHADER = {
  uniforms: { uTime: { value: 0 } },
  vertexShader: `
    uniform float uTime;
    attribute float phase;
    varying float vTwinkle;
    void main() {
      vTwinkle = 0.55 + 0.45 * sin(uTime * 1.8 + phase * 6.283);
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
      gl_PointSize = size;
    }
  `,
  fragmentShader: `
    varying float vTwinkle;
    void main() {
      vec4 diffuseColor = vec4(diffuse, opacity * vTwinkle);
      gl_FragColor = diffuseColor;
    }
  `,
};