import { Injectable, NgZone } from '@angular/core';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { CSS2DRenderer, CSS2DObject } from 'three/examples/jsm/renderers/CSS2DRenderer.js';
import { EffectComposer } from 'three/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/examples/jsm/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass.js';
import { ShaderPass } from 'three/examples/jsm/postprocessing/ShaderPass.js';
import { CONFIG, MILESTONES } from './config';
import { shaders } from './shaders';
import { SieveService } from './sieve.service';
import { easeOutCubic, easeOutElastic, primeHue, ordinal, ORIENTATIONS, FWD_NAMES, gauss } from './utils';

@Injectable({ providedIn: 'root' })
export class SceneService {
  private scene!: THREE.Scene; private camera!: THREE.PerspectiveCamera;
  private renderer!: THREE.WebGLRenderer; private composer!: EffectComposer;
  private labelRenderer!: CSS2DRenderer; private controls!: OrbitControls;
  private pipes!: THREE.InstancedMesh; private joints!: THREE.InstancedMesh;
  private innerCore!: THREE.InstancedMesh;
  private animPipes: any[] = []; private animJoints: any[] = [];
  private pipeCount = 0; private jointCount = 0; private pointCount = 0;
  private N = 0; private K = 0; private fwd = ORIENTATIONS[0].fwd.clone();
  private curPos = new THREE.Vector3(); private newPos = new THREE.Vector3();
  private lastPrimeN = 0; private nextMilestone = 0; private toastTimer = 0;
  private keys: Record<string, boolean> = {}; private paused = false;
  private lastFrame = 0; private introT = 0; private lastInput = 0;
  private mouseX = 0; private mouseY = 0;
  private labelDirty = true; private lastLabelUpdate = 0;
  private lastLabelCam = new THREE.Vector3(1e9, 1e9, 1e9);
  private labelsEnabled = true; private activeLabels = new Map<number, CSS2DObject>();
  private posBuf = new Float32Array(CONFIG.MAX_JOINTS * 3); private primeNs: number[] = [];
  private pendingTrails = 0;
  private trailHead = 0; private trailPos = new Float32Array(CONFIG.TRAIL_COUNT * 3);
  private trailVel = new Float32Array(CONFIG.TRAIL_COUNT * 3);
  private trailCol = new Float32Array(CONFIG.TRAIL_COUNT * 3);
  private trailSpawn = new Float32Array(CONFIG.TRAIL_COUNT).fill(-100);
  private burstHead = 0; private burstPos = new Float32Array(200 * 3);
  private burstVel = new Float32Array(200 * 3); private burstCol = new Float32Array(200 * 3);
  private burstLife = new Float32Array(200).fill(-1);
  private pulseHead = 0; private pulsePos = new Float32Array(200 * 3);
  private pulseCol = new Float32Array(200 * 3); private pulseLife = new Float32Array(200).fill(-1);
  private pulseDir = new Float32Array(200 * 3);
  private arcHead = 0; private arcPos = new Float32Array(64 * 2 * 3); private arcCol = new Float32Array(64 * 2 * 3);
  private arcPositions: { x: number; y: number; z: number; hue: number }[] = [];

  private ud: any = {};

  constructor(private zone: NgZone, private sieve: SieveService) {}

  init(canvas: HTMLCanvasElement) {
    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color(0x000000);
    this.scene.fog = new THREE.FogExp2(0x02040a, 0.000018);

    this.camera = new THREE.PerspectiveCamera(55, innerWidth / innerHeight, 0.1, 1e8);
    this.camera.up.set(0, 0, 1); this.camera.position.set(300, -380, 320);

    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, powerPreference: 'high-performance' });
    this.renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
    this.renderer.setSize(innerWidth, innerHeight);
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = CONFIG.TONEMAP_EXPOSURE;
    this.renderer.shadowMap.enabled = true; this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;

    this.composer = new EffectComposer(this.renderer);
    this.composer.addPass(new RenderPass(this.scene, this.camera));
    this.composer.addPass(new UnrealBloomPass(new THREE.Vector2(innerWidth, innerHeight), CONFIG.BLOOM_STRENGTH, CONFIG.BLOOM_RADIUS, CONFIG.BLOOM_THRESHOLD));
    this.composer.addPass(new ShaderPass(shaders.chromatic));
    this.composer.addPass(new ShaderPass(shaders.colorGrade));
    this.composer.addPass(new ShaderPass(shaders.motionBlur));

    this.labelRenderer = new CSS2DRenderer();
    this.labelRenderer.setSize(innerWidth, innerHeight);
    Object.assign(this.labelRenderer.domElement.style, { position: 'absolute', top: '0', left: '0', pointerEvents: 'none', zIndex: '10' });
    document.body.appendChild(this.labelRenderer.domElement);

    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = true; this.controls.dampingFactor = 0.06;
    this.controls.maxDistance = 1e8; this.controls.minDistance = 0.05;

    this.setupLights(); this.setupBackground(); this.setupGeometry(); this.setupHead();
    this.setupTrails(); this.setupPulses(); this.setupArcs(); this.setupBursts(); this.setupDust();
    this.bindEvents();

    this.zone.runOutsideAngular(() => this.animate());
  }

  private setupLights() {
    this.scene.add(new THREE.AmbientLight(0xffffff, CONFIG.AMBIENT));
    const key = new THREE.DirectionalLight(0xffffff, CONFIG.KEY_LIGHT);
    key.position.set(400, -400, 900); key.castShadow = true;
    key.shadow.mapSize.set(CONFIG.SHADOW_MAP, CONFIG.SHADOW_MAP);
    key.shadow.camera.near = 1; key.shadow.camera.far = 3000;
    key.shadow.camera.left = -CONFIG.SHADOW_RADIUS; key.shadow.camera.right = CONFIG.SHADOW_RADIUS;
    key.shadow.camera.top = CONFIG.SHADOW_RADIUS; key.shadow.camera.bottom = -CONFIG.SHADOW_RADIUS;
    key.shadow.bias = -0.00015; key.shadow.normalBias = 0.05;
    this.scene.add(key);
    const target = new THREE.Object3D(); this.scene.add(target); key.target = target;
    const fill = new THREE.DirectionalLight(0x88aaff, CONFIG.FILL_LIGHT);
    fill.position.set(-500, 500, -300); this.scene.add(fill);
  }

  private setupBackground() {
    const halo = this.makeHaloTexture(128);
    const starGroup = new THREE.Group(); this.scene.add(starGroup);
    const starMat = (size: number, opacity: number) => {
      const m = new THREE.PointsMaterial({ size, sizeAttenuation: false, vertexColors: true, transparent: true, opacity, depthWrite: false, fog: false, blending: THREE.AdditiveBlending });
      m.onBeforeCompile = (sh: any) => { sh.uniforms.uTime = { value: 0 }; sh.vertexShader = sh.vertexShader.replace('void main() {', `uniform float uTime; attribute float phase; varying float vTwinkle; void main(){vTwinkle=0.55+0.45*sin(uTime*1.8+phase*6.283);`).replace('vec4 diffuseColor = vec4( diffuse, opacity );', 'vec4 diffuseColor=vec4(diffuse,opacity*vTwinkle);'); m.userData['shader'] = sh; };
      return m;
    };
    const makePoints = (count: number, posFn: Function, colFn: Function, size: number, opacity: number) => {
      const pos = new Float32Array(count * 3), col = new Float32Array(count * 3), phase = new Float32Array(count);
      for (let i = 0; i < count; i++) { const p = posFn(i, count); pos[i*3]=p[0]; pos[i*3+1]=p[1]; pos[i*3+2]=p[2]; const c = colFn(i, count, p); col[i*3]=c[0]; col[i*3+1]=c[1]; col[i*3+2]=c[2]; phase[i]=Math.random()*Math.PI*2; }
      const g = new THREE.BufferGeometry(); g.setAttribute('position', new THREE.BufferAttribute(pos,3)); g.setAttribute('color', new THREE.BufferAttribute(col,3)); g.setAttribute('phase', new THREE.BufferAttribute(phase,1));
      return new THREE.Points(g, starMat(size, opacity));
    };
    const ds = makePoints(8000, () => { const t=Math.random()*Math.PI*2, p=Math.acos(2*Math.random()-1), r=14000+Math.random()*8000; return [r*Math.sin(p)*Math.cos(t), r*Math.sin(p)*Math.sin(t), r*Math.cos(p)]; }, () => { const t=Math.random(); if(t<0.03)return[0.6,0.7,0.9]; if(t<0.12)return[0.7,0.75,0.85]; if(t<0.4)return[0.85,0.85,0.9]; if(t<0.7)return[0.9,0.88,0.85]; return[0.8,0.7,0.65]; }, 1.2, 0.5);
    const mw = makePoints(16000, () => { const t=Math.random()*Math.PI*2, p=gauss()*0.14, r=15000+Math.random()*5000; return [r*Math.cos(t)*Math.cos(p), r*Math.sin(t)*Math.cos(p), r*Math.sin(p)]; }, () => { const t=Math.random(); if(t<0.5)return[0.7,0.72,0.8]; if(t<0.8)return[0.75,0.75,0.8]; return[0.7,0.75,0.85]; }, 0.8, 0.4);
    mw.rotation.x=Math.PI*0.3; mw.rotation.z=Math.PI*0.1; starGroup.add(ds); starGroup.add(mw);
    this.scene.add(starGroup);
    this.ud = { starGroup, deepStars: ds, milkyWay: mw };
  }
  }

  private makeHaloTexture(size: number) { const cv=document.createElement('canvas'); cv.width=cv.height=size; const ctx=cv.getContext('2d')!; const g=ctx.createRadialGradient(size/2,size/2,0,size/2,size/2,size/2); g.addColorStop(0,'rgba(255,255,255,1)'); g.addColorStop(0.15,'rgba(255,255,255,0.8)'); g.addColorStop(0.35,'rgba(160,200,255,0.35)'); g.addColorStop(1,'rgba(0,0,0,0)'); ctx.fillStyle=g; ctx.fillRect(0,0,size,size); return new THREE.CanvasTexture(cv); }

  private setupGeometry() {
    const cyl=new THREE.CylinderGeometry(CONFIG.PIPE_RADIUS,CONFIG.PIPE_RADIUS,1,14,1,false);
    const sph=new THREE.SphereGeometry(CONFIG.PIPE_RADIUS*CONFIG.JOINT_MULT,14,10);
    this.pipes=new THREE.InstancedMesh(cyl,new THREE.ShaderMaterial(shaders.pipe),CONFIG.MAX_PIPES);
    this.joints=new THREE.InstancedMesh(sph,new THREE.ShaderMaterial(shaders.joint),CONFIG.MAX_JOINTS);
    this.innerCore=new THREE.InstancedMesh(new THREE.CylinderGeometry(CONFIG.PIPE_RADIUS*0.35,CONFIG.PIPE_RADIUS*0.35,1,8,1,false),new THREE.ShaderMaterial(shaders.innerCore),CONFIG.MAX_PIPES);
    [this.pipes,this.joints,this.innerCore].forEach(m=>{m.count=0;m.frustumCulled=false;m.instanceMatrix.setUsage(THREE.DynamicDrawUsage);m.instanceColor?.setUsage?.(THREE.DynamicDrawUsage);this.scene.add(m);});
  }

  private setupHead() {
    this.ud.headCore=new THREE.Mesh(new THREE.SphereGeometry(CONFIG.PIPE_RADIUS*1.6,16,12),new THREE.MeshBasicMaterial({color:0xffffff,toneMapped:false,depthTest:false,depthWrite:false})); this.ud.headCore.renderOrder=100; this.scene.add(this.ud.headCore);
    this.ud.headArrow=new THREE.Mesh(new THREE.ConeGeometry(CONFIG.PIPE_RADIUS*1.4,CONFIG.PIPE_RADIUS*5,14),new THREE.MeshBasicMaterial({color:0xffffff,toneMapped:false,depthTest:false,depthWrite:false})); this.ud.headArrow.renderOrder=103; this.scene.add(this.ud.headArrow);
    this.ud.tipLight=new THREE.PointLight(0xffffff,0.3,100,2); this.scene.add(this.ud.tipLight);
    this.ud.headWake=new THREE.Mesh(new THREE.ConeGeometry(CONFIG.PIPE_RADIUS*0.4,CONFIG.PIPE_RADIUS*5,10),new THREE.MeshBasicMaterial({color:0xffffff,toneMapped:false,transparent:true,opacity:0.15,depthTest:false,depthWrite:false})); this.ud.headWake.geometry.translate(0,-CONFIG.PIPE_RADIUS*2.5,0); this.ud.headWake.renderOrder=102; this.scene.add(this.ud.headWake);
    this.ud.headWarp=new THREE.Mesh(new THREE.CylinderGeometry(0,CONFIG.PIPE_RADIUS*8,CONFIG.PIPE_RADIUS*30,16,1,true),new THREE.MeshBasicMaterial({color:0x50dcff,toneMapped:false,transparent:true,opacity:0,blending:THREE.AdditiveBlending,depthTest:false,depthWrite:false})); this.ud.headWarp.geometry.translate(0,-CONFIG.PIPE_RADIUS*15,0); this.ud.headWarp.renderOrder=104; this.scene.add(this.ud.headWarp);
  }

  private setupTrails() {
    const g=new THREE.BufferGeometry(); g.setAttribute('position',new THREE.BufferAttribute(this.trailPos,3).setUsage(THREE.DynamicDrawUsage)); g.setAttribute('velocity',new THREE.BufferAttribute(this.trailVel,3).setUsage(THREE.DynamicDrawUsage)); g.setAttribute('color',new THREE.BufferAttribute(this.trailCol,3).setUsage(THREE.DynamicDrawUsage)); g.setAttribute('spawnTime',new THREE.BufferAttribute(this.trailSpawn,1).setUsage(THREE.DynamicDrawUsage));
    this.ud.trail=new THREE.Points(g,new THREE.ShaderMaterial(shaders.trail)); this.ud.trail.frustumCulled=false; this.scene.add(this.ud.trail);
  }

  private setupPulses() { const g=new THREE.BufferGeometry(); g.setAttribute('position',new THREE.BufferAttribute(this.pulsePos,3).setUsage(THREE.DynamicDrawUsage)); g.setAttribute('color',new THREE.BufferAttribute(this.pulseCol,3).setUsage(THREE.DynamicDrawUsage)); this.ud.pulses=new THREE.Points(g,new THREE.PointsMaterial({size:3,sizeAttenuation:true,vertexColors:true,transparent:true,blending:THREE.AdditiveBlending,depthWrite:false,fog:false})); this.ud.pulses.frustumCulled=false; this.scene.add(this.ud.pulses); }
  private setupArcs() { const g=new THREE.BufferGeometry(); g.setAttribute('position',new THREE.BufferAttribute(this.arcPos,3).setUsage(THREE.DynamicDrawUsage)); g.setAttribute('color',new THREE.BufferAttribute(this.arcCol,3).setUsage(THREE.DynamicDrawUsage)); this.ud.arcs=new THREE.LineSegments(g,new THREE.LineBasicMaterial({vertexColors:true,transparent:true,opacity:0.5,blending:THREE.AdditiveBlending,depthWrite:false,fog:false})); this.ud.arcs.frustumCulled=false; this.scene.add(this.ud.arcs); }
  private setupBursts() { const g=new THREE.BufferGeometry(); g.setAttribute('position',new THREE.BufferAttribute(this.burstPos,3).setUsage(THREE.DynamicDrawUsage)); g.setAttribute('color',new THREE.BufferAttribute(this.burstCol,3).setUsage(THREE.DynamicDrawUsage)); this.ud.bursts=new THREE.Points(g,new THREE.PointsMaterial({size:4,sizeAttenuation:true,vertexColors:true,transparent:true,blending:THREE.AdditiveBlending,depthWrite:false,fog:false})); this.ud.bursts.frustumCulled=false; this.scene.add(this.ud.bursts); }
  private setupDust() { const g=new THREE.BufferGeometry(); const pos=new Float32Array(CONFIG.DUST_COUNT*3),ph=new Float32Array(CONFIG.DUST_COUNT); for(let i=0;i<CONFIG.DUST_COUNT;i++){pos[i*3]=(Math.random()-0.5)*2500;pos[i*3+1]=(Math.random()-0.5)*2500;pos[i*3+2]=(Math.random()-0.5)*1400;ph[i]=Math.random()*Math.PI*2;} g.setAttribute('position',new THREE.BufferAttribute(pos,3)); g.setAttribute('phase',new THREE.BufferAttribute(ph,1)); this.ud.dust=new THREE.Points(g,new THREE.ShaderMaterial(shaders.dust)); this.ud.dust.frustumCulled=false; this.scene.add(this.ud.dust); }

  private bindEvents() {
    window.addEventListener('keydown',e=>{if(e.ctrlKey||e.metaKey||e.altKey)return; this.keys[e.code]=true; this.lastInput=performance.now(); if(e.code==='Space')e.preventDefault(); if(e.code==='KeyP')this.paused=!this.paused; if(e.code==='KeyF')this.fitCamera(); if(e.code==='KeyR')this.reset(); if(e.code==='KeyL'){this.labelsEnabled=!this.labelsEnabled; this.activeLabels.forEach(l=>l.visible=this.labelsEnabled); this.labelDirty=true;}});
    window.addEventListener('keyup',e=>{this.keys[e.code]=false; this.lastInput=performance.now();});
    window.addEventListener('blur',()=>{for(const k in this.keys)this.keys[k]=false;});
    this.renderer.domElement.addEventListener('pointerdown',()=>this.lastInput=performance.now());
    this.renderer.domElement.addEventListener('wheel',()=>this.lastInput=performance.now(),{passive:true});
    window.addEventListener('mousemove',e=>{this.mouseX=(e.clientX/innerWidth)*2-1;this.mouseY=(e.clientY/innerHeight)*2-1;});
    window.addEventListener('resize',()=>{this.camera.aspect=innerWidth/innerHeight;this.camera.updateProjectionMatrix();this.renderer.setSize(innerWidth,innerHeight);this.composer.setSize(innerWidth,innerHeight);this.labelRenderer.setSize(innerWidth,innerHeight);});

    for(let i=0;i<100;i++)this.step(performance.now());
    this.flush(); this.fitCamera();
    this.ud.introFp=this.camera.position.clone(); this.ud.introFt=this.controls.target.clone();
    this.ud.introStart=this.ud.introFp.clone().multiplyScalar(3.5).add(new THREE.Vector3(0,0,600));
    this.ud.introEnd=new THREE.Vector3(0,0,0);
    this.camera.position.copy(this.ud.introStart); this.controls.target.copy(this.ud.introEnd);
  }

  private step(now: number) {
    if(this.pipeCount>=CONFIG.MAX_PIPES)return;
    this.N++; this.newPos.copy(this.curPos).addScaledVector(this.fwd,CONFIG.STEP_SIZE);
    const hue=primeHue(this.N); const gap=this.N-this.lastPrimeN; const thick=1.1+Math.min(gap*0.10,4.0);
    this.addPipe(this.curPos,this.newPos,hue,now,thick);
    if(this.sieve.isPrime(this.N)){this.lastPrimeN=this.N;this.addJoint(this.newPos,hue,now,this.N);this.arcPositions.push({x:this.newPos.x,y:this.newPos.y,z:this.newPos.z,hue});if(this.arcPositions.length>64)this.arcPositions.shift();this.pointCount++;this.labelDirty=true;if(this.nextMilestone<MILESTONES.length&&this.N===MILESTONES[this.nextMilestone]){this.showToast(this.N);this.nextMilestone++;this.spawnBurst(this.newPos.x,this.newPos.y,this.newPos.z,hue);}this.K=(this.K+1)%ORIENTATIONS.length;this.fwd.copy(ORIENTATIONS[this.K].fwd);this.pendingTrails+=4;}else{this.pendingTrails+=1;}
    this.curPos.copy(this.newPos); this.updateHead(this.curPos,this.fwd,hue);
  }

  private addPipe(from: THREE.Vector3, to: THREE.Vector3, hue: number, now: number, thick: number) {
    if(this.pipeCount>=CONFIG.MAX_PIPES)return;
    const dir=new THREE.Vector3().subVectors(to,from); const len=dir.length(); if(len<1e-6)return; dir.normalize();
    const q=new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0,1,0),dir);
    const idx=this.pipeCount++; const m=new THREE.Matrix4(), s=new THREE.Vector3(), c=new THREE.Color();
    m.compose(from,q,s.set(thick,1,thick)); this.pipes.setMatrixAt(idx,m); this.innerCore.setMatrixAt(idx,m);
    c.setHSL(hue,1.0,0.62); this.pipes.setColorAt(idx,c); this.innerCore.setColorAt(idx,c);
    this.animPipes.push({idx,from:from.clone(),dir:dir.clone(),len,q,t0:now,thick});
  }

  private addJoint(at: THREE.Vector3, hue: number, now: number, n: number) {
    if(this.jointCount>=CONFIG.MAX_JOINTS)return;
    const idx=this.jointCount++; const m=new THREE.Matrix4();
    m.compose(at,new THREE.Quaternion(),new THREE.Vector3(0,0,0)); this.joints.setMatrixAt(idx,m);
    const c=new THREE.Color().setHSL(hue,1.0,0.52); this.joints.setColorAt(idx,c);
    this.animJoints.push({idx,at:at.clone(),t0:now,n});
  }

  private animatePipes(now: number) {
    for(let i=this.animPipes.length-1;i>=0;i--){const a=this.animPipes[i];const t=Math.min(1,(now-a.t0)/CONFIG.PIPE_GROW_MS);const e=easeOutCubic(t);const thick=a.thick*e;const mid=new THREE.Vector3().copy(a.from).addScaledVector(a.dir,(a.len*e)/2);const m=new THREE.Matrix4().compose(mid,a.q,new THREE.Vector3(thick,Math.max(a.len*e,1e-5),thick));this.pipes.setMatrixAt(a.idx,m);this.innerCore.setMatrixAt(a.idx,m);if(t>=1)this.animPipes.splice(i,1);}
    this.pipes.instanceMatrix.needsUpdate=true; this.innerCore.instanceMatrix.needsUpdate=true;
  }

  private animateJoints(now: number) {
    for(let i=this.animJoints.length-1;i>=0;i--){const a=this.animJoints[i];const t=Math.min(1,(now-a.t0)/CONFIG.JOINT_POP_MS);const s=easeOutElastic(t);const m=new THREE.Matrix4().compose(a.at,new THREE.Quaternion(),new THREE.Vector3(s,s,s));this.joints.setMatrixAt(a.idx,m);if(t>=1){m.compose(a.at,new THREE.Quaternion(),new THREE.Vector3(1,1,1));this.joints.setMatrixAt(a.idx,m);this.animJoints.splice(i,1);}}
    this.joints.instanceMatrix.needsUpdate=true;
  }

  private updateHead(pos: THREE.Vector3, fwd: THREE.Vector3, hue: number) {
    const c=new THREE.Color().setHSL(hue,1.0,0.55);
    this.ud.headCore.position.copy(pos); (this.ud.headCore.material as THREE.MeshBasicMaterial).color.copy(c).lerp(new THREE.Color(0xffffff),0.4);
    this.ud.headArrow.position.copy(pos).addScaledVector(fwd,CONFIG.PIPE_RADIUS*3.6); this.ud.headArrow.quaternion.setFromUnitVectors(new THREE.Vector3(0,1,0),fwd); (this.ud.headArrow.material as THREE.MeshBasicMaterial).color.copy(c);
    this.ud.headWake.position.copy(pos).addScaledVector(fwd,-CONFIG.PIPE_RADIUS*2.2); this.ud.headWake.quaternion.setFromUnitVectors(new THREE.Vector3(0,1,0),fwd); (this.ud.headWake.material as THREE.MeshBasicMaterial).color.copy(c); (this.ud.headWake.material as THREE.MeshBasicMaterial).opacity=0.12+0.06*Math.sin(performance.now()*0.008);
    const boosting=this.keys['ShiftLeft']||this.keys['ShiftRight'];
    if(boosting){this.ud.headWarp.position.copy(pos).addScaledVector(fwd,-CONFIG.PIPE_RADIUS*14);this.ud.headWarp.quaternion.setFromUnitVectors(new THREE.Vector3(0,1,0),fwd);const ws=0.6+0.4*Math.sin(performance.now()*0.02);this.ud.headWarp.scale.setScalar(CONFIG.PIPE_RADIUS*6*ws);(this.ud.headWarp.material as THREE.MeshBasicMaterial).opacity=0.2+0.1*Math.sin(performance.now()*0.015);(this.ud.headWarp.material as THREE.MeshBasicMaterial).color.copy(c);}(this.ud.headWarp.material as THREE.MeshBasicMaterial).opacity=boosting?(this.ud.headWarp.material as THREE.MeshBasicMaterial).opacity:0;
    this.ud.tipLight.position.copy(pos); this.ud.tipLight.color.copy(c);
  }

  private animate() { requestAnimationFrame(()=>this.animate()); const now=performance.now(); const dt=now-this.lastFrame; this.lastFrame=now;
    if(this.introT<1){const ud=this.ud;const interrupted=this.lastInput>performance.now()-dt+50;if(interrupted){this.introT=1;this.camera.position.copy(ud.introFp||new THREE.Vector3(300,-380,320));this.controls.target.copy(ud.introFt||new THREE.Vector3());this.controls.update();}else{this.introT=Math.min(1,this.introT+dt/CONFIG.INTRO_MS);const t=easeOutCubic(this.introT);this.camera.position.lerpVectors(ud.introStart||new THREE.Vector3(),ud.introEnd||new THREE.Vector3(),t);this.controls.target.lerpVectors(new THREE.Vector3(),new THREE.Vector3(),t);this.camera.lookAt(this.controls.target);}}else{if(!this.paused)this.handleInput();const idle=now-this.lastInput>CONFIG.IDLE_ORBIT_DELAY;this.controls.autoRotate=idle&&!this.paused;this.controls.update();}
    if(!this.paused&&this.pipeCount<CONFIG.MAX_PIPES){this.pendingTrails+=0.5;}
    this.animatePipes(now); this.animateJoints(now); this.updateTrails(now); this.updatePulses(now); this.updateArcs(); this.updateBursts(now); this.updateBackground(now); this.updatePost(now);
    this.composer.render(); this.labelRenderer.render(this.scene,this.camera);
    this.updateLabels(); this.updateUI(now);
  }

  private handleInput() {
    this.camera.getWorldDirection(new THREE.Vector3()); const fwd=this.camera.getWorldDirection(new THREE.Vector3()); const right=new THREE.Vector3().crossVectors(fwd,new THREE.Vector3(0,0,1)).normalize(); const move=new THREE.Vector3(); const zUp=new THREE.Vector3(0,0,1);
    if(this.keys['KeyW'])move.add(fwd); if(this.keys['KeyS'])move.sub(fwd); if(this.keys['KeyD'])move.add(right); if(this.keys['KeyA'])move.sub(right); if(this.keys['Space']||this.keys['KeyE'])move.add(zUp); if(this.keys['KeyQ'])move.sub(zUp);
    if(move.lengthSq()>0){const dist=this.camera.position.distanceTo(this.controls.target);const boost=(this.keys['ShiftLeft']||this.keys['ShiftRight'])?CONFIG.MOVE_BOOST:1;const speed=Math.max(dist*CONFIG.MOVE_BASE,0.05)*boost;move.normalize().multiplyScalar(speed);this.camera.position.add(move);this.controls.target.add(move);}
  }

  private updateTrails(now: number) {
    if(!this.paused&&this.pendingTrails>0){const hue=primeHue(this.N);const spawn=Math.min(this.pendingTrails,8);for(let i=0;i<spawn;i++)this.spawnTrail(this.curPos,hue);this.pendingTrails=Math.max(0,this.pendingTrails-spawn);}
    (this.ud.trail.material as THREE.ShaderMaterial).uniforms['uTime'].value=now*0.001;
  }

  private spawnTrail(pos: THREE.Vector3, hue: number) {
    const i=this.trailHead; this.trailHead=(this.trailHead+1)%CONFIG.TRAIL_COUNT;
    this.trailPos[i*3]=pos.x; this.trailPos[i*3+1]=pos.y; this.trailPos[i*3+2]=pos.z;
    this.trailVel[i*3]=(Math.random()-0.5)*8; this.trailVel[i*3+1]=(Math.random()-0.5)*8; this.trailVel[i*3+2]=Math.random()*5+1;
    const c=new THREE.Color().setHSL(hue,1.0,0.6); this.trailCol[i*3]=c.r; this.trailCol[i*3+1]=c.g; this.trailCol[i*3+2]=c.b;
    this.trailSpawn[i]=performance.now()*0.001;
    const t=this.ud.trail; t.geometry.attributes['position'].needsUpdate=true; t.geometry.attributes['velocity'].needsUpdate=true; t.geometry.attributes['color'].needsUpdate=true; t.geometry.attributes['spawnTime'].needsUpdate=true;
  }

  private updatePulses(now: number) {
    let any=false; for(let i=0;i<200;i++){if(this.pulseLife[i]>0){this.pulseLife[i]-=0.0012*(now-this.lastFrame);if(this.pulseLife[i]<=0){this.pulseLife[i]=0;continue;}this.pulsePos[i*3]+=this.pulseDir[i*3]*0.001*(now-this.lastFrame);this.pulsePos[i*3+1]+=this.pulseDir[i*3+1]*0.001*(now-this.lastFrame);this.pulsePos[i*3+2]+=this.pulseDir[i*3+2]*0.001*(now-this.lastFrame);this.pulseCol[i*3]*=0.99;this.pulseCol[i*3+1]*=0.99;this.pulseCol[i*3+2]*=0.99;any=true;}}if(any){const p=this.ud.pulses;p.geometry.attributes['position'].needsUpdate=true;p.geometry.attributes['color'].needsUpdate=true;}
  }

  private spawnPulse(x: number, y: number, z: number, hue: number, dx: number, dy: number, dz: number) {
    const i=this.pulseHead; this.pulseHead=(this.pulseHead+1)%200;
    this.pulsePos[i*3]=x; this.pulsePos[i*3+1]=y; this.pulsePos[i*3+2]=z;
    const c=new THREE.Color().setHSL(hue,1,0.8); this.pulseCol[i*3]=c.r; this.pulseCol[i*3+1]=c.g; this.pulseCol[i*3+2]=c.b;
    this.pulseLife[i]=1.0; this.pulseDir[i*3]=dx; this.pulseDir[i*3+1]=dy; this.pulseDir[i*3+2]=dz;
    const p=this.ud.pulses; p.geometry.attributes['position'].needsUpdate=true; p.geometry.attributes['color'].needsUpdate=true;
  }

  private updateArcs() { const c=Math.max(0,this.arcPositions.length-1); for(let i=0;i<64;i++){const b=i*6;if(i<c){const a=this.arcPositions[i],bb=this.arcPositions[i+1];this.arcPos[b]=a.x;this.arcPos[b+1]=a.y;this.arcPos[b+2]=a.z;this.arcPos[b+3]=bb.x;this.arcPos[b+4]=bb.y;this.arcPos[b+5]=bb.z;const ca=new THREE.Color().setHSL(a.hue,1,0.6),cb=new THREE.Color().setHSL(bb.hue,1,0.6);this.arcCol[b]=ca.r;this.arcCol[b+1]=ca.g;this.arcCol[b+2]=ca.b;this.arcCol[b+3]=cb.r;this.arcCol[b+4]=cb.g;this.arcCol[b+5]=cb.b;}else{this.arcPos.fill(0,b,b+6);this.arcCol.fill(0,b,b+6);}}const a=this.ud.arcs;a.geometry.attributes['position'].needsUpdate=true;a.geometry.attributes['color'].needsUpdate=true;a.material.opacity=0.4+0.2*Math.sin(performance.now()*0.003);}

  private spawnBurst(x: number, y: number, z: number, hue: number) { for(let i=0;i<60;i++){const j=this.burstHead;this.burstHead=(this.burstHead+1)%200;this.burstPos[j*3]=x;this.burstPos[j*3+1]=y;this.burstPos[j*3+2]=z;const a=Math.random()*Math.PI*2,s=Math.random()*30+10;this.burstVel[j*3]=Math.cos(a)*s;this.burstVel[j*3+1]=Math.sin(a)*s;this.burstVel[j*3+2]=(Math.random()-0.5)*20;const c=new THREE.Color().setHSL(hue,1,0.7);this.burstCol[j*3]=c.r;this.burstCol[j*3+1]=c.g;this.burstCol[j*3+2]=c.b;this.burstLife[j]=1.0;}const b=this.ud.bursts;b.geometry.attributes['position'].needsUpdate=true;b.geometry.attributes['color'].needsUpdate=true;}

  private updateBursts(now: number) { let any=false; const dt=now-this.lastFrame; for(let i=0;i<200;i++){if(this.burstLife[i]>0){this.burstLife[i]-=dt*0.0015;if(this.burstLife[i]<=0){this.burstLife[i]=0;continue;}this.burstPos[i*3]+=this.burstVel[i*3]*dt*0.001;this.burstPos[i*3+1]+=this.burstVel[i*3+1]*dt*0.001;this.burstPos[i*3+2]+=this.burstVel[i*3+2]*dt*0.001;this.burstVel[i*3+2]-=dt*0.0002;any=true;}}if(any){const b=this.ud.bursts;b.geometry.attributes['position'].needsUpdate=true;(b.material as THREE.PointsMaterial).opacity=0.6;}}

  private updateBackground(now: number) {
    const t=now*0.001;
    [this.ud.deepStars,this.ud.milkyWay].forEach(s=>{if(s.material.userData?.shader)s.material.userData.shader.uniforms['uTime'].value=t;});
    if(this.ud.dust)(this.ud.dust.material as THREE.ShaderMaterial).uniforms['uTime'].value=t;
  }

  private updatePost(now: number) {
    const t=now*0.001;
    shaders.chromatic.uniforms['uTime'].value=t;
    shaders.colorGrade.uniforms['uExposure'].value=0.95+0.03*Math.sin(t);
    shaders.colorGrade.uniforms['uSaturation'].value=1.05;
    shaders.colorGrade.uniforms['uVibrance'].value=0.08;
    shaders.motionBlur.uniforms['uDamp'].value=0.94+0.03*Math.sin(t*2);
    (this.pipes.material as THREE.ShaderMaterial).uniforms['uTime'].value=t;
    (this.joints.material as THREE.ShaderMaterial).uniforms['uTime'].value=t;
    (this.innerCore.material as THREE.ShaderMaterial).uniforms['uTime'].value=t;
  }

  private updateLabels() {
    if(!this.labelsEnabled){this.activeLabels.forEach((_,k)=>this.removeLabel(k));return;}
    if(!this.labelDirty&&this.camera.position.distanceToSquared(this.lastLabelCam)<1)return;
    this.lastLabelUpdate=performance.now();this.lastLabelCam.copy(this.camera.position);
    const r2=CONFIG.LABEL_RADIUS*CONFIG.LABEL_RADIUS; const cand:[number,number][]=[];
    for(let i=0;i<this.pointCount;i++){const dx=this.posBuf[i*3]-this.camera.position.x,dy=this.posBuf[i*3+1]-this.camera.position.y,dz=this.posBuf[i*3+2]-this.camera.position.z;const d2=dx*dx+dy*dy+dz*dz;if(d2<r2)cand.push([i,d2]);}
    cand.sort((a,b)=>a[1]-b[1]); const n=Math.min(CONFIG.MAX_LABELS,cand.length); const chosen=new Set<number>();
    for(let k=0;k<n;k++)chosen.add(cand[k][0]);
    for(const idx of this.activeLabels.keys())if(!chosen.has(idx))this.removeLabel(idx);
    for(const idx of chosen)if(!this.activeLabels.has(idx))this.makeLabel(idx);
    this.labelDirty=false;
  }

  private makeLabel(idx: number) { const n=this.primeNs[idx]; const div=document.createElement('div');div.className='prime-label';const sp=document.createElement('span');sp.textContent=String(n);sp.style.color=`hsl(${primeHue(n)*360},100%,62%)`;div.appendChild(sp);const l=new CSS2DObject(div);l.position.set(this.posBuf[idx*3],this.posBuf[idx*3+1],this.posBuf[idx*3+2]);l.center.set(0.5,1.8);l.visible=this.labelsEnabled;this.scene.add(l);this.activeLabels.set(idx,l); }
  private removeLabel(idx: number) { const l=this.activeLabels.get(idx); if(!l)return; this.scene.remove(l); if(l.element?.parentNode)l.element.parentNode.removeChild(l.element); this.activeLabels.delete(idx); }

  private flush() {
    this.pipes.count=this.pipeCount; this.joints.count=this.jointCount; this.innerCore.count=this.pipeCount;
    this.pipes.instanceMatrix.needsUpdate=true; this.joints.instanceMatrix.needsUpdate=true; this.innerCore.instanceMatrix.needsUpdate=true;
    (this.pipes.instanceColor as THREE.InstancedBufferAttribute).needsUpdate=true;
    (this.joints.instanceColor as THREE.InstancedBufferAttribute).needsUpdate=true;
    (this.innerCore.instanceColor as THREE.InstancedBufferAttribute).needsUpdate=true;
  }

  private fitCamera() {
    if(this.N<2)return;
    const min=new THREE.Vector3(Infinity,Infinity,Infinity),max=new THREE.Vector3(-Infinity,-Infinity,-Infinity);
    const m=new THREE.Matrix4(),s=new THREE.Vector3();
    for(let i=0;i<this.pipeCount;i++){this.pipes.getMatrixAt(i,m);m.decompose(s,new THREE.Quaternion(),s);min.min(s);max.max(s);}
    const cx=(min.x+max.x)*0.5,cy=(min.y+max.y)*0.5,cz=(min.z+max.z)*0.5;
    const size=Math.max(max.x-min.x,max.y-min.y,max.z-min.z,10);
    const fov=this.camera.fov*Math.PI/180;const dist=(size*0.5)/Math.tan(fov*0.5)*1.9;
    const dir=new THREE.Vector3(0.7,-1.0,0.5).normalize();
    this.controls.target.set(cx,cy,cz);this.camera.position.copy(this.controls.target).addScaledVector(dir,dist);this.controls.update();
  }

  private reset() {
    this.N=0;this.K=0;this.fwd.copy(ORIENTATIONS[0].fwd);
    this.pipeCount=0;this.jointCount=0;this.pointCount=0;this.lastPrimeN=0;
    this.animPipes.length=0;this.animJoints.length=0;this.primeNs.length=0;
    this.curPos.set(0,0,0);this.nextMilestone=0;this.toastTimer=0;
    this.pipes.count=0;this.joints.count=0;this.innerCore.count=0;
    this.activeLabels.forEach((_,k)=>this.removeLabel(k));
    this.trailSpawn.fill(-100);this.burstLife.fill(-1);this.pulseLife.fill(-1);this.arcPositions.length=0;
    for(let i=0;i<100;i++)this.step(performance.now());
    this.flush();this.fitCamera();
  }

  private showToast(n: number) { const t=document.getElementById('toast') as HTMLElement; if(t){(t.querySelector('#t-n') as HTMLElement).textContent=n.toLocaleString();(t.querySelector('#t-label') as HTMLElement).textContent=`the ${n}${ordinal(n)} prime`;t.classList.add('show');this.toastTimer=performance.now();} }

  private updateUI(now: number) {
    if(this.toastTimer&&now-this.toastTimer>1800){const t=document.getElementById('toast');if(t){t.classList.remove('show');this.toastTimer=0;}}
    if(now-this.lastFrame>400){const fps=Math.round(1000/(now-this.lastFrame));
      (document.getElementById('s-n') as HTMLElement).textContent=this.N.toLocaleString();
      (document.getElementById('s-primes') as HTMLElement).textContent=this.pointCount.toLocaleString();
      (document.getElementById('s-dir') as HTMLElement).textContent=FWD_NAMES[this.K];
      (document.getElementById('s-pos') as HTMLElement).textContent=`${this.curPos.x.toFixed(0)}, ${this.curPos.y.toFixed(0)}, ${this.curPos.z.toFixed(0)}`;
      (document.getElementById('s-fps') as HTMLElement).textContent=String(fps);
    }
  }

  dispose() { this.scene.traverse(o=>{if(o instanceof THREE.Mesh){o.geometry.dispose();if(Array.isArray(o.material))o.material.forEach(m=>m.dispose());else o.material.dispose();}});this.renderer.dispose();this.labelRenderer.domElement.remove(); }
}