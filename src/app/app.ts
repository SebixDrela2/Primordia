import { Component, ViewChild, ElementRef, OnInit, OnDestroy } from '@angular/core';
import { SceneService } from './scene.service';

@Component({
  selector: 'app-root',
  standalone: true,
  template: `
    <div id="loader"><div class="loader-content"><div class="loader-title">PRIME PIPES</div><div class="loader-bar"></div><div class="loader-subtitle">sifting primes · painting light</div></div></div>
    <div id="hud"><h1>PRIME PIPES</h1><div class="subtitle">// balanced prime spiral · WebGL</div><div class="keys"><kbd>WASD</kbd><span class="dim">·</span><span>fly</span><kbd>SPACE</kbd><span class="dim">·</span><span>ascend</span><kbd>Q</kbd><span class="dim">·</span><span>descend</span><kbd>SHIFT</kbd><span class="dim">·</span><span>boost</span><kbd>P</kbd><span class="dim">·</span><span>pause</span><kbd>F</kbd><span class="dim">·</span><span>fit</span><kbd>R</kbd><span class="dim">·</span><span>restart</span><kbd>L</kbd><span class="dim">·</span><span>labels</span></div></div>
    <div id="stats"><span><span class="label">n</span><span class="value" id="s-n">0</span></span><span><span class="label">primes</span><span class="value" id="s-primes">0</span></span><span><span class="label">dir</span><span class="value" id="s-dir">+X</span></span><span><span class="label">pos</span><span class="value" id="s-pos">0, 0, 0</span></span><span><span class="label">fps</span><span class="value" id="s-fps">0</span></span></div>
    <div id="vignette"></div><div id="flash"></div>
    <div id="toast"><div class="big">PRIME <span id="t-n">0</span></div><div class="small" id="t-label">the 0th prime</div></div>
    <div id="reticle"></div><div id="film"></div>
    <canvas #canvas></canvas>
  `,
  styles: [`
    :root{--accent:#50dcff;--accent2:#ff5cb8;--bg:#000;--text:#e8e8f0;--glass:rgba(10,12,20,0.6);--border:rgba(80,220,255,0.25);--easing:cubic-bezier(0.34,1.56,0.64,1);--shadow:0 24px 80px rgba(0,0,0,.7);}
    *,*::before,*::after{box-sizing:border-box} html,body{margin:0;padding:0;overflow:hidden;background:var(--bg);color:var(--text);font-family:'JetBrains Mono','SF Mono',Menlo,Consolas,monospace;font-size:clamp(11px,0.95vw,13px);-webkit-font-smoothing:antialiased} canvas{display:block}
    #loader{position:fixed;inset:0;z-index:1000;display:grid;place-items:center;background:radial-gradient(ellipse at center,#0a0a15 0%,#000 70%);transition:opacity .8s cubic-bezier(.4,0,.2,1)} #loader.hidden{opacity:0;pointer-events:none}
    .loader-content{text-align:center;animation:fadeUp 1s var(--easing) both} .loader-title{font-size:clamp(1.6rem,3.2vw,2.8rem);font-weight:700;letter-spacing:.35em;background:linear-gradient(135deg,#ff5cb8 0%,#ffdd55 25%,#50dcff 50%,#b45cff 75%,#ff5cb8 100%);-webkit-background-clip:text;background-clip:text;color:transparent;background-size:200% 100%;animation:rainbowShift 3s linear infinite}
    .loader-bar{width:min(320px,60vw);height:2px;margin:2.2rem auto 0;background:rgba(255,255,255,.08);border-radius:1px;overflow:hidden;position:relative} .loader-bar::after{content:'';position:absolute;inset:0;background:linear-gradient(90deg,transparent,#ff5cb8,#ffdd55,#50dcff,transparent);animation:slide 1.4s ease-in-out infinite}
    .loader-subtitle{margin-top:1.2rem;font-size:.7em;letter-spacing:.3em;opacity:.55;text-transform:uppercase}
    #hud{position:fixed;top:24px;left:24px;z-index:100;padding:20px 24px;background:var(--glass);backdrop-filter:blur(32px) saturate(200%);-webkit-backdrop-filter:blur(32px) saturate(200%);border:1px solid var(--border);border-radius:16px;pointer-events:none;user-select:none;box-shadow:0 0 0 1px rgba(255,255,255,.05) inset,var(--shadow);animation:slideIn .9s var(--easing) .2s both}
    #hud h1{margin:0 0 14px;font-size:1rem;letter-spacing:.25em;background:linear-gradient(90deg,#ff5cb8,#ffdd55,#50dcff,#b45cff);background-size:200% 100%;-webkit-background-clip:text;background-clip:text;color:transparent;font-weight:800;animation:rainbowShift 5s linear infinite;text-transform:uppercase}
    #hud .subtitle{font-size:.68em;letter-spacing:.28em;opacity:.55;text-transform:uppercase;margin-bottom:12px;color:var(--text)}
    #hud .keys{display:grid;grid-template-columns:auto auto 1fr;gap:6px 12px;font-size:.85em;line-height:1.6;align-items:center}
    #hud kbd{display:inline-block;padding:2px 8px;background:rgba(80,220,255,.1);border:1px solid rgba(80,220,255,.3);border-radius:5px;font-family:inherit;font-size:.88em;color:var(--accent);font-weight:600;min-width:24px;text-align:center} #hud .dim{opacity:.55}
    #stats{position:fixed;bottom:24px;left:24px;right:24px;z-index:100;padding:16px 22px;background:var(--glass);backdrop-filter:blur(32px) saturate(200%);-webkit-backdrop-filter:blur(32px) saturate(200%);border:1px solid var(--border);border-radius:16px;display:flex;gap:32px;flex-wrap:wrap;font-size:.82em;pointer-events:none;user-select:none;box-shadow:0 0 0 1px rgba(255,255,255,.05) inset,var(--shadow);animation:slideIn .9s var(--easing) .4s both}
    #stats span{display:flex;gap:10px;align-items:center} #stats .label{opacity:.5;text-transform:uppercase;letter-spacing:.16em;font-size:.85em} #stats .value{color:var(--accent);font-weight:700;font-variant-numeric:tabular-nums;text-shadow:0 0 14px rgba(80,220,255,.5)}
    #vignette{position:fixed;inset:0;z-index:50;pointer-events:none;background:radial-gradient(ellipse at center,transparent 35%,rgba(0,0,0,.28) 70%,rgba(0,0,0,.7) 100%)}
    #flash{position:fixed;inset:0;z-index:199;pointer-events:none;opacity:0;background:rgba(255,255,255,0.0);transition:opacity 120ms linear}
    .prime-label{pointer-events:none;user-select:none;white-space:nowrap;font:800 14px 'JetBrains Mono',monospace;letter-spacing:.06em} .prime-label span{display:inline-block;text-shadow:0 0 3px currentColor,0 0 8px currentColor,0 0 16px currentColor,0 0 32px rgba(0,0,0,.95);animation:labelPop .55s var(--easing) both}
    #toast{position:fixed;top:50%;left:50%;transform:translate(-50%,-50%) scale(.7);z-index:200;pointer-events:none;user-select:none;text-align:center;opacity:0;transition:transform .4s var(--easing),opacity .4s var(--easing)} #toast.show{opacity:1;transform:translate(-50%,-50%) scale(1)}
    #toast .big{font:900 clamp(1.2rem,6vw,4rem) 'JetBrains Mono',monospace;letter-spacing:.04em;background:linear-gradient(90deg,#ff5cb8,#ffdd55,#50dcff,#b45cff);-webkit-background-clip:text;background-clip:text;color:transparent;text-shadow:0 0 30px rgba(80,220,255,.35);display:block;line-height:1} #toast .small{font-size:.8em;letter-spacing:.25em;opacity:.75;text-transform:uppercase;margin-top:6px;color:var(--text)}
    #reticle{position:fixed;top:50%;left:50%;width:20px;height:20px;z-index:95;pointer-events:none;transform:translate(-50%,-50%)} #reticle::before,#reticle::after{content:'';position:absolute;top:50%;left:50%;width:2px;height:14px;background:var(--accent);transform:translate(-50%,-50%);opacity:.55} #reticle::after{width:14px;height:2px} #reticle::after,#reticle::before{box-shadow:0 0 8px var(--accent)}
    #film{position:fixed;inset:0;z-index:210;pointer-events:none;background:linear-gradient(rgba(18,16,22,0.03) 50%, rgba(0,0,0,0.03) 50%), linear-gradient(90deg, rgba(255,255,255,0.02) 1%, transparent 1%);background-size:100% 4px, 3px 100%;mix-blend-mode:overlay;opacity:0.6}
    @keyframes labelPop{from{transform:scale(.2);opacity:0;filter:blur(4px)}to{transform:scale(1);opacity:1;filter:blur(0)}} @keyframes slideIn{from{transform:translateY(24px);opacity:0}to{transform:translateY(0);opacity:1}} @keyframes fadeUp{from{transform:translateY(30px);opacity:0}to{transform:translateY(0);opacity:1}} @keyframes slide{0%{transform:translateX(-100%)}100%{transform:translateX(100%)}} @keyframes rainbowShift{0%{background-position:0% 50%}100%{background-position:200% 50%}}
  `]
})
export class AppComponent implements OnInit, OnDestroy {
  @ViewChild('canvas') canvasRef!: ElementRef<HTMLCanvasElement>;
  constructor(private scene: SceneService) {}
  ngOnInit() { setTimeout(() => { this.scene.init(this.canvasRef.nativeElement); document.getElementById('loader')!.classList.add('hidden'); }, 120); }
  ngOnDestroy() { this.scene.dispose?.(); }
}