import type { Page } from "@playwright/test";
import { MONO, PALETTE, SANS, mark, type Theme } from "./palette";

// The README's hero: the website's light (three slow ribbons in the accent, cores going
// ice-white, surfaces/site/src/HeroLight.tsx) behind the headline, drawn as a loop. The site's
// shader drifts forever; this one walks a circle through the same noise and moves its waves by
// whole turns, so the last frame leads back into the first.
const VERT = "attribute vec2 p;void main(){gl_Position=vec4(p,0.,1.);}";
const FRAG = `precision highp float;
uniform vec2 r;uniform float a;uniform vec3 c;uniform vec3 core;uniform float k;
float h(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float n(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);
  return mix(mix(h(i),h(i+vec2(1.,0.)),f.x),mix(h(i+vec2(0.,1.)),h(i+vec2(1.,1.)),f.x),f.y);}
float fbm(vec2 p){float v=0.,s=.5;for(int i=0;i<4;i++){v+=s*n(p);p=p*2.03+vec2(1.7,9.2);s*=.5;}return v;}
void main(){
  vec2 uv=gl_FragCoord.xy/r;vec2 p=vec2(uv.x*r.x/r.y,uv.y);vec2 q=vec2(cos(a),sin(a));float light=0.;
  for(int i=0;i<3;i++){float fi=float(i);
    float warp=fbm(vec2(p.x*1.1,fi*3.1)+q*(.5+fi*.15))-.5;
    float y=.28+fi*.17+.11*sin(p.x*(1.2+fi*.35)+a*(1.+fi)+fi*1.7)+warp*.38;
    float w=.035+.055*fbm(vec2(p.x*1.8,fi)+q*.7);float d=(uv.y-y)/w;
    light+=(exp(-d*d)+.3*exp(-d*d*.1))*(.55+.45*fbm(vec2(p.x*2.6,fi*7.)+q))*(1.-fi*.2);}
  float side=smoothstep(.46,.93,uv.x)*smoothstep(0.,.2,uv.y)*smoothstep(1.,.8,uv.y);
  float glow=smoothstep(.75,0.,length((uv-vec2(.84,.5))*vec2(1.,1.8)))*.3;
  vec3 col=mix(c,core,clamp(light*.34,0.,.6));
  gl_FragColor=vec4(col,clamp((light*side+glow)*k,0.,1.));
}`;

/** The hero card: wordmark, eyebrow, headline, one line, three facts; the light behind. */
export function hero(theme: Theme, w: number, h: number, big = false) {
  const c = PALETTE[theme];
  const gap = big ? 26 : 20;
  return `<div id="card" style="position:relative;box-sizing:border-box;width:${w}px;height:${h}px;
      padding:${big ? 72 : 54}px ${big ? 84 : 72}px;background:${c.paper};color:${c.ink};font-family:${SANS};
      display:flex;flex-direction:column;justify-content:center;border:1px solid ${c.line};
      ${big ? "" : "border-radius:16px;"}overflow:hidden">
    <canvas id="light" style="position:absolute;inset:0;width:100%;height:100%"></canvas>
    <div style="position:relative;display:flex;flex-direction:column;gap:${gap}px">
      ${mark(theme, big ? 26 : 22)}
      <div style="font:500 12px/1 ${MONO};letter-spacing:0.14em;text-transform:uppercase;color:${c.accent}">A coworker that lives on your computer</div>
      <div style="font-size:${big ? 76 : 62}px;line-height:1.05;font-weight:600;letter-spacing:-0.028em">The AI you can tell everything.</div>
      <div style="font-size:${big ? 23 : 20}px;line-height:1.5;color:${c.muted};max-width:${big ? 860 : 760}px">It runs on your machine, asks before it acts, and shows you everything that left.</div>
      <div style="font:400 ${big ? 15 : 14}px/1 ${MONO};color:${c.faint}">no account needed &nbsp;·&nbsp; no telemetry &nbsp;·&nbsp; runs with the Wi-Fi off</div>
    </div>
  </div>`;
}

const rgb = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);

/** Compiles the light into #light and leaves `window.__light(phase)` to draw one frame. */
export async function lightUp(page: Page, theme: Theme) {
  await page.evaluate(
    ({ vert, frag, color, core, k }) => {
      const canvas = document.getElementById("light") as HTMLCanvasElement;
      canvas.width = Math.round(canvas.clientWidth * devicePixelRatio);
      canvas.height = Math.round(canvas.clientHeight * devicePixelRatio);
      const gl = canvas.getContext("webgl", { alpha: true, premultipliedAlpha: false, preserveDrawingBuffer: true })!;
      const shader = (type: number, src: string) => {
        const s = gl.createShader(type)!;
        gl.shaderSource(s, src);
        gl.compileShader(s);
        if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) ?? "shader");
        return s;
      };
      const prog = gl.createProgram()!;
      gl.attachShader(prog, shader(gl.VERTEX_SHADER, vert));
      gl.attachShader(prog, shader(gl.FRAGMENT_SHADER, frag));
      gl.linkProgram(prog);
      gl.useProgram(prog);
      gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
      gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
      const loc = gl.getAttribLocation(prog, "p");
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
      gl.viewport(0, 0, canvas.width, canvas.height);
      const u = (name: string) => gl.getUniformLocation(prog, name);
      gl.uniform2f(u("r"), canvas.width, canvas.height);
      gl.uniform3fv(u("c"), color);
      gl.uniform3fv(u("core"), core);
      gl.uniform1f(u("k"), k);
      (window as unknown as { __light: (phase: number) => void }).__light = (phase) => {
        gl.uniform1f(u("a"), phase * Math.PI * 2);
        gl.drawArrays(gl.TRIANGLES, 0, 3);
        gl.finish();
      };
    },
    // Cores go ice-white on dark; on light, white would vanish into the page, so they deepen.
    { vert: VERT, frag: FRAG, color: rgb(PALETTE[theme].accent), core: theme === "dark" ? [0.93, 0.95, 1] : rgb("#2f32a0"), k: theme === "dark" ? 0.92 : 0.5 },
  );
}

/** Draws the light at `phase` (0..1, one loop) and waits for it to reach the screen. */
export async function lightAt(page: Page, phase: number) {
  await page.evaluate((p) => (window as unknown as { __light: (p: number) => void }).__light(p), phase);
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
}
