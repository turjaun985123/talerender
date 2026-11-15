// src/core/lib.ts
var W = 1080;
var H = 1920;
var ctx;
function setCtx(c) {
  ctx = c;
}
var clamp = (v, a = 0, b = 1) => Math.max(a, Math.min(b, v));
var lerp = (a, b, t) => a + (b - a) * t;
var seg = (t, a, b) => clamp((t - a) / (b - a || 0.000001));
var eob = (t) => {
  const c1 = 1.70158, c3 = c1 + 1;
  return 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
};
var TAU = Math.PI * 2;

// src/core/timing.ts
var T = { dur: 0, lines: [], words: [] };
async function loadTiming() {
  const get = async (f) => {
    const r = await fetch(f, { cache: "no-store" });
    if (!r.ok)
      throw new Error(`missing ${f}: run voice.py + words.py (or estimate.py)`);
    return r.json();
  };
  const lines = await get("data/lines.json"), words = await get("data/words.json");
  T.lines = lines.lines;
  T.words = words;
  T.dur = lines.duration;
  return T;
}
var lineWords = (i) => T.words.filter((w) => w.line === i);

// src/core/captions.ts
var CAP = { y: 1340, size: 88, maxW: 960 };
var FONT = `900 ${CAP.size}px Inter`;
var fitCache = new Map;
function measure(text) {
  let m = fitCache.get(text);
  if (!m) {
    const w = ctx.measureText(text).width;
    m = { w, fit: Math.min(1, CAP.maxW / w) };
    fitCache.set(text, m);
  }
  return m;
}
var caps = [];
var SHORT = /^(a|an|the|to|of|in|on|at|by|and|but|or|so|is|it|he|she|we|i|his|her|my|no|up|for|was)$/i;
function autoChunks(ws) {
  const out = [];
  for (let i = 0;i < ws.length; i++) {
    const w = ws[i], n = ws[i + 1], bare = w.replace(/[^A-Za-z']/g, "");
    if (n && SHORT.test(bare) && !/[,.!?;:]$/.test(w) && (w + n).length <= 11) {
      out.push([i, i + 1]);
      i++;
    } else
      out.push([i, i]);
  }
  return out;
}
function buildCaptions(script) {
  caps = [];
  script.lines.forEach((ln, i) => {
    const ws = lineWords(i);
    if (!ws.length)
      return;
    const textWords = ln.text.split(/\s+/).filter(Boolean);
    const spoken = ws.length === textWords.length ? textWords : ws.map((w) => w.w);
    if (ln.caps) {
      const nm = (w) => String(w).toLowerCase().replace(/[^a-z0-9']+/g, "");
      let from = 0;
      ln.caps.split("|").map((c) => c.trim()).filter(Boolean).forEach((c, j) => {
        const first = nm(c.split(/\s+/)[0]);
        let idx = j === 0 ? 0 : ws.findIndex((w, k) => k > from && nm(w.w) === first);
        if (idx < 0)
          idx = Math.min(ws.length - 1, from + 1);
        from = idx;
        caps.push({ line: i, s: ws[idx].s - 0.03, e: 0, text: c.replace(/\*/g, ""), auto: false });
      });
    } else
      autoChunks(spoken).forEach(([a, b]) => caps.push({ line: i, s: ws[a].s - 0.03, e: 0, text: spoken.slice(a, b + 1).join(" "), auto: true }));
  });
  caps.forEach((c, k) => {
    const next = caps[k + 1], lineEnd = T.lines[c.line].end;
    c.e = next ? next.s - lineEnd > 0.6 && next.line !== c.line ? lineEnd + 0.25 : next.s : T.dur;
  });
  return caps;
}
function captionMetrics() {
  ctx.save();
  ctx.font = FONT;
  ctx.letterSpacing = "0px";
  const out = caps.map((c) => {
    const m = measure(c.text);
    return { line: c.line, s: c.s, e: c.e, text: c.text, auto: c.auto, rawW: +m.w.toFixed(1), fit: +m.fit.toFixed(3), renderedW: +(m.w * m.fit).toFixed(1) };
  });
  ctx.restore();
  return out;
}
var captionList = () => {
  const ms = captionMetrics();
  return caps.map((c, k) => ({ ...c, rawW: ms[k].rawW, fit: ms[k].fit, renderedW: ms[k].renderedW, words: [{ txt: c.text }] }));
};
function captionText(text, x = 540, y = CAP.y, sc = 1) {
  ctx.save();
  ctx.font = FONT;
  ctx.letterSpacing = "0px";
  const fit = measure(text).fit;
  ctx.translate(x, y);
  ctx.scale(sc * fit, sc * fit);
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.lineJoin = "round";
  ctx.fillStyle = "rgba(0,0,0,0.45)";
  ctx.fillText(text, 3, 9);
  ctx.lineWidth = 15;
  ctx.strokeStyle = "#0B0B10";
  ctx.strokeText(text, 0, 0);
  ctx.fillStyle = "#FFFFFF";
  ctx.fillText(text, 0, 0);
  ctx.restore();
}
function drawCaption(t) {
  const c = caps.find((c) => t >= c.s && t < c.e);
  if (!c)
    return;
  const lt = t - c.s;
  captionText(c.text, 540, CAP.y, lt < 0.1 ? lerp(0.86, 1, eob(seg(lt, 0, 0.1))) : 1);
}

// src/core/engine.ts
import * as THREE from "three";
import { EffectComposer } from "three/addons/postprocessing/EffectComposer.js";
import { RenderPass } from "three/addons/postprocessing/RenderPass.js";
import { UnrealBloomPass } from "three/addons/postprocessing/UnrealBloomPass.js";
import { BokehPass } from "three/addons/postprocessing/BokehPass.js";
import { ShaderPass } from "three/addons/postprocessing/ShaderPass.js";
import { OutputPass } from "three/addons/postprocessing/OutputPass.js";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
var W2 = 1080;
var H2 = 1920;
var Q = new URLSearchParams(typeof location !== "undefined" ? location.search : "");
var qsVal = (name, dflt, ok) => {
  const s = Q.get(name);
  const n = s === null || s === "" ? NaN : +s;
  return Number.isFinite(n) && ok(n) ? n : dflt;
};
var RES = qsVal("res", 1, (v) => v > 0 && v <= 1);
var AA = qsVal("aa", 4, (v) => [0, 2, 4, 8].includes(v));
var SHADOW = qsVal("shadow", 2048, (v) => [512, 1024, 2048, 4096].includes(v));
var GRADE = {
  uniforms: { tDiffuse: { value: null }, uVig: { value: 0.35 }, uGrain: { value: 0.025 }, uTime: { value: 0 }, uWarm: { value: 0.04 }, uSat: { value: 1.12 } },
  vertexShader: "varying vec2 vUv; void main(){ vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }",
  fragmentShader: `uniform sampler2D tDiffuse; uniform float uVig, uGrain, uTime, uWarm, uSat; varying vec2 vUv;
    float h(vec2 p){ return fract(sin(dot(p, vec2(12.9898, 78.233)) + uTime * 7.13) * 43758.5453); }
    void main(){ vec4 c = texture2D(tDiffuse, vUv); vec2 d = vUv - 0.5; d.x *= 0.5625;
      c.rgb *= 1.0 - uVig * smoothstep(0.25, 0.75, length(d) * 1.4);
      c.rgb += (h(vUv * 1000.0) - 0.5) * uGrain; c.r += uWarm * 0.5; c.b -= uWarm * 0.5; float l = dot(c.rgb, vec3(0.299, 0.587, 0.114)); c.rgb = mix(vec3(l), c.rgb, uSat); gl_FragColor = c; }`
};
function makeEngine(canvas, { w = W2, h = H2, samples = 4, shadowSize = 2048 } = {}) {
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, preserveDrawingBuffer: true, powerPreference: "high-performance" });
  renderer.setSize(w, h, false);
  renderer.setPixelRatio(1);
  renderer.toneMapping = THREE.NeutralToneMapping;
  renderer.toneMappingExposure = 1;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  renderer.shadowMap.autoUpdate = false;
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  const pmrem = new THREE.PMREMGenerator(renderer), envMap = pmrem.fromScene(new RoomEnvironment, 0.04).texture;
  const rt = new THREE.WebGLRenderTarget(w, h, { type: THREE.HalfFloatType, samples });
  renderer.userData = { shadowSize };
  const composer = new EffectComposer(renderer, rt);
  composer.setSize(w, h);
  const rp = new RenderPass(new THREE.Scene, new THREE.PerspectiveCamera);
  composer.addPass(rp);
  const bokeh = new BokehPass(rp.scene, rp.camera, { focus: 3, aperture: 0.002, maxblur: 0.008 });
  composer.addPass(bokeh);
  const bloom = new UnrealBloomPass(new THREE.Vector2(w, h), 0.35, 0.6, 0.85);
  composer.addPass(bloom);
  composer.addPass(new OutputPass);
  const grade = new ShaderPass(GRADE);
  composer.addPass(grade);
  return {
    renderer,
    envMap,
    composer,
    render(scene, camera, look = {}) {
      renderer.shadowMap.needsUpdate = true;
      rp.scene = scene;
      rp.camera = camera;
      bokeh.scene = scene;
      bokeh.camera = camera;
      bokeh.enabled = (look.aperture ?? 0.0015) > 0;
      const bu = bokeh.uniforms;
      bu.focus.value = look.focus ?? 3;
      bu.aperture.value = look.aperture ?? 0.0015;
      bu.maxblur.value = look.maxblur ?? 0.006;
      bloom.strength = look.bloom ?? 0.3;
      bloom.threshold = look.bloomThreshold ?? 0.85;
      bloom.radius = 0.6;
      renderer.toneMappingExposure = look.exposure ?? 1;
      grade.uniforms.uVig.value = look.vignette ?? 0.18;
      grade.uniforms.uGrain.value = look.grain ?? 0.008;
      grade.uniforms.uSat.value = look.sat ?? 1.12;
      grade.uniforms.uWarm.value = look.warm ?? 0.02;
      grade.uniforms.uTime.value = look.t ?? 0;
      composer.render();
    }
  };
}
function lightRig(scene, { key = "#FFE2C2", keyI = 3.2, keyPos = [2.5, 4, 3], rimCol = "#BFD8FF", rimI = 2.2, sky = "#FFF4E6", ground = "#8C6E55", hemiI = 0.9, shadowBox = 3, target = [0, 1, 0] } = {}) {
  const g = new THREE.Group;
  scene.add(g);
  const k = new THREE.DirectionalLight(key, keyI);
  k.position.set(...keyPos);
  k.target.position.set(...target);
  k.castShadow = true;
  k.shadow.mapSize.set(SHADOW, SHADOW);
  k.shadow.bias = -0.0004;
  k.shadow.normalBias = 0.02;
  k.shadow.radius = 6;
  Object.assign(k.shadow.camera, { left: -shadowBox, right: shadowBox, top: shadowBox, bottom: -shadowBox, near: 0.1, far: 20 });
  k.shadow.camera.updateProjectionMatrix();
  g.add(k, k.target);
  const r = new THREE.DirectionalLight(rimCol, rimI);
  r.position.set(-3, 3, -3);
  g.add(r);
  g.add(new THREE.HemisphereLight(sky, ground, hemiI));
  return { key: k, rim: r, group: g };
}
function camera(fov = 30) {
  return new THREE.PerspectiveCamera(fov, W2 / H2, 0.05, 800);
}

// src/core/toon.ts
import * as THREE2 from "three";
var INK = "#1F1C33";
var LW = 9;
var R2D = Math.PI / 180;
var MOODS = ["neutral", "smug", "happy", "laugh", "shock", "worried", "angry", "sad", "sly", "proud", "tired", "dizzy", "determined", "embarrassed", "awe", "suspicious", "relieved", "puzzled", "deadpan"];
var HATS = ["none", "cap", "bowler", "boater", "beanie", "visor", "tophat", "bun", "flower", "bow", "mailcap"];
var NECKS = ["none", "bowtie", "tie", "scarf", "collar"];
var clamp3 = (v, a, b) => Math.max(a, Math.min(b, v));
function shadeOf(hex, k = 0.78) {
  const n = parseInt(hex.slice(1), 16), r = n >> 16 & 255, gg = n >> 8 & 255, b = n & 255;
  const f = (v) => Math.max(0, Math.min(255, Math.round(v * k)));
  return "#" + ((1 << 24) + (f(r) << 16) + (f(gg) << 8) + f(b)).toString(16).slice(1);
}
var STAGE = { W: 512, H: 1024, CW: 1280, CH: 1408, OX: 384, OY: 256 };
var TOONS = [];
var OVERFLOW_CANVASES = [];
function registerOverflowCanvas(canvas, label, opts = {}) {
  if (!canvas || OVERFLOW_CANVASES.some((e) => e.canvas === canvas))
    return;
  OVERFLOW_CANVASES.push({ canvas, res: opts.res ?? 1, label, pose: opts.pose });
}
function scanCanvasOverflow(canvas, res = 1, strip = 1) {
  const counts = { top: 0, right: 0, bottom: 0, left: 0 };
  if (!canvas || !canvas.width || !canvas.height)
    return counts;
  const g = canvas.getContext("2d");
  if (!g)
    return counts;
  const { width: W, height: Hh } = canvas;
  const s = Math.max(1, Math.round(strip * res));
  const scan = (x, y, w, h) => {
    if (w <= 0 || h <= 0)
      return 0;
    const d = g.getImageData(x, y, w, h).data;
    let n = 0;
    for (let i = 3;i < d.length; i += 4)
      if (d[i] > 32)
        n++;
    return n;
  };
  counts.top = scan(0, 0, W, Math.min(s, Hh));
  counts.bottom = scan(0, Hh - Math.min(s, Hh), W, Math.min(s, Hh));
  counts.left = scan(0, 0, Math.min(s, W), Hh);
  counts.right = scan(W - Math.min(s, W), 0, Math.min(s, W), Hh);
  return counts;
}
function drawToon(g, look = {}, st = {}) {
  return drawToonLayer(g, look, st, "full");
}
function drawToonLayer(g, look = {}, st = {}, layer) {
  const L = { body: "#E8734A", head: "#F7F7FA", hat: "none", neck: "none", pattern: "plain", extra: [], ...look };
  const S = { mood: "neutral", look: [0, 0], blink: 0, talk: 0, armL: [8, 6], armR: [8, 6], legs: "stand", phase: 0, lean: 0, bob: 0, squash: 0, ...st, headTilt: st.headTilt ?? 0, headTurn: st.headTurn ?? 0, camPitch: st.camPitch ?? 0 };
  g.save();
  g.lineCap = "round";
  g.lineJoin = "round";
  if (S.flip) {
    g.translate(512, 0);
    g.scale(-1, 1);
  }
  g.translate(256, 1000);
  g.rotate((S.lean || 0) * R2D);
  g.translate(-256, -1000);
  const sit = S.legs === "sit", jump = S.legs === "jump";
  const sitS = S.sitDrop ?? 220.2;
  const by = (sit ? (426 - sitS) / 0.98 : 0) + (L.kid ? 190 : 0) + (S.bob || 0) + (jump ? -60 : 0);
  const bx = 256, top = 330 + by, bot = 600 + by * 0.98, bw = 92;
  const hipY = bot - 26;
  const hx = 256, hy = 178 + by * 0.99, r = 138, pivotY = 318 + by * 0.99;
  if (layer === "headonly") {
    drawHeadPart(g, L, S, hx, hy, r, pivotY);
    g.restore();
    return {};
  }
  g.strokeStyle = INK;
  g.lineWidth = 15;
  const leg = (sx, side) => {
    let kx, ky, fx, fy;
    if (sit) {
      const kx2 = sx + side * 30, ky2 = hipY + 44;
      fx = sx + side * 34;
      fy = Math.min(1000, hipY + 220.2);
      g.beginPath();
      g.moveTo(sx, hipY);
      g.lineTo(kx2, ky2);
      g.lineTo(fx, fy);
      g.stroke();
      g.fillStyle = INK;
      g.beginPath();
      g.ellipse(fx + side * 10, fy - 4, 20, 10, 0, 0, Math.PI * 2);
      g.fill();
      return;
    } else if (S.legs === "walk" || S.legs === "run") {
      const amp = S.legs === "run" ? 1 : 0.55, a = Math.sin(S.phase + (side > 0 ? Math.PI : 0)) * amp;
      const lift = Math.max(0, -Math.cos(S.phase + (side > 0 ? Math.PI : 0))) * (S.legs === "run" ? 70 : 34);
      kx = sx + a * 22 + side * 6;
      ky = hipY + 200 - lift * 0.6;
      fx = sx + a * 34 + side * 8;
      fy = 1000 - lift;
    } else if (jump) {
      kx = sx + side * 40;
      ky = hipY + 170;
      fx = sx + side * 18;
      fy = 940;
    } else {
      kx = sx + side * 4;
      ky = hipY + 210;
      fx = sx + side * 10;
      fy = 1000;
    }
    g.beginPath();
    g.moveTo(sx, hipY);
    g.quadraticCurveTo(kx, ky, fx, fy);
    g.stroke();
    g.fillStyle = INK;
    g.beginPath();
    g.ellipse(fx + side * 10, fy - 4, 20, 10, 0, 0, Math.PI * 2);
    g.fill();
  };
  leg(bx - 42, -1);
  leg(bx + 42, 1);
  const body = new Path2D;
  body.moveTo(bx - 58, top + 10);
  body.bezierCurveTo(bx - 80, top + 40, bx - bw - 6, bot - 120, bx - bw, bot - 50);
  body.bezierCurveTo(bx - bw + 4, bot + 6, bx + bw - 4, bot + 6, bx + bw, bot - 50);
  body.bezierCurveTo(bx + bw + 6, bot - 120, bx + 80, top + 40, bx + 58, top + 10);
  body.bezierCurveTo(bx + 30, top - 8, bx - 30, top - 8, bx - 58, top + 10);
  body.closePath();
  g.fillStyle = L.body;
  g.fill(body);
  g.save();
  g.clip(body);
  drawPattern(g, L, bx, top, bot, bw);
  g.fillStyle = L.shade || shadeOf(L.body, 0.8);
  g.globalAlpha = 0.85;
  g.beginPath();
  g.ellipse(bx + bw + 40, (top + bot) / 2 + 20, 70, 260, 0, 0, Math.PI * 2);
  g.fill();
  g.globalAlpha = 0.25;
  g.fillStyle = "#FFFFFF";
  g.beginPath();
  g.ellipse(bx - 50, top + 90, 16, 60, 0.2, 0, Math.PI * 2);
  g.fill();
  g.globalAlpha = 1;
  g.restore();
  g.strokeStyle = INK;
  g.lineWidth = LW;
  g.stroke(body);
  drawNeckwear(g, L, bx, top);
  const hands = {};
  const arm = (side, [a, e], hold) => {
    const sx = bx + side * 66, sy = top + 46;
    const u = 118, lo = 112;
    const a1 = a * R2D, a2 = (a + e) * R2D;
    const ex = sx + side * Math.sin(a1) * u, ey = sy + Math.cos(a1) * u;
    const hx = ex + side * Math.sin(a2) * lo, hy = ey + Math.cos(a2) * lo;
    g.strokeStyle = INK;
    g.lineWidth = 14;
    g.beginPath();
    g.moveTo(sx, sy);
    g.quadraticCurveTo(ex, ey, hx, hy);
    g.stroke();
    if (hold) {
      g.save();
      hold(g, hx, hy, side * (a + e) * R2D, side);
      g.restore();
    }
    g.fillStyle = INK;
    g.beginPath();
    g.ellipse(hx, hy, 15, 17, 0, 0, Math.PI * 2);
    g.fill();
    hands[side < 0 ? "L" : "R"] = [hx, hy];
  };
  arm(-1, S.armL, S.holdL);
  arm(1, S.armR, S.holdR);
  if (layer === "full")
    drawHeadPart(g, L, S, hx, hy, r, pivotY);
  g.restore();
  return hands;
}
function drawHeadPart(g, L, S, hx, hy, r, pivotY) {
  const turn = S.headTurn || 0;
  const rp = r * (1 + 0.04 * (S.camPitch || 0));
  const fx = hx + turn * 30;
  g.save();
  g.translate(256, pivotY);
  g.rotate((S.headTilt || 0) * R2D);
  g.translate(-256, -pivotY);
  g.save();
  g.translate(hx, hy);
  g.scale(1 + (S.squash || 0) * -0.3, 1 + (S.squash || 0));
  g.translate(-hx, -hy);
  if (L.hat === "bun") {
    g.fillStyle = L.hatCol || "#6B4A3A";
    g.beginPath();
    g.arc(hx + 10 + turn * 18, hy - rp - 8, 46, 0, Math.PI * 2);
    g.fill();
    g.strokeStyle = INK;
    g.lineWidth = LW;
    g.stroke();
  }
  const head = new Path2D;
  head.ellipse(hx, hy, rp, rp * 0.97, 0, 0, Math.PI * 2);
  g.fillStyle = L.head;
  g.fill(head);
  g.save();
  g.clip(head);
  g.fillStyle = shadeOf(L.head, 0.9);
  g.beginPath();
  g.arc(hx + 52 + turn * 30, hy + 44, rp, 0, Math.PI * 2);
  g.rect(hx - 400, hy - 400, 800, 800);
  g.fill("evenodd");
  g.restore();
  g.strokeStyle = INK;
  g.lineWidth = LW + 1;
  g.stroke(head);
  drawFace(g, L, S, fx, hy);
  drawHat(g, L, hx + turn * 18, hy, rp);
  g.restore();
  g.restore();
}
function drawPattern(g, L, bx, top, bot, bw) {
  const pc = L.patCol || shadeOf(L.body, 0.7);
  if (L.pattern === "check") {
    g.strokeStyle = pc;
    g.globalAlpha = 0.55;
    g.lineWidth = 6;
    for (let x = bx - 120;x < bx + 120; x += 34) {
      g.beginPath();
      g.moveTo(x, top - 20);
      g.lineTo(x, bot + 20);
      g.stroke();
    }
    for (let y = top;y < bot + 20; y += 34) {
      g.beginPath();
      g.moveTo(bx - 130, y);
      g.lineTo(bx + 130, y);
      g.stroke();
    }
    g.lineWidth = 2;
    g.globalAlpha = 0.5;
    g.strokeStyle = L.patCol2 || "#B5372E";
    for (let x = bx - 103;x < bx + 120; x += 68) {
      g.beginPath();
      g.moveTo(x, top - 20);
      g.lineTo(x, bot + 20);
      g.stroke();
    }
    g.globalAlpha = 1;
  } else if (L.pattern === "stripes") {
    g.fillStyle = pc;
    for (let x = bx - 130;x < bx + 130; x += 36)
      g.fillRect(x, top - 20, 14, bot - top + 40);
  } else if (L.pattern === "overalls") {
    g.fillStyle = L.patCol || "#3E5FA8";
    g.beginPath();
    g.roundRect(bx - 62, top + 110, 124, 110, 14);
    g.fill();
    g.fillRect(bx - 130, top + 190, 260, 200);
    g.strokeStyle = shadeOf(g.fillStyle, 0.75);
    g.lineWidth = 4;
    g.setLineDash([8, 7]);
    g.strokeRect(bx - 48, top + 124, 96, 80);
    g.setLineDash([]);
    g.fillStyle = L.patCol || "#3E5FA8";
    g.fillRect(bx - 58, top, 16, 120);
    g.fillRect(bx + 42, top, 16, 120);
    g.fillStyle = "#F1C24B";
    [[-50, 118], [50, 118]].forEach(([dx, dy]) => {
      g.beginPath();
      g.arc(bx + dx, top + dy, 8, 0, 7);
      g.fill();
    });
  } else if (L.pattern === "vest") {
    g.fillStyle = L.patCol || "#3B3346";
    g.beginPath();
    g.moveTo(bx - 140, top - 10);
    g.lineTo(bx - 8, top + 120);
    g.lineTo(bx - 8, bot + 20);
    g.lineTo(bx - 140, bot + 20);
    g.fill();
    g.beginPath();
    g.moveTo(bx + 140, top - 10);
    g.lineTo(bx + 8, top + 120);
    g.lineTo(bx + 8, bot + 20);
    g.lineTo(bx + 140, bot + 20);
    g.fill();
    g.fillStyle = "#E9C46A";
    [150, 195, 240].forEach((dy) => {
      g.beginPath();
      g.arc(bx + 26, top + dy, 6, 0, 7);
      g.fill();
    });
  } else if (L.pattern === "stamps" || L.stamps) {
    const n = L.stamps ?? 6;
    const spots = [[-50, 70, -0.2], [30, 60, 0.15], [-10, 140, 0.05], [50, 150, -0.12], [-60, 180, 0.18], [10, 215, -0.1], [-30, 100, 0.3], [60, 105, 0.08], [20, 175, 0.25], [-45, 240, -0.05]];
    spots.slice(0, n).forEach(([dx, dy, r], i) => {
      g.save();
      g.translate(bx + dx, top + dy);
      g.rotate(r);
      const cols = ["#D8343F", "#3C6FB6", "#2E8A5A", "#E2B33C", "#8C6FC9"];
      g.fillStyle = "#FFFDF4";
      g.strokeStyle = INK;
      g.lineWidth = 3;
      g.beginPath();
      g.rect(-22, -27, 44, 54);
      g.fill();
      g.stroke();
      g.fillStyle = cols[i % 5];
      g.fillRect(-15, -20, 30, 32);
      g.fillStyle = "#FFFDF4";
      g.font = "900 13px Inter";
      g.textAlign = "center";
      g.fillText(i % 2 ? "5¢" : "2¢", 0, 22);
      g.beginPath();
      g.arc(0, -6, 8, 0, 7);
      g.fill();
      g.restore();
    });
  } else if (L.pattern === "apron") {
    g.fillStyle = L.patCol || "#F4EFE4";
    g.beginPath();
    g.roundRect(bx - 70, top + 90, 140, bot - top, 18);
    g.fill();
    g.strokeStyle = shadeOf(L.patCol || "#F4EFE4", 0.8);
    g.lineWidth = 4;
    g.beginPath();
    g.roundRect(bx - 40, top + 190, 80, 60, 8);
    g.stroke();
  }
}
function drawNeckwear(g, L, bx, top) {
  const c = L.neckCol || "#D8343F";
  g.strokeStyle = INK;
  g.lineWidth = 6;
  if (L.neck === "bowtie") {
    const y = top + 24, bt = new Path2D;
    bt.moveTo(bx, y);
    bt.bezierCurveTo(bx - 30, y - 30, bx - 64, y - 26, bx - 62, y);
    bt.bezierCurveTo(bx - 64, y + 26, bx - 30, y + 30, bx, y);
    bt.bezierCurveTo(bx + 30, y - 30, bx + 64, y - 26, bx + 62, y);
    bt.bezierCurveTo(bx + 64, y + 26, bx + 30, y + 30, bx, y);
    g.fillStyle = c;
    g.fill(bt);
    if (L.dots) {
      g.save();
      g.clip(bt);
      g.fillStyle = "#FFFFFF";
      for (let i = -3;i <= 3; i++)
        for (let j = -1;j <= 1; j++) {
          g.beginPath();
          g.arc(bx + i * 18 + (j & 1) * 9, y + j * 15, 4.5, 0, 7);
          g.fill();
        }
      g.restore();
    }
    g.stroke(bt);
    g.fillStyle = shadeOf(c, 0.75);
    g.beginPath();
    g.roundRect(bx - 13, y - 13, 26, 26, 6);
    g.fill();
    g.stroke();
  } else if (L.neck === "tie") {
    g.fillStyle = c;
    g.beginPath();
    g.moveTo(bx - 14, top + 10);
    g.lineTo(bx + 14, top + 10);
    g.lineTo(bx + 20, top + 140);
    g.lineTo(bx, top + 170);
    g.lineTo(bx - 20, top + 140);
    g.closePath();
    g.fill();
    g.stroke();
  } else if (L.neck === "scarf") {
    g.fillStyle = c;
    g.beginPath();
    g.moveTo(bx + 20, top + 20);
    g.lineTo(bx + 58, top + 130);
    g.lineTo(bx + 24, top + 138);
    g.lineTo(bx + 6, top + 26);
    g.closePath();
    g.fill();
    g.stroke();
    g.beginPath();
    g.moveTo(bx - 70, top + 4);
    g.quadraticCurveTo(bx, top + 44, bx + 70, top + 4);
    g.lineTo(bx + 66, top + 34);
    g.quadraticCurveTo(bx, top + 76, bx - 66, top + 34);
    g.closePath();
    g.fill();
    g.stroke();
    g.strokeStyle = shadeOf(c, 0.75);
    g.lineWidth = 4;
    g.beginPath();
    g.moveTo(bx - 50, top + 26);
    g.quadraticCurveTo(bx, top + 56, bx + 50, top + 26);
    g.stroke();
  } else if (L.neck === "collar") {
    g.fillStyle = "#FFFFFF";
    [-1, 1].forEach((s) => {
      g.beginPath();
      g.moveTo(bx, top + 18);
      g.lineTo(bx + s * 56, top + 2);
      g.lineTo(bx + s * 40, top + 46);
      g.closePath();
      g.fill();
      g.stroke();
    });
  }
}
function eye(g, x, y, S, side, L) {
  const [lx, ly] = S.look;
  const m = S.mood;
  const bl = S.blink || 0;
  g.fillStyle = INK;
  g.strokeStyle = INK;
  if (m === "happy" || m === "laugh" || m === "relieved" || m === "proud" && side > 0 && false) {
    g.lineWidth = 10;
    g.beginPath();
    g.arc(x, y + 10, 20, Math.PI * 1.15, Math.PI * 1.85);
    g.stroke();
    return;
  }
  if (m === "dizzy") {
    g.lineWidth = 6;
    g.beginPath();
    for (let a = 0;a < 12; a += 0.3) {
      const rr = a * 1.6;
      g.lineTo(x + Math.cos(a * side) * rr, y + Math.sin(a * side) * rr);
    }
    g.stroke();
    return;
  }
  if (m === "angry" && false)
    ;
  if (m === "shock") {
    g.fillStyle = "#FFFFFF";
    g.lineWidth = 6;
    g.beginPath();
    g.ellipse(x, y, 26, 30, 0, 0, 7);
    g.fill();
    g.stroke();
    g.fillStyle = INK;
    g.beginPath();
    g.arc(x + lx * 8, y + ly * 8, 7, 0, 7);
    g.fill();
    return;
  }
  if (m === "awe") {
    g.fillStyle = "#FFFFFF";
    g.lineWidth = 6;
    g.beginPath();
    g.ellipse(x, y, 26, 30, 0, 0, 7);
    g.fill();
    g.stroke();
    g.fillStyle = INK;
    g.beginPath();
    g.arc(x + lx * 8, y + ly * 8, 9, 0, 7);
    g.fill();
    g.fillStyle = "#FFFFFF";
    g.beginPath();
    g.arc(x + lx * 8 - 3, y + ly * 8 - 3, 3, 0, 7);
    g.fill();
    return;
  }
  if (bl > 0.6) {
    g.lineWidth = 8;
    g.beginPath();
    g.moveTo(x - 14, y + 4);
    g.quadraticCurveTo(x, y + 10, x + 14, y + 4);
    g.stroke();
    return;
  }
  const px = x + lx * 9, py = y + ly * 8;
  g.beginPath();
  g.ellipse(px, py, 13, 17 * (1 - bl * 0.6) * (m === "suspicious" && side < 0 ? 0.55 : 1) * (m === "deadpan" ? 0.8 : 1), 0, 0, 7);
  g.fill();
  if (m === "smug" || m === "tired" || m === "sly" || m === "proud" || m === "determined" || m === "deadpan") {
    const lid = m === "tired" ? 6 : m === "deadpan" ? 8 : 0;
    g.fillStyle = L.head;
    g.fillRect(x - 24, y - 28, 48, 26 + lid);
    g.lineWidth = 7;
    g.beginPath();
    g.moveTo(x - 20, y - 2 + lid);
    g.lineTo(x + 20, y - 2 + lid);
    g.stroke();
  }
  if (m === "sad" || m === "worried") {
    g.fillStyle = "#FFFFFF";
    g.beginPath();
    g.arc(px - 3, py - 5, 3.5, 0, 7);
    g.fill();
  }
}
function drawFace(g, L, S, fx, hy) {
  const m = S.mood, ex = 46 * (1 - 0.18 * Math.abs(S.headTurn || 0)), ey = hy + 4 + (S.camPitch || 0) * 8;
  const brow = {
    neutral: [4, 0],
    smug: [-8, 10],
    happy: [8, 0],
    laugh: [10, 0],
    shock: [24, 0],
    worried: [10, -16],
    angry: [-10, 22],
    sad: [8, -18],
    sly: [-6, 12],
    proud: [6, 4],
    tired: [-2, 0],
    dizzy: [14, 0],
    determined: [-4, 16],
    embarrassed: [12, -10],
    awe: [26, 0],
    suspicious: [-2, 6],
    relieved: [14, -4],
    puzzled: [6, 4],
    deadpan: [2, 0]
  }[m] || [4, 0];
  g.strokeStyle = INK;
  g.lineWidth = 12;
  [-1, 1].forEach((s) => {
    let lift = brow[0], tilt = brow[1] * s;
    if ((m === "smug" || m === "sly") && s > 0) {
      lift = 16;
      tilt = -4;
    }
    if (m === "suspicious" && s < 0) {
      lift = 18;
      tilt = -6;
    }
    if (m === "puzzled") {
      if (s < 0) {
        lift = 16;
        tilt = -2;
      } else {
        lift = -2;
        tilt = 8;
      }
    }
    const y = ey - 44 - lift;
    g.beginPath();
    g.moveTo(fx + s * (ex - 26), y + tilt * 0.5);
    g.quadraticCurveTo(fx + s * ex, y - 7, fx + s * (ex + 26), y - tilt * 0.5);
    g.stroke();
  });
  eye(g, fx - ex, ey, S, -1, L);
  eye(g, fx + ex, ey, S, 1, L);
  if (L.extra.includes("glasses")) {
    g.strokeStyle = INK;
    g.lineWidth = 5;
    [-1, 1].forEach((s) => {
      g.beginPath();
      g.arc(fx + s * ex, ey, 30, 0, 7);
      g.stroke();
    });
    g.beginPath();
    g.moveTo(fx - 16, ey - 4);
    g.lineTo(fx + 16, ey - 4);
    g.stroke();
  }
  if (L.extra.includes("monocle")) {
    g.strokeStyle = "#C9A13B";
    g.lineWidth = 5;
    g.beginPath();
    g.arc(fx + ex, ey, 30, 0, 7);
    g.stroke();
    g.beginPath();
    g.moveTo(fx + ex + 22, ey + 22);
    g.quadraticCurveTo(fx + ex + 40, ey + 90, fx + ex + 10, ey + 140);
    g.stroke();
  }
  if (L.extra.includes("blush") || m === "laugh" || m === "proud" || m === "embarrassed") {
    g.fillStyle = "rgba(240,120,130,0.35)";
    [-1, 1].forEach((s) => {
      g.beginPath();
      g.ellipse(fx + s * 78, ey + 40, 22, 12, 0, 0, 7);
      g.fill();
    });
  }
  if (L.extra.includes("freckles")) {
    g.fillStyle = "#C98A64";
    [-1, 1].forEach((s) => [[0, 0], [12, 8], [-10, 10]].forEach(([dx, dy]) => {
      g.beginPath();
      g.arc(fx + s * (74 + dx), ey + 36 + dy, 3.5, 0, 7);
      g.fill();
    }));
  }
  const my = ey + 62, t = S.talk || 0;
  g.strokeStyle = INK;
  g.fillStyle = INK;
  g.lineWidth = 8;
  const openMouth = (w, h, tongue = true) => {
    g.beginPath();
    g.ellipse(fx, my + h * 0.3, w, Math.max(4, h), 0, 0, 7);
    g.fillStyle = "#3A1A2A";
    g.fill();
    g.stroke();
    if (tongue && h > 12) {
      g.save();
      g.clip();
      g.fillStyle = "#E2697A";
      g.beginPath();
      g.ellipse(fx, my + h * 1.1, w * 0.7, h * 0.6, 0, 0, 7);
      g.fill();
      g.restore();
    }
  };
  if (m === "shock")
    openMouth(16, 20 + t * 6);
  else if (m === "laugh") {
    g.beginPath();
    g.moveTo(fx - 40, my - 6);
    g.quadraticCurveTo(fx, my - 2, fx + 40, my - 6);
    g.quadraticCurveTo(fx + 34, my + 52, fx, my + 52);
    g.quadraticCurveTo(fx - 34, my + 52, fx - 40, my - 6);
    g.closePath();
    g.fillStyle = "#3A1A2A";
    g.fill();
    g.save();
    g.clip();
    g.fillStyle = "#E2697A";
    g.beginPath();
    g.ellipse(fx, my + 50, 24, 18, 0, 0, 7);
    g.fill();
    g.fillStyle = "#FFFFFF";
    g.fillRect(fx - 40, my - 10, 80, 12);
    g.restore();
    g.stroke();
  } else if (t > 0.05)
    openMouth(14 + t * 10, 6 + t * 18);
  else if (m === "happy" || m === "proud") {
    g.beginPath();
    g.moveTo(fx - 28, my);
    g.quadraticCurveTo(fx, my + 26, fx + 28, my);
    g.stroke();
  } else if (m === "smug" || m === "sly") {
    g.beginPath();
    g.moveTo(fx - 22, my + 6);
    g.quadraticCurveTo(fx + 4, my + 16, fx + 24, my - 4);
    g.stroke();
  } else if (m === "worried") {
    g.beginPath();
    g.moveTo(fx - 26, my + 8);
    g.bezierCurveTo(fx - 12, my - 4, fx + 8, my + 14, fx + 26, my + 2);
    g.stroke();
  } else if (m === "angry") {
    g.beginPath();
    g.moveTo(fx - 30, my + 12);
    g.quadraticCurveTo(fx, my - 8, fx + 30, my + 12);
    g.stroke();
    g.lineWidth = 5;
    g.fillStyle = "#FFFFFF";
    g.beginPath();
    g.roundRect(fx - 24, my - 2, 48, 14, 6);
    g.fill();
    g.stroke();
  } else if (m === "sad") {
    g.beginPath();
    g.moveTo(fx - 24, my + 12);
    g.quadraticCurveTo(fx, my - 4, fx + 24, my + 12);
    g.stroke();
  } else if (m === "tired" || m === "dizzy" || m === "deadpan") {
    g.beginPath();
    g.moveTo(fx - 18, my + 6);
    g.lineTo(fx + 18, my + 6);
    g.stroke();
  } else if (m === "determined") {
    g.beginPath();
    g.moveTo(fx - 24, my + 5);
    g.lineTo(fx + 24, my + 1);
    g.stroke();
  } else if (m === "embarrassed") {
    g.beginPath();
    g.moveTo(fx - 17, my + 4);
    g.quadraticCurveTo(fx - 8, my - 2, fx, my + 4);
    g.quadraticCurveTo(fx + 8, my + 10, fx + 17, my + 4);
    g.stroke();
  } else if (m === "awe")
    openMouth(11, 13, false);
  else if (m === "suspicious") {
    g.beginPath();
    g.moveTo(fx - 22, my);
    g.quadraticCurveTo(fx, my + 2, fx + 24, my + 9);
    g.stroke();
  } else if (m === "puzzled") {
    g.beginPath();
    g.moveTo(fx - 16, my + 5);
    g.quadraticCurveTo(fx, my + 1, fx + 16, my - 1);
    g.stroke();
  } else if (m === "relieved") {
    g.beginPath();
    g.moveTo(fx - 20, my);
    g.quadraticCurveTo(fx, my + 18, fx + 20, my);
    g.stroke();
    g.fillStyle = "#3A1A2A";
    g.beginPath();
    g.ellipse(fx, my + 16, 5, 3, 0, 0, 7);
    g.fill();
  } else {
    g.beginPath();
    g.moveTo(fx - 20, my + 2);
    g.quadraticCurveTo(fx, my + 14, fx + 20, my + 2);
    g.stroke();
  }
  if (L.extra.includes("mustache")) {
    g.fillStyle = L.mustacheCol || "#3A2A22";
    g.strokeStyle = INK;
    g.lineWidth = 4;
    g.beginPath();
    g.moveTo(fx, my - 18);
    g.bezierCurveTo(fx - 20, my - 32, fx - 60, my - 22, fx - 64, my - 2);
    g.bezierCurveTo(fx - 44, my - 8, fx - 20, my - 4, fx, my - 10);
    g.bezierCurveTo(fx + 20, my - 4, fx + 44, my - 8, fx + 64, my - 2);
    g.bezierCurveTo(fx + 60, my - 22, fx + 20, my - 32, fx, my - 18);
    g.fill();
    g.stroke();
  }
  if (L.extra.includes("pencil")) {
    g.save();
    g.translate(fx + 132, hy - 50);
    g.rotate(-0.5);
    g.fillStyle = "#F2C230";
    g.fillRect(-8, -60, 16, 120);
    g.fillStyle = "#E58A9A";
    g.fillRect(-8, -70, 16, 14);
    g.strokeStyle = INK;
    g.lineWidth = 4;
    g.strokeRect(-8, -70, 16, 130);
    g.restore();
  }
  if (S.sweat || m === "worried") {
    const k = S.sweat ?? 1;
    g.globalAlpha = Math.min(1, k);
    g.fillStyle = "#8FD3F4";
    g.strokeStyle = INK;
    g.lineWidth = 4;
    g.beginPath();
    const sx = fx + 104, sy = hy - 70;
    g.moveTo(sx, sy - 26);
    g.quadraticCurveTo(sx + 18, sy, sx, sy + 12);
    g.quadraticCurveTo(sx - 18, sy, sx, sy - 26);
    g.fill();
    g.stroke();
    g.globalAlpha = 1;
  }
  if (S.tears || m === "sad") {
    g.fillStyle = "#8FD3F4";
    [-1, 1].forEach((s) => {
      g.beginPath();
      g.ellipse(fx + s * (ex + 6), ey + 40, 7, 14, 0, 0, 7);
      g.fill();
    });
  }
}
function drawHat(g, L, hx, hy, r) {
  const c = L.hatCol || "#7A5236", top = hy - r * 0.97;
  g.strokeStyle = INK;
  g.lineWidth = LW;
  g.fillStyle = c;
  const band = (y, w, col) => {
    g.fillStyle = col;
    g.fillRect(hx - w, y - 12, w * 2, 24);
    g.strokeRect(hx - w, y - 12, w * 2, 24);
  };
  if (L.hat === "cap") {
    g.beginPath();
    g.ellipse(hx + 6, top + 34, r * 0.98, 62, -0.05, Math.PI, 0);
    g.closePath();
    g.fill();
    g.stroke();
    g.fillStyle = shadeOf(c, 0.75);
    g.beginPath();
    g.ellipse(hx - 24, top + 40, r * 0.78, 22, 0.05, 0, Math.PI);
    g.fill();
    g.stroke();
    g.fillStyle = shadeOf(c, 1.15);
    g.beginPath();
    g.arc(hx + 6, top - 26, 12, 0, 7);
    g.fill();
    g.stroke();
  } else if (L.hat === "bowler") {
    g.beginPath();
    g.ellipse(hx, top + 26, r * 1.02, 22, 0, 0, 7);
    g.fill();
    g.stroke();
    g.beginPath();
    g.ellipse(hx, top + 14, r * 0.66, 80, 0, Math.PI, 0);
    g.closePath();
    g.fill();
    g.stroke();
    band(top + 4, r * 0.64, L.band || shadeOf(c, 0.6));
  } else if (L.hat === "boater") {
    g.fillStyle = c;
    g.beginPath();
    g.ellipse(hx, top + 30, r * 1.3, 26, 0, 0, 7);
    g.fill();
    g.stroke();
    g.beginPath();
    g.roundRect(hx - r * 0.66, top - 46, r * 1.32, 76, [16, 16, 4, 4]);
    g.fill();
    g.stroke();
    band(top + 8, r * 0.66, L.band || "#3C6FB6");
    g.strokeStyle = shadeOf(c, 0.82);
    g.lineWidth = 3;
    for (let x = -r * 0.6;x < r * 0.6; x += 16) {
      g.beginPath();
      g.moveTo(hx + x, top - 40);
      g.lineTo(hx + x + 6, top - 6);
      g.stroke();
    }
  } else if (L.hat === "beanie") {
    g.beginPath();
    g.ellipse(hx, top + 50, r * 0.95, 96, 0, Math.PI, 0);
    g.closePath();
    g.fill();
    g.stroke();
    g.fillStyle = shadeOf(c, 0.8);
    g.beginPath();
    g.roundRect(hx - r * 0.98, top + 30, r * 1.96, 40, 18);
    g.fill();
    g.stroke();
    g.fillStyle = L.band || "#F4EFE4";
    g.beginPath();
    g.arc(hx, top - 52, 22, 0, 7);
    g.fill();
    g.stroke();
  } else if (L.hat === "visor") {
    g.fillStyle = "rgba(70,190,120,0.85)";
    g.beginPath();
    g.moveTo(hx - r * 1, top + 70);
    g.quadraticCurveTo(hx, top + 10, hx + r * 1, top + 70);
    g.lineTo(hx + r * 1.15, top + 98);
    g.quadraticCurveTo(hx, top + 54, hx - r * 1.15, top + 98);
    g.closePath();
    g.fill();
    g.stroke();
    g.fillStyle = c;
    g.beginPath();
    g.moveTo(hx - r * 0.96, top + 64);
    g.quadraticCurveTo(hx, top + 4, hx + r * 0.96, top + 64);
    g.lineTo(hx + r * 0.9, top + 80);
    g.quadraticCurveTo(hx, top + 24, hx - r * 0.9, top + 80);
    g.closePath();
    g.fill();
    g.stroke();
  } else if (L.hat === "tophat") {
    g.beginPath();
    g.ellipse(hx, top + 24, r * 1, 20, 0, 0, 7);
    g.fill();
    g.stroke();
    g.beginPath();
    g.roundRect(hx - r * 0.6, top - 140, r * 1.2, 164, 8);
    g.fill();
    g.stroke();
    band(top - 4, r * 0.6, L.band || "#B5372E");
  } else if (L.hat === "bow") {
    const fx = hx + r * 0.5, fy = top + 26, c2 = L.hatCol || "#D8343F";
    g.fillStyle = c2;
    g.lineWidth = 6;
    [-1, 1].forEach((s2) => {
      g.beginPath();
      g.moveTo(fx, fy);
      g.bezierCurveTo(fx + s2 * 30, fy - 46, fx + s2 * 66, fy - 30, fx + s2 * 60, fy);
      g.bezierCurveTo(fx + s2 * 66, fy + 30, fx + s2 * 30, fy + 40, fx, fy);
      g.fill();
      g.stroke();
    });
    g.beginPath();
    g.arc(fx, fy, 15, 0, 7);
    g.fill();
    g.stroke();
    g.fillStyle = L.hairCol || "#6B4A3A";
    [-1, 1].forEach((s2) => {
      g.beginPath();
      g.ellipse(hx + s2 * (r + 4), hy + 10, 22, 50, s2 * -0.2, 0, 7);
      g.fill();
      g.stroke();
    });
  } else if (L.hat === "mailcap") {
    g.fillStyle = c;
    g.beginPath();
    g.roundRect(hx - r * 0.72, top - 40, r * 1.44, 70, [20, 20, 4, 4]);
    g.fill();
    g.stroke();
    g.fillStyle = INK;
    g.beginPath();
    g.ellipse(hx - 10, top + 34, r * 0.82, 18, 0.04, 0, Math.PI);
    g.fill();
    g.fillStyle = "#E2B33C";
    g.beginPath();
    g.arc(hx, top - 6, 13, 0, 7);
    g.fill();
    g.lineWidth = 4;
    g.stroke();
  } else if (L.hat === "flower") {
    const fx = hx + r * 0.62, fy = top + 40;
    for (let i = 0;i < 6; i++) {
      const a = i / 6 * Math.PI * 2;
      g.fillStyle = L.hatCol || "#F29BB5";
      g.beginPath();
      g.arc(fx + Math.cos(a) * 18, fy + Math.sin(a) * 18, 14, 0, 7);
      g.fill();
      g.lineWidth = 4;
      g.stroke();
    }
    g.fillStyle = "#F6C945";
    g.beginPath();
    g.arc(fx, fy, 11, 0, 7);
    g.fill();
    g.stroke();
  }
}
var PROPS = {
  parcels: (g, x, y) => {
    [[0, 0, 150, 90], [10, -88, 120, 80], [-6, -160, 96, 64]].forEach(([dx, dy, w, h], i) => {
      g.fillStyle = ["#C8935A", "#B9844E", "#D3A26A"][i];
      g.strokeStyle = INK;
      g.lineWidth = 6;
      g.beginPath();
      g.rect(x - w / 2 + dx - 40, y - h + dy, w, h);
      g.fill();
      g.stroke();
      g.fillStyle = "#E9D7A5";
      g.fillRect(x + dx - 46, y - h + dy, 12, h);
      g.fillStyle = "#D8343F";
      g.fillRect(x + dx + w / 2 - 70, y - h + dy + 10, 18, 22);
    });
  },
  book: (title = "RULES", open = 0) => (g, x, y) => {
    g.save();
    g.translate(x, y - 70);
    g.strokeStyle = INK;
    g.lineWidth = 6;
    if (open > 0.5) {
      g.fillStyle = "#F4EFE4";
      g.beginPath();
      g.rect(-110, -70, 110, 140);
      g.rect(0, -70, 110, 140);
      g.fill();
      g.stroke();
      g.fillStyle = "#9B958A";
      for (let i = 0;i < 6; i++) {
        g.fillRect(-96, -50 + i * 20, 82, 6);
        g.fillRect(14, -50 + i * 20, 82, 6);
      }
    } else {
      g.fillStyle = "#8A2E2E";
      g.beginPath();
      g.roundRect(-80, -100, 160, 200, 8);
      g.fill();
      g.stroke();
      g.fillStyle = "#E2B33C";
      g.font = "900 30px Inter";
      g.textAlign = "center";
      g.fillText(title, 0, -20);
      g.fillRect(-50, 10, 100, 6);
    }
    g.restore();
  },
  ticket: (g, x, y) => {
    g.save();
    g.translate(x + 10, y - 40);
    g.rotate(-0.2);
    g.fillStyle = "#F2D37A";
    g.strokeStyle = INK;
    g.lineWidth = 5;
    g.beginPath();
    g.roundRect(-70, -40, 140, 80, 6);
    g.fill();
    g.stroke();
    g.fillStyle = INK;
    g.font = "900 24px Inter";
    g.textAlign = "center";
    g.fillText("TICKET", 0, -4);
    g.font = "900 30px Inter";
    g.fillStyle = "#B5372E";
    g.fillText("$$$", 0, 30);
    g.restore();
  },
  purse: (g, x, y) => {
    g.fillStyle = "#7A5236";
    g.strokeStyle = INK;
    g.lineWidth = 5;
    g.beginPath();
    g.ellipse(x, y - 30, 44, 34, 0, 0, 7);
    g.fill();
    g.stroke();
    g.strokeStyle = "#C9A13B";
    g.beginPath();
    g.moveTo(x - 30, y - 56);
    g.lineTo(x + 30, y - 56);
    g.stroke();
    g.fillStyle = "#C9C2B5";
    g.beginPath();
    g.arc(x + 30, y - 6, 4, 0, 7);
    g.fill();
  },
  letters: (g, x, y) => {
    for (let i = 0;i < 3; i++) {
      g.save();
      g.translate(x - 10 + i * 10, y - 40 - i * 8);
      g.rotate(-0.3 + i * 0.2);
      g.fillStyle = i === 1 ? "#F2E6CF" : "#FFFDF4";
      g.strokeStyle = INK;
      g.lineWidth = 4;
      g.beginPath();
      g.rect(-50, -30, 100, 60);
      g.fill();
      g.stroke();
      g.beginPath();
      g.moveTo(-50, -30);
      g.lineTo(0, 6);
      g.lineTo(50, -30);
      g.stroke();
      g.fillStyle = "#D8343F";
      g.fillRect(28, -24, 14, 16);
      g.restore();
    }
  },
  rubberStamp: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 5;
    g.fillStyle = "#7A5236";
    g.beginPath();
    g.roundRect(x - 14, y - 90, 28, 60, 8);
    g.fill();
    g.stroke();
    g.fillStyle = "#B5372E";
    g.beginPath();
    g.roundRect(x - 40, y - 34, 80, 28, 4);
    g.fill();
    g.stroke();
    g.fillStyle = "#7A5236";
    g.beginPath();
    g.arc(x, y - 100, 20, 0, 7);
    g.fill();
    g.stroke();
  },
  stamp: (g, x, y) => {
    g.save();
    g.translate(x, y - 40);
    g.fillStyle = "#FFFDF4";
    g.strokeStyle = INK;
    g.lineWidth = 4;
    g.beginPath();
    g.rect(-24, -30, 48, 60);
    g.fill();
    g.stroke();
    g.fillStyle = "#D8343F";
    g.fillRect(-16, -22, 32, 36);
    g.restore();
  },
  cash: (n = 1) => (g, x, y) => {
    for (let i = 0;i < n; i++)
      bill(g, x - 30, y - 34 - i * 12, 0.06 * (i % 2 ? -1 : 1));
  },
  bill: (g, x, y) => bill(g, x - 10, y - 50, -0.5),
  stack: (g, x, y) => {
    for (let i = 0;i < 5; i++) {
      g.fillStyle = i % 2 ? "#7FBF7A" : "#8FD08A";
      g.strokeStyle = INK;
      g.lineWidth = 5;
      g.beginPath();
      g.roundRect(x - 60, y - 40 - i * 16, 120, 18, 4);
      g.fill();
      g.stroke();
    }
    g.fillStyle = "#E9D7A5";
    g.fillRect(x - 12, y - 120, 24, 82);
    g.strokeRect(x - 12, y - 120, 24, 82);
  },
  paper: (text = "NEWS") => (g, x, y) => {
    g.save();
    g.translate(x, y - 70);
    g.rotate(-0.08);
    g.fillStyle = "#F2EDE0";
    g.strokeStyle = INK;
    g.lineWidth = 5;
    g.beginPath();
    g.rect(-70, -90, 140, 180);
    g.fill();
    g.stroke();
    g.fillStyle = INK;
    g.font = "900 30px Inter";
    g.textAlign = "center";
    g.fillText(text, 0, -50);
    g.fillStyle = "#9B958A";
    for (let i = 0;i < 6; i++)
      g.fillRect(-56, -26 + i * 18, i === 2 ? 60 : 112, 7);
    g.restore();
  },
  mirror: (g, x, y) => {
    g.strokeStyle = "#B8862F";
    g.lineWidth = 12;
    g.beginPath();
    g.moveTo(x, y);
    g.lineTo(x, y - 60);
    g.stroke();
    g.fillStyle = "#E6B850";
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.beginPath();
    g.arc(x, y - 110, 54, 0, 7);
    g.fill();
    g.stroke();
    g.fillStyle = "#BFE6F5";
    g.beginPath();
    g.arc(x, y - 110, 40, 0, 7);
    g.fill();
    g.fillStyle = "rgba(255,255,255,0.8)";
    g.beginPath();
    g.ellipse(x - 14, y - 124, 9, 18, 0.6, 0, 7);
    g.fill();
  },
  cup: (g, x, y) => {
    g.fillStyle = "#F4EFE4";
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.beginPath();
    g.roundRect(x - 30, y - 70, 60, 70, [4, 4, 14, 14]);
    g.fill();
    g.stroke();
    g.beginPath();
    g.arc(x + 34, y - 36, 16, -1.4, 1.4);
    g.stroke();
    g.fillStyle = "#7A4A2A";
    g.fillRect(x - 24, y - 64, 48, 10);
  },
  key: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 5;
    g.fillStyle = "#E6B850";
    g.beginPath();
    g.arc(x, y - 60, 22, 0, 7);
    g.fill();
    g.stroke();
    g.fillRect(x - 5, y - 40, 10, 50);
    g.strokeRect(x - 5, y - 40, 10, 50);
  },
  sign: (text = "SALE", col = "#F2EDE0") => (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#9A6A43";
    g.fillRect(x - 6, y - 150, 12, 160);
    g.strokeRect(x - 6, y - 150, 12, 160);
    g.fillStyle = col;
    g.beginPath();
    g.roundRect(x - 100, y - 250, 200, 110, 10);
    g.fill();
    g.stroke();
    g.fillStyle = INK;
    g.font = "900 44px Inter";
    g.textAlign = "center";
    g.textBaseline = "middle";
    g.fillText(text, x, y - 194);
  },
  axe: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#9A6A43";
    g.beginPath();
    g.roundRect(x - 8, y - 135, 16, 150, 7);
    g.fill();
    g.stroke();
    g.fillStyle = "#C9CDD4";
    g.beginPath();
    g.moveTo(x + 4, y - 138);
    g.lineTo(x + 18, y - 95);
    g.lineTo(x + 2, y - 88);
    g.quadraticCurveTo(x - 30, y - 104, x - 45, y - 150);
    g.quadraticCurveTo(x - 22, y - 140, x + 4, y - 138);
    g.closePath();
    g.fill();
    g.stroke();
    g.fillStyle = "#8A8D91";
    g.fillRect(x - 4, y - 140, 12, 46);
  },
  hammer: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#9A6A43";
    g.beginPath();
    g.roundRect(x - 7, y - 100, 14, 120, 6);
    g.fill();
    g.stroke();
    g.fillStyle = "#B9BEC7";
    g.beginPath();
    g.roundRect(x - 52, y - 128, 104, 32, 6);
    g.fill();
    g.stroke();
    g.fillStyle = "#8A8D91";
    g.fillRect(x - 52, y - 118, 14, 12);
    g.fillRect(x + 38, y - 118, 14, 12);
  },
  wrench: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#B9BEC7";
    g.beginPath();
    g.roundRect(x - 9, y - 95, 18, 113, 8);
    g.fill();
    g.stroke();
    g.beginPath();
    g.arc(x, y - 104, 19, 0, 7);
    g.fill();
    g.stroke();
    g.fillStyle = "#FFFFFF";
    g.beginPath();
    g.moveTo(x, y - 104);
    g.lineTo(x + 20, y - 122);
    g.lineTo(x + 22, y - 96);
    g.closePath();
    g.fill();
  },
  shovel: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#9A6A43";
    g.fillRect(x - 7, y - 140, 14, 200);
    g.strokeRect(x - 7, y - 140, 14, 200);
    g.lineWidth = 7;
    g.beginPath();
    g.roundRect(x - 20, y - 168, 40, 34, 10);
    g.stroke();
    g.fillStyle = "#8A8D91";
    g.beginPath();
    g.moveTo(x - 13, y + 52);
    g.lineTo(x + 13, y + 52);
    g.lineTo(x + 26, y + 128);
    g.quadraticCurveTo(x + 24, y + 168, x, y + 186);
    g.quadraticCurveTo(x - 24, y + 168, x - 26, y + 128);
    g.closePath();
    g.fill();
    g.stroke();
  },
  apple: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#D8343F";
    g.beginPath();
    g.arc(x, y - 26, 24, 0, 7);
    g.fill();
    g.stroke();
    g.strokeStyle = "#6B4A3A";
    g.lineWidth = 5;
    g.beginPath();
    g.moveTo(x, y - 44);
    g.quadraticCurveTo(x + 4, y - 54, x + 8, y - 56);
    g.stroke();
    g.fillStyle = "#5DA83E";
    g.strokeStyle = INK;
    g.lineWidth = 4;
    g.beginPath();
    g.ellipse(x + 14, y - 50, 11, 6, -0.5, 0, 7);
    g.fill();
    g.stroke();
    g.fillStyle = "rgba(255,255,255,0.55)";
    g.beginPath();
    g.ellipse(x - 9, y - 34, 6, 10, 0.4, 0, 7);
    g.fill();
  },
  bottle: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#3E7C59";
    g.beginPath();
    g.roundRect(x - 17, y - 80, 34, 80, 8);
    g.fill();
    g.stroke();
    g.beginPath();
    g.moveTo(x - 17, y - 80);
    g.quadraticCurveTo(x - 8, y - 92, x - 7, y - 100);
    g.lineTo(x + 7, y - 100);
    g.quadraticCurveTo(x + 8, y - 92, x + 17, y - 80);
    g.fill();
    g.fillRect(x - 7, y - 104, 14, 8);
    g.fillStyle = "#C9A13B";
    g.fillRect(x - 9, y - 112, 18, 9);
    g.strokeRect(x - 9, y - 112, 18, 9);
    g.fillStyle = "#F2E6CF";
    g.fillRect(x - 12, y - 62, 24, 30);
  },
  sandwich: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#EFBE7D";
    g.beginPath();
    g.moveTo(x - 40, y - 40);
    g.lineTo(x + 40, y - 40);
    g.lineTo(x, y + 6);
    g.closePath();
    g.fill();
    g.stroke();
    g.strokeStyle = "#5DA83E";
    g.lineWidth = 5;
    g.beginPath();
    g.moveTo(x - 30, y - 26);
    for (let i = 0;i < 4; i++)
      g.quadraticCurveTo(x - 22 + i * 16, y - 34, x - 14 + i * 16, y - 26);
    g.stroke();
    g.fillStyle = "#D8343F";
    g.beginPath();
    g.arc(x - 8, y - 18, 7, 0, 7);
    g.fill();
    g.fillStyle = "#F6C945";
    g.beginPath();
    g.moveTo(x - 24, y - 12);
    g.lineTo(x + 16, y - 12);
    g.lineTo(x - 4, y + 2);
    g.closePath();
    g.fill();
  },
  fish: (g, x, y) => {
    g.save();
    g.translate(x + 22, y - 38);
    g.rotate(-0.6);
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#E8873A";
    g.beginPath();
    g.ellipse(0, 0, 55, 24, 0, 0, 7);
    g.fill();
    g.stroke();
    g.beginPath();
    g.moveTo(-50, 0);
    g.lineTo(-78, -20);
    g.lineTo(-78, 20);
    g.closePath();
    g.fill();
    g.stroke();
    g.fillStyle = "#FFFFFF";
    g.beginPath();
    g.arc(34, -6, 8, 0, 7);
    g.fill();
    g.stroke();
    g.fillStyle = INK;
    g.beginPath();
    g.arc(36, -6, 4, 0, 7);
    g.fill();
    g.strokeStyle = "#C25E1E";
    g.lineWidth = 4;
    [-16, 0, 16].forEach((dx) => {
      g.beginPath();
      g.arc(dx, 0, 12, -0.9, 0.9);
      g.stroke();
    });
    g.restore();
  },
  flower: (g, x, y) => {
    g.strokeStyle = "#3E7C43";
    g.lineWidth = 6;
    g.beginPath();
    g.moveTo(x, y + 10);
    g.quadraticCurveTo(x - 4, y - 30, x, y - 70);
    g.stroke();
    g.fillStyle = "#5DA83E";
    g.strokeStyle = INK;
    g.lineWidth = 4;
    g.beginPath();
    g.ellipse(x - 13, y - 38, 12, 6, -0.6, 0, 7);
    g.fill();
    g.stroke();
    for (let i = 0;i < 6; i++) {
      const a = i / 6 * Math.PI * 2;
      g.fillStyle = "#F29BB5";
      g.beginPath();
      g.arc(x + Math.cos(a) * 15, y - 86 + Math.sin(a) * 15, 12, 0, 7);
      g.fill();
      g.lineWidth = 4;
      g.stroke();
    }
    g.fillStyle = "#F6C945";
    g.beginPath();
    g.arc(x, y - 86, 10, 0, 7);
    g.fill();
    g.stroke();
  },
  egg: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#F7F1E3";
    g.beginPath();
    g.ellipse(x, y - 28, 21, 27, 0, 0, 7);
    g.fill();
    g.stroke();
    g.strokeStyle = "#C9BFA8";
    g.lineWidth = 4;
    g.beginPath();
    g.moveTo(x - 20, y - 30);
    g.lineTo(x - 8, y - 22);
    g.lineTo(x + 2, y - 31);
    g.lineTo(x + 12, y - 23);
    g.lineTo(x + 20, y - 29);
    g.stroke();
  },
  feather: (g, x, y) => {
    g.save();
    g.translate(x, y);
    g.rotate(0.3);
    g.strokeStyle = INK;
    g.lineWidth = 5;
    g.fillStyle = "#BFD9EA";
    g.beginPath();
    g.moveTo(0, 10);
    g.bezierCurveTo(-24, -20, -20, -70, 6, -104);
    g.bezierCurveTo(26, -66, 22, -18, 0, 10);
    g.closePath();
    g.fill();
    g.stroke();
    g.strokeStyle = "#7FA3BC";
    g.lineWidth = 3;
    g.beginPath();
    g.moveTo(0, 10);
    g.quadraticCurveTo(2, -50, 6, -102);
    g.stroke();
    g.restore();
  },
  ball: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#F4EFE4";
    g.beginPath();
    g.arc(x, y - 36, 34, 0, 7);
    g.fill();
    g.stroke();
    g.fillStyle = "#D8343F";
    g.beginPath();
    g.arc(x, y - 36, 34, 0.9, 2.2);
    g.lineTo(x, y - 36);
    g.closePath();
    g.fill();
    g.beginPath();
    g.arc(x, y - 36, 34, 3.6, 4.9);
    g.lineTo(x, y - 36);
    g.closePath();
    g.fill();
    g.fillStyle = "#3C6FB6";
    g.beginPath();
    g.arc(x, y - 36, 34, 5.6, 6.9);
    g.lineTo(x, y - 36);
    g.closePath();
    g.fill();
  },
  trophy: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#E6B850";
    g.beginPath();
    g.moveTo(x - 30, y - 95);
    g.lineTo(x + 30, y - 95);
    g.quadraticCurveTo(x + 28, y - 48, x + 8, y - 40);
    g.lineTo(x - 8, y - 40);
    g.quadraticCurveTo(x - 28, y - 48, x - 30, y - 95);
    g.closePath();
    g.fill();
    g.stroke();
    g.beginPath();
    g.arc(x - 32, y - 72, 13, Math.PI * 0.5, Math.PI * 1.6);
    g.stroke();
    g.beginPath();
    g.arc(x + 32, y - 72, 13, -Math.PI * 0.6, Math.PI * 0.5);
    g.stroke();
    g.fillRect(x - 6, y - 40, 12, 20);
    g.strokeRect(x - 6, y - 40, 12, 20);
    g.fillRect(x - 22, y - 20, 44, 14);
    g.strokeRect(x - 22, y - 20, 44, 14);
    g.fillStyle = "#FFF3C4";
    g.beginPath();
    g.arc(x, y - 72, 9, 0, 7);
    g.fill();
  },
  phone: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#2A2A33";
    g.beginPath();
    g.roundRect(x - 21, y - 78, 42, 76, 8);
    g.fill();
    g.stroke();
    g.fillStyle = "#BFE6F5";
    g.fillRect(x - 15, y - 64, 30, 48);
    g.fillStyle = "#8A8D91";
    g.beginPath();
    g.arc(x, y - 70, 3, 0, 7);
    g.fill();
    g.fillStyle = "#F4EFE4";
    g.beginPath();
    g.arc(x, y - 9, 4, 0, 7);
    g.fill();
  },
  camera: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.fillStyle = "#4A4A52";
    g.beginPath();
    g.roundRect(x - 46, y - 78, 92, 72, 8);
    g.fill();
    g.stroke();
    g.fillStyle = "#6E6A6A";
    g.fillRect(x - 14, y - 92, 24, 14);
    g.strokeRect(x - 14, y - 92, 24, 14);
    g.fillStyle = "#B9BEC7";
    g.beginPath();
    g.arc(x, y - 42, 20, 0, 7);
    g.fill();
    g.stroke();
    g.fillStyle = "#2B4A70";
    g.beginPath();
    g.arc(x, y - 42, 12, 0, 7);
    g.fill();
    g.fillStyle = "rgba(255,255,255,0.7)";
    g.beginPath();
    g.arc(x - 4, y - 46, 4, 0, 7);
    g.fill();
    g.fillStyle = "#D8343F";
    g.beginPath();
    g.arc(x + 36, y - 68, 5, 0, 7);
    g.fill();
  },
  radio: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.beginPath();
    g.moveTo(x + 26, y - 90);
    g.lineTo(x + 64, y - 140);
    g.stroke();
    g.fillStyle = "#D8343F";
    g.beginPath();
    g.arc(x + 64, y - 140, 5, 0, 7);
    g.fill();
    g.stroke();
    g.fillStyle = "#7A5236";
    g.beginPath();
    g.roundRect(x - 52, y - 96, 82, 100, 8);
    g.fill();
    g.stroke();
    g.fillStyle = "#3A2A22";
    for (let r = 0;r < 3; r++)
      for (let c = 0;c < 3; c++) {
        g.beginPath();
        g.arc(x - 34 + c * 13, y - 62 + r * 15, 4, 0, 7);
        g.fill();
      }
    g.fillStyle = "#F2D37A";
    g.beginPath();
    g.arc(x + 8, y - 52, 13, 0, 7);
    g.fill();
    g.stroke();
    g.fillStyle = "#F4EFE4";
    g.fillRect(x - 44, y - 88, 66, 14);
  },
  umbrella: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.beginPath();
    g.moveTo(x, y - 100);
    g.lineTo(x, y + 4);
    g.stroke();
    g.beginPath();
    g.arc(x + 8, y + 6, 8, 0, Math.PI);
    g.stroke();
    g.fillStyle = "#3C6FB6";
    g.beginPath();
    g.moveTo(x - 100, y - 56);
    g.quadraticCurveTo(x, y - 128, x + 100, y - 56);
    g.quadraticCurveTo(x + 66, y - 42, x + 33, y - 56);
    g.quadraticCurveTo(x, y - 42, x - 33, y - 56);
    g.quadraticCurveTo(x - 66, y - 42, x - 100, y - 56);
    g.closePath();
    g.fill();
    g.stroke();
    g.fillStyle = "#2A2A33";
    g.fillRect(x - 3, y - 116, 6, 14);
  },
  lantern: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 6;
    g.beginPath();
    g.arc(x, y - 96, 16, Math.PI, 0);
    g.stroke();
    g.fillStyle = "#3A3A3F";
    g.fillRect(x - 20, y - 96, 40, 12);
    g.strokeRect(x - 20, y - 96, 40, 12);
    g.fillStyle = "#FFDF8A";
    g.beginPath();
    g.roundRect(x - 22, y - 84, 44, 60, 6);
    g.fill();
    g.stroke();
    g.fillStyle = "#FFB84D";
    g.beginPath();
    g.ellipse(x, y - 52, 8, 13, 0, 0, 7);
    g.fill();
    g.fillStyle = "#3A3A3F";
    g.fillRect(x - 24, y - 24, 48, 12);
    g.strokeRect(x - 24, y - 24, 48, 12);
  },
  map: (text = "") => (g, x, y) => {
    g.save();
    g.translate(x, y - 58);
    g.rotate(-0.05);
    g.fillStyle = "#F2E6CF";
    g.strokeStyle = INK;
    g.lineWidth = 5;
    g.beginPath();
    g.rect(-75, -55, 150, 110);
    g.fill();
    g.stroke();
    g.strokeStyle = "#C9BFA8";
    g.lineWidth = 3;
    g.beginPath();
    g.moveTo(-25, -55);
    g.lineTo(-25, 55);
    g.moveTo(25, -55);
    g.lineTo(25, 55);
    g.stroke();
    g.strokeStyle = "#D8343F";
    g.lineWidth = 4;
    g.setLineDash([8, 6]);
    g.beginPath();
    g.moveTo(-58, 34);
    g.quadraticCurveTo(-10, -6, 30, -18);
    g.stroke();
    g.setLineDash([]);
    g.lineWidth = 6;
    g.beginPath();
    g.moveTo(38, -34);
    g.lineTo(56, -16);
    g.moveTo(56, -34);
    g.lineTo(38, -16);
    g.stroke();
    if (text) {
      g.fillStyle = INK;
      g.font = "900 20px Inter";
      g.textAlign = "center";
      g.fillText(text, 0, 46);
    }
    g.restore();
  },
  rope: (g, x, y) => {
    g.strokeStyle = "#9A6A43";
    g.lineWidth = 9;
    [-16, -34, -52].forEach((cy) => {
      g.beginPath();
      g.ellipse(x, y + cy, 40, 15, 0, 0, 7);
      g.stroke();
    });
    g.strokeStyle = INK;
    g.lineWidth = 3;
    [-16, -34, -52].forEach((cy) => {
      g.beginPath();
      g.ellipse(x, y + cy, 40, 15, 0, 0, 7);
      g.stroke();
    });
    g.strokeStyle = "#9A6A43";
    g.lineWidth = 8;
    g.beginPath();
    g.moveTo(x + 34, y - 20);
    g.quadraticCurveTo(x + 48, y + 6, x + 30, y + 26);
    g.stroke();
  },
  medal: (g, x, y) => {
    g.strokeStyle = INK;
    g.lineWidth = 5;
    g.fillStyle = "#3C6FB6";
    g.beginPath();
    g.moveTo(x - 4, y);
    g.lineTo(x - 18, y + 36);
    g.lineTo(x - 6, y + 40);
    g.lineTo(x, y + 10);
    g.closePath();
    g.fill();
    g.stroke();
    g.fillStyle = "#D8343F";
    g.beginPath();
    g.moveTo(x + 4, y);
    g.lineTo(x, y + 10);
    g.lineTo(x + 6, y + 40);
    g.lineTo(x + 18, y + 36);
    g.closePath();
    g.fill();
    g.stroke();
    g.fillStyle = "#E6B850";
    g.beginPath();
    g.arc(x, y + 58, 23, 0, 7);
    g.fill();
    g.stroke();
    g.fillStyle = "#FFF3C4";
    g.beginPath();
    for (let i = 0;i < 5; i++) {
      const a = -Math.PI / 2 + i * (Math.PI * 4 / 5);
      g.lineTo(x + Math.cos(a) * 12, y + 58 + Math.sin(a) * 12);
    }
    g.closePath();
    g.fill();
  }
};
function bill(g, x, y, rot) {
  g.save();
  g.translate(x, y);
  g.rotate(rot);
  g.fillStyle = "#9ED39A";
  g.strokeStyle = INK;
  g.lineWidth = 5;
  g.beginPath();
  g.roundRect(-10, -26, 100, 52, 6);
  g.fill();
  g.stroke();
  g.strokeStyle = "#4F8F55";
  g.lineWidth = 3;
  g.beginPath();
  g.roundRect(-2, -18, 84, 36, 4);
  g.stroke();
  g.fillStyle = "#4F8F55";
  g.beginPath();
  g.arc(40, 0, 11, 0, 7);
  g.fill();
  g.restore();
}
var shadowTex = null;
function blobTex() {
  if (shadowTex)
    return shadowTex;
  const c = document.createElement("canvas");
  c.width = c.height = 128;
  const g = c.getContext("2d");
  const gr = g.createRadialGradient(64, 64, 0, 64, 64, 64);
  gr.addColorStop(0, "rgba(30,20,40,0.75)");
  gr.addColorStop(0.6, "rgba(30,20,40,0.25)");
  gr.addColorStop(1, "rgba(30,20,40,0)");
  g.fillStyle = gr;
  g.fillRect(0, 0, 128, 128);
  shadowTex = new THREE2.CanvasTexture(c);
  shadowTex.colorSpace = THREE2.SRGBColorSpace;
  return shadowTex;
}

class Toon {
  look;
  res;
  name;
  _overflowKey;
  _overflowCache;
  canvas;
  g;
  tex;
  mat;
  mesh;
  blob;
  group;
  pivot;
  key;
  H;
  _facePos;
  st;
  hands;
  perspective;
  envMatch;
  multiPlane;
  groundY = 0;
  headCanvas;
  headG;
  headTex;
  headMesh;
  headPivot;
  _attachments = [];
  _attQ = new THREE2.Quaternion;
  _attOff = new THREE2.Vector3;
  _attW = new THREE2.Vector3;
  _envC = new THREE2.Color;
  _envFog = new THREE2.Color;
  constructor(look = {}, { height = 1.75, res = 2, castShadow = true, perspective = false, multiPlane = false, envMatch = false } = {}) {
    this.look = look;
    this.res = res;
    this.perspective = perspective;
    this.multiPlane = multiPlane;
    this.envMatch = envMatch;
    this.headCanvas = null;
    this.headG = null;
    this.headTex = null;
    this.headMesh = null;
    this.headPivot = null;
    this.name = look.name || `toon#${TOONS.length}`;
    TOONS.push(this);
    this._overflowKey = null;
    this._overflowCache = null;
    this.canvas = document.createElement("canvas");
    this.canvas.width = STAGE.CW * res;
    this.canvas.height = STAGE.CH * res;
    OVERFLOW_CANVASES.push({ canvas: this.canvas, res, label: this.name, pose: () => this.poseSummary() });
    this.g = this.canvas.getContext("2d");
    this.tex = new THREE2.CanvasTexture(this.canvas);
    this.tex.colorSpace = THREE2.SRGBColorSpace;
    this.tex.anisotropy = 4;
    const H = height / 0.96;
    const planeW = H * 1280 / 1024, planeH = H * 1408 / 1024;
    const geo = new THREE2.PlaneGeometry(planeW, planeH);
    geo.translate(0, H * 552 / 1024, 0);
    this.mat = new THREE2.MeshBasicMaterial({ map: this.tex, transparent: true, alphaTest: 0.04, side: THREE2.DoubleSide, toneMapped: false });
    this.mesh = new THREE2.Mesh(geo, this.mat);
    this.mesh.castShadow = castShadow;
    this.mesh.customDepthMaterial = new THREE2.MeshDepthMaterial({ depthPacking: THREE2.RGBADepthPacking, map: this.tex, alphaTest: 0.5 });
    const blobW = H / 2 * 0.9;
    this.blob = new THREE2.Mesh(new THREE2.PlaneGeometry(blobW, H / 2 * 0.34), new THREE2.MeshBasicMaterial({ map: blobTex(), transparent: true, depthWrite: false, opacity: 0.55 }));
    this.blob.rotation.x = -Math.PI / 2;
    this.blob.position.y = 0.01;
    this.group = new THREE2.Group;
    this.pivot = new THREE2.Group;
    this.pivot.rotation.order = "YXZ";
    this.pivot.add(this.mesh);
    this.group.add(this.pivot, this.blob);
    this.group.userData.toon = this;
    this.key = "";
    this.H = H;
    this._facePos = new THREE2.Vector3;
    if (multiPlane) {
      const s = H / 1024;
      this.headCanvas = document.createElement("canvas");
      this.headCanvas.width = 1024 * res;
      this.headCanvas.height = 1024 * res;
      this.headG = this.headCanvas.getContext("2d");
      this.headTex = new THREE2.CanvasTexture(this.headCanvas);
      this.headTex.colorSpace = THREE2.SRGBColorSpace;
      this.headTex.anisotropy = 4;
      const headMat = new THREE2.MeshBasicMaterial({ map: this.headTex, transparent: true, alphaTest: 0.04, side: THREE2.DoubleSide, toneMapped: false });
      this.headMesh = new THREE2.Mesh(new THREE2.PlaneGeometry(1024 * s, 1024 * s), headMat);
      this.headMesh.castShadow = true;
      this.headMesh.customDepthMaterial = new THREE2.MeshDepthMaterial({ depthPacking: THREE2.RGBADepthPacking, map: this.headTex, alphaTest: 0.5 });
      this.headMesh.renderOrder = this.mesh.renderOrder + 1;
      this.headPivot = new THREE2.Group;
      this.headPivot.position.set(0, 648 * s, 0.055);
      this.headPivot.add(this.headMesh);
      this.pivot.add(this.headPivot);
      OVERFLOW_CANVASES.push({ canvas: this.headCanvas, res, label: this.name + ":head", pose: () => this.poseSummary() });
    }
    this.set({});
  }
  set(st = {}) {
    if (this.group.visible === false)
      return this;
    const key = JSON.stringify(st, (k, v) => typeof v === "function" ? v.name || String(v).length : typeof v === "number" ? Math.round(v * 100) / 100 : v);
    if (key === this.key) {
      this.syncBlob();
      return this;
    }
    this.key = key;
    this.st = st;
    const drawSt = st.sitOn != null && st.sitDrop == null ? { ...st, sitDrop: st.sitOn * 1024 / this.H } : st;
    const g = this.g;
    g.setTransform(1, 0, 0, 1, 0, 0);
    g.clearRect(0, 0, this.canvas.width, this.canvas.height);
    g.scale(this.res, this.res);
    g.translate(STAGE.OX, STAGE.OY);
    if (this.multiPlane && this.headCanvas && this.headG && this.headTex) {
      this.hands = drawToonLayer(g, this.look, drawSt, "nohead");
      const g2 = this.headG;
      g2.setTransform(1, 0, 0, 1, 0, 0);
      g2.clearRect(0, 0, this.headCanvas.width, this.headCanvas.height);
      g2.scale(this.res, this.res);
      g2.translate(256, 160);
      drawToonLayer(g2, this.look, drawSt, "headonly");
      this.headTex.needsUpdate = true;
    } else {
      this.hands = drawToon(g, this.look, drawSt);
    }
    this.tex.needsUpdate = true;
    this.syncBlob();
    return this;
  }
  overflow() {
    if (!this.key)
      return null;
    if (this._overflowKey === this.key && this._overflowCache)
      return this._overflowCache;
    const r = scanCanvasOverflow(this.canvas, this.res);
    this._overflowKey = this.key;
    this._overflowCache = r;
    return r;
  }
  describe() {
    const L = this.look || {};
    return `hat=${L.hat ?? "none"} body=${L.body ?? "#E8734A"} pattern=${L.pattern ?? "plain"}`;
  }
  poseSummary() {
    if (!this.st)
      return "never painted";
    const s = this.st, fmt = (a) => `[${(Array.isArray(a) ? a : [8, 6]).join(",")}]`;
    const base = `mood=${s.mood ?? "neutral"} armL=${fmt(s.armL)} armR=${fmt(s.armR)} legs=${s.legs ?? "stand"}`;
    const ht = s.headTilt || 0, hn = s.headTurn || 0;
    return ht || hn ? `${base} head=${ht}/${hn}` : base;
  }
  face(cam) {
    if (this.group.visible === false) {
      this.syncAttachments();
      return this;
    }
    const p = this._facePos;
    this.group.getWorldPosition(p);
    this.pivot.rotation.y = Math.atan2(cam.position.x - p.x, cam.position.z - p.z) - this.group.rotation.y;
    if (this.perspective) {
      const dy = cam.position.y - (p.y + this.H * 0.6), dh = Math.hypot(cam.position.x - p.x, cam.position.z - p.z);
      const pitchAng = dh < 0.0001 ? 0 : Math.atan2(dy, dh);
      this.pivot.rotation.x = clamp3(pitchAng * 0.45, -0.38, 0.38);
    }
    if (this.multiPlane && this.headPivot)
      this.headPivot.rotation.y = (this.st?.headTurn || 0) * -0.22;
    this.syncBlob();
    this.syncAttachments();
    return this;
  }
  syncBlob() {
    const air = clamp3(this.group.position.y - this.groundY, 0, 3);
    this.blob.position.y = 0.01 - air;
    const sc = 1 / (1 + air * 0.55);
    this.blob.scale.set(sc, sc, 1);
    this.blob.material.opacity = 0.55 * Math.exp(-air * 1.1);
  }
  matchEnv(t) {
    let r = 1, gg = 1, b = 1;
    r *= 1 + t.warm * 0.6;
    b *= 1 - t.warm * 0.6;
    if (t.sat < 1) {
      const k = (1 - t.sat) * 0.25;
      r += (0.92 - r) * k;
      gg += (0.92 - gg) * k;
      b += (0.92 - b) * k;
    }
    const e = clamp3(Math.sqrt(Math.max(t.exposure, 0)), 0, 1.15);
    r *= e;
    gg *= e;
    b *= e;
    const c = this._envC.setRGB(r, gg, b);
    if (t.fogMix > 0)
      c.lerp(this._envFog.set(t.fogColor), clamp3(t.fogMix, 0, 1));
    this.mat.color.copy(c);
    if (this.headMesh)
      this.headMesh.material.color.copy(c);
    return this;
  }
  attach(obj, hand, opts = {}) {
    this._attachments = this._attachments.filter((e) => e.obj !== obj);
    const [rx, ry, rz] = opts.rot ?? [0, 0, 0];
    this._attachments.push({ obj, hand, offset: new THREE2.Vector3(...opts.offset ?? [0, 0, 0]), rotQ: new THREE2.Quaternion().setFromEuler(new THREE2.Euler(rx, ry, rz)) });
    return this;
  }
  detach(obj) {
    this._attachments = obj ? this._attachments.filter((e) => e.obj !== obj) : [];
    return this;
  }
  syncAttachments() {
    if (!this._attachments.length)
      return;
    if (this.group.visible === false) {
      for (const e of this._attachments)
        e.obj.visible = false;
      return;
    }
    const s = this.H / 1024;
    this.mesh.updateWorldMatrix(true, false);
    this.mesh.getWorldQuaternion(this._attQ);
    for (const e of this._attachments) {
      const h = this.hands?.[e.hand];
      if (!h)
        continue;
      const w = this._attW.set((STAGE.OX + h[0] - STAGE.CW / 2) * s, (1256 - (STAGE.OY + h[1])) * s, 0);
      this.mesh.localToWorld(w);
      e.obj.position.copy(w).add(this._attOff.copy(e.offset).applyQuaternion(this._attQ));
      e.obj.quaternion.copy(this._attQ).multiply(e.rotQ);
      e.obj.visible = true;
    }
  }
  tint(col) {
    this.mat.color.set(col);
    return this;
  }
  restyle(patch) {
    Object.assign(this.look, patch);
    this.key = "";
    return this;
  }
  at(x, y, z) {
    this.group.position.set(x, y, z);
    this.syncBlob();
    return this;
  }
}
var _camPitchTmp = new THREE2.Vector3;
var moodMeta = (name, display, since) => ({
  id: `mood.${name}`,
  kind: "mood",
  name: display,
  snippet: `p.set({ mood: '${name}' })`,
  captionSafe: "face-only change; nothing enters the caption zone",
  preview: `library/previews/mood-${name}.jpg`,
  optIn: `mood: '${name}'`,
  since
});
var MOODS_LIBRARY = [
  moodMeta("neutral", "Neutral", "pre-5.0"),
  moodMeta("smug", "Smug", "pre-5.0"),
  moodMeta("happy", "Happy", "pre-5.0"),
  moodMeta("laugh", "Laugh", "pre-5.0"),
  moodMeta("shock", "Shock", "pre-5.0"),
  moodMeta("worried", "Worried", "pre-5.0"),
  moodMeta("angry", "Angry", "pre-5.0"),
  moodMeta("sad", "Sad", "pre-5.0"),
  moodMeta("sly", "Sly", "pre-5.0"),
  moodMeta("proud", "Proud", "pre-5.0"),
  moodMeta("tired", "Tired", "pre-5.0"),
  moodMeta("dizzy", "Dizzy", "pre-5.0"),
  moodMeta("determined", "Determined", "4.0"),
  moodMeta("embarrassed", "Embarrassed", "4.0"),
  moodMeta("awe", "Awe", "4.0"),
  moodMeta("suspicious", "Suspicious", "4.0"),
  moodMeta("relieved", "Relieved", "4.0"),
  moodMeta("puzzled", "Puzzled — one brow up, one down; the thinking-it-over face", "5.0"),
  moodMeta("deadpan", "Deadpan — heavy lids, flat mouth; the micro-beat after a reveal", "5.0")
];
var hatMeta = (name) => ({
  id: `hat.${name}`,
  kind: "hat",
  name,
  snippet: `registerCast('hero', { hat: '${name}' })`,
  captionSafe: "identity field (lookSignature) — choose at registerCast time, never swap mid-film",
  preview: `library/previews/hat-${name}.jpg`,
  optIn: `hat: '${name}'`,
  since: "pre-5.0"
});
var HATS_LIBRARY = HATS.map(hatMeta);
var neckMeta = (name) => ({
  id: `neck.${name}`,
  kind: "neck",
  name,
  snippet: `registerCast('hero', { neck: '${name}' })`,
  captionSafe: "identity field (lookSignature) — choose at registerCast time, never swap mid-film",
  preview: `library/previews/neck-${name}.jpg`,
  optIn: `neck: '${name}'`,
  since: "pre-5.0"
});
var NECKS_LIBRARY = NECKS.map(neckMeta);
var EXTRAS = ["mustache", "glasses", "freckles", "blush", "pencil", "monocle"];
var EXTRAS_LIBRARY = EXTRAS.map((name) => ({
  id: `extra.${name}`,
  kind: "extra",
  name,
  snippet: `registerCast('hero', { extra: ['${name}'] })`,
  captionSafe: "identity field (lookSignature) — choose at registerCast time, never swap mid-film",
  preview: `library/previews/extra-${name}.jpg`,
  optIn: `extra: ['${name}']`,
  since: "pre-5.0"
}));

// src/main.ts
import * as THREE4 from "three";

// src/shots.ts
import * as THREE3 from "three";
var FOV = 40;
var FOG_COLOR = 13225922;
var FOG_NEAR = 4.5;
var FOG_FAR = 20;
var ROWS = [
  [3, 0.62],
  [6, 0.31],
  [10, 0],
  [15, -0.31],
  [25, -0.62]
];
var BASIC_NDC_X = -0.45;
var STD_NDC_X = 0.45;
var PANEL_NDC_W = 0.62;
var PANEL_NDC_H = 0.3;
var BASIC_COL = {
  3: "#D94F30",
  6: "#3F6FD9",
  10: "#7A8B99",
  15: "#2FA89B",
  25: "#8C8C46"
};
var STD_COL = {
  3: "#30D94F",
  6: "#D9B83F",
  10: "#8E44AD",
  15: "#C94F7C",
  25: "#466B8C"
};
var GROUND_COL = "#2E4A34";
function flatTexture(hex) {
  const S = 64;
  const cv = document.createElement("canvas");
  cv.width = S;
  cv.height = S;
  const g = cv.getContext("2d");
  g.fillStyle = hex;
  g.fillRect(0, 0, S, S);
  const tex = new THREE3.CanvasTexture(cv);
  tex.colorSpace = THREE3.SRGBColorSpace;
  tex.minFilter = THREE3.LinearFilter;
  tex.magFilter = THREE3.LinearFilter;
  tex.generateMipmaps = false;
  return tex;
}
function buildShots(E) {
  const scene = new THREE3.Scene;
  scene.fog = new THREE3.Fog(FOG_COLOR, FOG_NEAR, FOG_FAR);
  scene.background = new THREE3.Color(FOG_COLOR);
  const cam = camera(FOV);
  cam.position.set(0, 0, 0);
  cam.lookAt(0, 0, -1);
  scene.add(new THREE3.AmbientLight(16777215, 0.35));
  const dir = new THREE3.DirectionalLight(16777215, 0.65);
  dir.position.set(10, 16, 12);
  scene.add(dir);
  const ground = new THREE3.Mesh(new THREE3.BoxGeometry(80, 0.1, 72), new THREE3.MeshStandardMaterial({ color: GROUND_COL, roughness: 1, metalness: 0 }));
  ground.position.set(0, -8.05, -29);
  scene.add(ground);
  const tanH = Math.tan(THREE3.MathUtils.degToRad(FOV / 2));
  for (const [d, ny] of ROWS) {
    const hw = d * tanH * (1080 / 1920), hh = d * tanH;
    const w = PANEL_NDC_W * hw, h = PANEL_NDC_H * hh, cy = ny * hh;
    const basic = new THREE3.Mesh(new THREE3.PlaneGeometry(w, h), new THREE3.MeshBasicMaterial({ map: flatTexture(BASIC_COL[d]) }));
    basic.position.set(BASIC_NDC_X * hw, cy, -d);
    scene.add(basic);
    const std = new THREE3.Mesh(new THREE3.BoxGeometry(w, h, 0.1), new THREE3.MeshStandardMaterial({ color: STD_COL[d], roughness: 1, metalness: 0 }));
    std.position.set(STD_NDC_X * hw, cy, -(d + 0.05));
    scene.add(std);
  }
  const draw = (_lt, _d, _t, blit) => {
    E.render(scene, cam, {
      focus: 10,
      aperture: 0,
      maxblur: 0.006,
      bloom: 0,
      exposure: 1,
      vignette: 0,
      grain: 0,
      warm: 0,
      sat: 1
    });
    blit();
  };
  return { shots: [{ id: "fog-chart", at: 0, note: "Fog chart: basic + standard panels at 3/6/10/15/25 m under S06 fog", draw }], cues: [] };
}

// src/core/cast.ts
var CAST = new Map;
function lookSignature(look) {
  return JSON.stringify({
    body: look.body ?? null,
    pattern: look.pattern ?? null,
    patCol: look.patCol ?? null,
    hat: look.hat ?? null,
    hatCol: look.hatCol ?? null,
    band: look.band ?? null,
    neck: look.neck ?? null,
    neckCol: look.neckCol ?? null,
    head: look.head ?? null,
    kid: Boolean(look.kid),
    extra: [...look.extra ?? []].sort()
  });
}

// src/main.ts
var cv = document.getElementById("cv");
var GL = document.getElementById("gl");
setCtx(cv.getContext("2d"));
var OP = window.OP = { ready: false };
function draw(t) {
  t = clamp(t, 0, T.dur - 0.0001);
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.globalAlpha = 1;
  ctx.letterSpacing = "0px";
  const S = OP._shots;
  let i = S.length - 1;
  while (i > 0 && S[i].at > t)
    i--;
  const sh = S[i], end = i < S.length - 1 ? S[i + 1].at : T.dur, d = end - sh.at, lt = t - sh.at;
  const blit = () => ctx.drawImage(GL, 0, 0, W, H);
  ctx.save();
  sh.draw(lt, d, t, blit);
  ctx.restore();
  ctx.globalAlpha = 1;
  ctx.letterSpacing = "0px";
  const fade = Math.min(clamp(t / 0.25), clamp((T.dur - t) / 0.4));
  if (fade < 1) {
    ctx.fillStyle = "#000";
    ctx.globalAlpha = 1 - fade;
    ctx.fillRect(0, 0, W, H);
    ctx.globalAlpha = 1;
  }
  drawCaption(t);
}
(async () => {
  try {
    await document.fonts.load("900 100px Inter");
    await document.fonts.load("800 100px Inter");
    const script = await (await fetch("script.json", { cache: "no-store" })).json();
    await loadTiming();
    buildCaptions(script);
    GL.width = Math.round(1080 * RES);
    GL.height = Math.round(1920 * RES);
    const E = makeEngine(GL, { w: GL.width, h: GL.height, samples: AA });
    const { shots, cues } = buildShots(E);
    const _render = E.render.bind(E);
    E.render = (scene, cam, look) => {
      OP._cam = cam;
      OP._scene = scene;
      return _render(scene, cam, look);
    };
    shots.sort((a, b) => a.at - b.at);
    OP._shots = shots;
    OP.script = script;
    OP.duration = () => T.dur;
    OP.draw = draw;
    OP.frame = (t, q = 0.92) => {
      draw(t);
      return cv.toDataURL("image/jpeg", q);
    };
    OP.shots = () => shots.map((s, i) => ({ id: s.id, at: +s.at.toFixed(3), end: +(i < shots.length - 1 ? shots[i + 1].at : T.dur).toFixed(3), note: s.note || "" }));
    OP.captions = () => captionList().map((c) => ({ s: +c.s.toFixed(3), e: +c.e.toFixed(3), text: c.words.map((w) => (w.hi ? "*" : "") + w.txt).join(" "), line: c.line, auto: c.auto, rawW: c.rawW, fit: c.fit, renderedW: c.renderedW }));
    OP.cues = () => cues.slice().sort((a, b) => a.t - b.t);
    OP.cutouts = () => {
      const items = TOONS.map((t) => {
        let box = null;
        let inScene = !OP._scene;
        if (!inScene) {
          let n = t.group;
          while (n) {
            if (n === OP._scene) {
              inScene = true;
              break;
            }
            n = n.parent;
          }
        }
        if (inScene && t.group.visible !== false && OP._cam) {
          const cam = OP._cam;
          cam.updateMatrixWorld();
          t.mesh.updateWorldMatrix(true, false);
          const geo = t.mesh.geometry;
          if (!geo.boundingBox)
            geo.computeBoundingBox();
          const bb = geo.boundingBox;
          const corners = [
            [bb.min.x, bb.min.y],
            [bb.min.x, bb.max.y],
            [bb.max.x, bb.min.y],
            [bb.max.x, bb.max.y]
          ];
          let behind = false;
          const xs = [], ys = [];
          for (const [cx, cy] of corners) {
            const v = new THREE4.Vector3(cx, cy, bb.min.z).applyMatrix4(t.mesh.matrixWorld);
            const viewZ = v.clone().applyMatrix4(cam.matrixWorldInverse).z;
            if (viewZ > 0)
              behind = true;
            v.project(cam);
            if (!Number.isFinite(v.x) || !Number.isFinite(v.y) || !Number.isFinite(v.z)) {
              behind = true;
              v.x = Number.isFinite(v.x) ? v.x : 0;
              v.y = Number.isFinite(v.y) ? v.y : 0;
              v.z = Number.isFinite(v.z) ? v.z : 2;
            }
            if (v.z > 1)
              behind = true;
            const clampPx = (n) => Math.max(-1e4, Math.min(1e4, n));
            xs.push(clampPx((v.x + 1) / 2 * 1080));
            ys.push(clampPx((1 - v.y) / 2 * 1920));
          }
          box = {
            x0: Math.round(Math.min(...xs)),
            y0: Math.round(Math.min(...ys)),
            x1: Math.round(Math.max(...xs)),
            y1: Math.round(Math.max(...ys))
          };
          if (behind)
            box.behind = true;
        }
        return { id: t.name, desc: t.describe(), pose: t.poseSummary(), overflow: t.overflow(), box, sig: lookSignature(t.look) };
      });
      const toonCanvases = new Set(TOONS.map((t) => t.canvas));
      for (const e of OVERFLOW_CANVASES) {
        if (toonCanvases.has(e.canvas))
          continue;
        items.push({ id: e.label, desc: "registered canvas", pose: e.pose ? e.pose() : "", overflow: scanCanvasOverflow(e.canvas, e.res), box: null, sig: "" });
      }
      return items;
    };
    OP.cast = () => [...CAST.keys()];
    window.__registerOverflowCanvas = registerOverflowCanvas;
    OP.ready = true;
    const q = new URLSearchParams(location.search);
    if (!q.has("capture")) {
      if (q.has("t"))
        draw(+q.get("t"));
      else {
        const audio = new Audio("data/mix.wav");
        let t0 = null;
        const btn = document.getElementById("play");
        btn.hidden = false;
        btn.onclick = () => {
          btn.hidden = true;
          t0 = performance.now();
          audio.currentTime = 0;
          audio.play().catch(() => {});
        };
        const loop = (now) => {
          const t = t0 == null ? 0 : (now - t0) / 1000;
          draw(t);
          if (t0 != null && t > T.dur) {
            t0 = null;
            btn.hidden = false;
          }
          requestAnimationFrame(loop);
        };
        requestAnimationFrame(loop);
      }
    }
  } catch (e) {
    OP.bootError = String(e instanceof Error ? e.stack : e);
    console.error(e);
  }
})();
