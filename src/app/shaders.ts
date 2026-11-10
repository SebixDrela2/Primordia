import { CONFIG } from './config';

export const shaders = {
  chromatic: { uniforms: { tDiffuse: { value: null }, uAmount: { value: 0.0020 }, uTime: { value: 0 }, uGrain: { value: 0.012 } },
    vertexShader: `varying vec2 vUv; void main(){vUv=uv;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.0);}`,
    fragmentShader: `uniform sampler2D tDiffuse; uniform float uAmount,uTime,uGrain; varying vec2 vUv; float rand(vec2 co){return fract(sin(dot(co,vec2(12.9898,78.233)))*43758.5453);} void main(){ vec2 dir=vUv-vec2(0.5); float dist=length(dir); float amt=uAmount*dist*dist; vec2 rOff=dir*amt; vec4 r=texture2D(tDiffuse,vUv+rOff); vec4 g=texture2D(tDiffuse,vUv); vec4 b=texture2D(tDiffuse,vUv-rOff); vec3 col=vec3(r.r,g.g,b.b); float v=smoothstep(1.0,0.5,dist); col*=mix(0.85,1.0,v); float n=rand(vUv*vec2(uTime*0.008,uTime*0.01)); col+=(n-0.5)*uGrain*0.6; gl_FragColor=vec4(col,1.0); }` },

  colorGrade: { uniforms: { tDiffuse: { value: null }, uExposure: { value: 0.95 }, uContrast: { value: 1.05 }, uSaturation: { value: 1.05 }, uLift: { value: 0.0 }, uGamma: { value: 1.0 }, uVibrance: { value: 0.08 } },
    vertexShader: `varying vec2 vUv; void main(){vUv=uv;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.0);}`,
    fragmentShader: `uniform sampler2D tDiffuse; uniform float uExposure,uContrast,uSaturation,uLift,uGamma,uVibrance; varying vec2 vUv; void main(){ vec4 col=texture2D(tDiffuse,vUv); col.rgb*=uExposure; col.rgb=uLift+col.rgb*(1.0-uLift); col.rgb=pow(col.rgb,vec3(uGamma)); float avg=dot(col.rgb,vec3(0.299,0.587,0.114)); col.rgb=avg+(col.rgb-avg)*uContrast; col.rgb=mix(col.rgb,vec3(avg),1.0-uSaturation); float maxC=max(col.r,max(col.g,col.b)); float minC=min(col.r,min(col.g,col.b)); float intensity=maxC-minC; float factor=(maxC+1.0e-7); col.rgb+=(col.rgb-avg)*uVibrance*intensity/factor; gl_FragColor=vec4(col.rgb,col.a); }` },

  motionBlur: { uniforms: { tDiffuse: { value: null }, uDamp: { value: 0.95 } },
    vertexShader: `varying vec2 vUv; void main(){vUv=uv;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.0);}`,
    fragmentShader: `uniform sampler2D tDiffuse; uniform float uDamp; varying vec2 vUv; void main(){ vec4 col=texture2D(tDiffuse,vUv); col.rgb*=uDamp; gl_FragColor=vec4(col.rgb,col.a); }` },

  pipe: { uniforms: { uTime: { value: 0 }, uGlow: { value: 0.9 }, uRim: { value: CONFIG.PIPE_RIM }, uCore: { value: CONFIG.PIPE_EMISSIVE }, uSpec: { value: 0.15 } },
    vertexShader: `varying vec3 vNormal,vViewDir,vColor; varying float vFresnel; void main(){ vNormal=normal; vColor=instanceColor; vec4 mvPos=modelViewMatrix*vec4(position,1.0); vViewDir=normalize(-mvPos.xyz); vFresnel=pow(1.0-clamp(dot(normalize(normal),vViewDir),0.0,1.0),4.0); gl_Position=projectionMatrix*mvPos; }`,
    fragmentShader: `uniform float uTime,uRim,uCore,uSpec; varying vec3 vNormal,vViewDir,vColor; varying float vFresnel; void main(){ vec3 c=vColor; vec3 core=c*uCore; vec3 rim=c*vFresnel*uRim; vec3 reflectDir=reflect(-vViewDir,normalize(vNormal)); float spec=pow(max(dot(reflectDir,vViewDir),0.0),64.0); vec3 shine=vec3(1.0)*spec*uSpec*0.2; float pulse=0.97+0.03*sin(uTime*1.2); gl_FragColor=vec4((core+rim+shine)*pulse,1.0); }` },

  joint: { uniforms: { uTime: { value: 0 }, uRim: { value: CONFIG.JOINT_RIM }, uCore: { value: CONFIG.JOINT_EMISSIVE }, uSpec: { value: 0.2 } },
    vertexShader: `varying vec3 vNormal,vViewDir,vColor; varying float vFresnel; void main(){ vNormal=normal; vColor=instanceColor; vec4 mvPos=modelViewMatrix*vec4(position,1.0); vViewDir=normalize(-mvPos.xyz); vFresnel=pow(1.0-clamp(dot(normalize(normal),vViewDir),0.0,1.0),5.0); gl_Position=projectionMatrix*mvPos; }`,
    fragmentShader: `uniform float uTime,uRim,uCore,uSpec; varying vec3 vNormal,vViewDir,vColor; varying float vFresnel; void main(){ vec3 c=vColor; vec3 core=c*uCore; vec3 rim=c*vFresnel*uRim; vec3 reflectDir=reflect(-vViewDir,normalize(vNormal)); float spec=pow(max(dot(reflectDir,vViewDir),0.0),64.0); vec3 shine=vec3(1.0)*spec*uSpec*0.25; float pulse=0.95+0.05*sin(uTime*1.5); gl_FragColor=vec4((core+rim+shine)*pulse,1.0); }` },

  innerCore: { uniforms: { uTime: { value: 0 }, uCore: { value: 0.9 } },
    vertexShader: `varying vec3 vColor; void main(){ vColor=instanceColor; gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.0); }`,
    fragmentShader: `uniform float uTime,uCore; varying vec3 vColor; void main(){ float pulse=0.96+0.04*sin(uTime*1.0); gl_FragColor=vec4(vColor*uCore*pulse,1.0); }` },

  trail: { uniforms: { uTime: { value: 0 }, uLifetime: { value: 1.2 }, uSize: { value: 14.0 } },
    vertexShader: `uniform float uTime,uLifetime,uSize; attribute vec3 velocity,color; attribute float spawnTime; varying vec3 vColor; varying float vAlpha; void main(){ float age=uTime-spawnTime; if(age<0.0||age>uLifetime){ gl_Position=vec4(2.0); return; } float t=age/uLifetime; vec3 p=position+velocity*age*0.5; p.z+=age*age*1.5; vec4 mv=modelViewMatrix*vec4(p,1.0); gl_PointSize=uSize*(1.0-t)/-mv.z*200.0; gl_Position=projectionMatrix*mv; vAlpha=1.0-t; vColor=color; }`,
    fragmentShader: `varying vec3 vColor; varying float vAlpha; void main(){ vec2 c=gl_PointCoord-vec2(0.5); float d=length(c); if(d>0.5) discard; float a=(1.0-d*2.0)*vAlpha*0.6; gl_FragColor=vec4(vColor,a); }` },

  dust: { uniforms: { uTime: { value: 0 }, uSize: { value: 3.0 } },
    vertexShader: `uniform float uTime,uSize; attribute float phase; varying float vAlpha; void main(){ vec3 p=position; p.x+=sin(uTime*0.12+phase)*2.0; p.y+=cos(uTime*0.09+phase*1.1)*2.0; p.z+=sin(uTime*0.07+phase*0.6)*1.5; vec4 mv=modelViewMatrix*vec4(p,1.0); gl_PointSize=uSize/-mv.z*120.0; gl_Position=projectionMatrix*mv; vAlpha=0.15+0.2*sin(uTime*0.4+phase); }`,
    fragmentShader: `varying float vAlpha; void main(){ vec2 c=gl_PointCoord-vec2(0.5); float d=length(c); if(d>0.5) discard; float a=(1.0-d*2.0)*vAlpha*0.4; gl_FragColor=vec4(0.7,0.85,1.0,a); }` },
};