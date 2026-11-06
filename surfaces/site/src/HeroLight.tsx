import { useEffect, useRef } from "react";

// The light behind the headline: three slow ribbons in the site's own accent, their cores going
// ice-white, drawn with alpha so they read as light on dark and a tint on light. Half resolution (it is soft by nature),
// paused off screen and in a hidden tab, one still frame under reduced motion, and a plain CSS
// gradient (the canvas's own background) wherever WebGL isn't there.

const VERT = "attribute vec2 p;void main(){gl_Position=vec4(p,0.,1.);}";
const FRAG = `precision mediump float;
uniform vec2 r;uniform float t;uniform vec3 c;uniform float k;
float h(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float n(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);
  return mix(mix(h(i),h(i+vec2(1.,0.)),f.x),mix(h(i+vec2(0.,1.)),h(i+vec2(1.,1.)),f.x),f.y);}
float fbm(vec2 p){float v=0.,a=.5;for(int i=0;i<4;i++){v+=a*n(p);p=p*2.03+vec2(1.7,9.2);a*=.5;}return v;}
void main(){
  vec2 uv=gl_FragCoord.xy/r;vec2 p=vec2(uv.x*r.x/r.y,uv.y);float s=t*.06;float light=0.;
  for(int i=0;i<3;i++){float fi=float(i);
    float warp=fbm(vec2(p.x*1.2+s*(.4+fi*.15),fi*3.1+s*.2))-.5;
    float y=.34+fi*.13+.1*sin(p.x*(1.3+fi*.4)+s*(1.+fi*.3)+fi)+warp*.35;
    float w=.03+.05*fbm(vec2(p.x*2.-s,fi));float d=(uv.y-y)/w;
    light+=exp(-d*d)*(.55+.45*fbm(vec2(p.x*3.+s*2.,fi*7.)))*(1.-fi*.22);}
  float side=smoothstep(.34,.84,uv.x)*(1.-smoothstep(.97,1.,uv.x));
  float glow=smoothstep(.9,0.,length((uv-vec2(.72,.52))*vec2(1.,1.4)))*.3;
  vec3 col=mix(c,vec3(.93,.95,1.),clamp(light*.4,0.,.65));
  gl_FragColor=vec4(col,clamp((light*side+glow)*k,0.,1.));
}`;

function accent(): [number, number, number] {
  const probe = document.createElement("span");
  probe.style.color = "var(--accent)";
  document.body.appendChild(probe);
  const m = getComputedStyle(probe).color.match(/\d+(\.\d+)?/g)?.map(Number) ?? [138, 140, 240];
  probe.remove();
  return [m[0] / 255, m[1] / 255, m[2] / 255];
}

export function HeroLight() {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current!;
    const gl = canvas.getContext("webgl", { alpha: true, premultipliedAlpha: false, antialias: false });
    if (!gl) return;
    const shader = (type: number, src: string) => {
      const s = gl.createShader(type)!;
      gl.shaderSource(s, src);
      gl.compileShader(s);
      return s;
    };
    const prog = gl.createProgram()!;
    gl.attachShader(prog, shader(gl.VERTEX_SHADER, VERT));
    gl.attachShader(prog, shader(gl.FRAGMENT_SHADER, FRAG));
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) return;
    gl.useProgram(prog);
    gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    const loc = gl.getAttribLocation(prog, "p");
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
    const u = (name: string) => gl.getUniformLocation(prog, name);
    canvas.dataset.live = "1";

    const dark = window.matchMedia("(prefers-color-scheme: dark)");
    const still = window.matchMedia("(prefers-reduced-motion: reduce)");
    const tint = () => {
      gl.uniform3fv(u("c"), accent());
      gl.uniform1f(u("k"), dark.matches ? 0.55 : 0.3);
    };
    tint();
    dark.addEventListener("change", tint);

    let raf = 0;
    let visible = true;
    const t0 = performance.now();
    const draw = () => {
      const w = Math.max(1, Math.round(canvas.clientWidth * 0.5));
      const h = Math.max(1, Math.round(canvas.clientHeight * 0.5));
      if (canvas.width !== w || canvas.height !== h) {
        canvas.width = w;
        canvas.height = h;
        gl.viewport(0, 0, w, h);
      }
      gl.uniform2f(u("r"), w, h);
      gl.uniform1f(u("t"), still.matches ? 40 : (performance.now() - t0) / 1000 + 40);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
      if (visible && !still.matches && !document.hidden) raf = requestAnimationFrame(draw);
    };
    const resume = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(draw);
    };
    const io = new IntersectionObserver(([e]) => {
      visible = e.isIntersecting;
      if (visible) resume();
    });
    io.observe(canvas);
    document.addEventListener("visibilitychange", resume);
    still.addEventListener("change", resume);
    resume();
    return () => {
      cancelAnimationFrame(raf);
      io.disconnect();
      document.removeEventListener("visibilitychange", resume);
      still.removeEventListener("change", resume);
      dark.removeEventListener("change", tint);
    };
  }, []);

  return <canvas ref={ref} className="hero-light" aria-hidden="true" />;
}
