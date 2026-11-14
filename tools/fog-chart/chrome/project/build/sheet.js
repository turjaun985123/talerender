// src/sheet.ts
import * as THREE5 from "three";

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

// src/core/board.ts
var BW = 1920;
var BH = 1080;
var B;
var off = document.createElement("canvas");
off.width = 1080;
off.height = 1920;
var offCtx = off.getContext("2d");
function startBoard(canvas, { title, author = "MADE BY YOUR NAME · 2026", bg = "#17181C", line = "#4A4D55", head = "#F2F2F2", accent = "#FFD43B", font = "Inter" }) {
  B = canvas.getContext("2d");
  setCtx(B);
  B.fillStyle = bg;
  B.fillRect(0, 0, BW, BH);
  B.font = `900 44px ${font}`;
  B.fillStyle = head;
  B.textBaseline = "alphabetic";
  B.textAlign = "left";
  B.letterSpacing = "3px";
  B.fillText(title, 24, 52);
  B.font = `800 22px Inter`;
  B.textAlign = "right";
  B.fillStyle = "#B9BCC4";
  B.letterSpacing = "2px";
  B.fillText(author, BW - 24, 48);
  B.letterSpacing = "0px";
  board.line = line;
  board.head = head;
  board.accent = accent;
  board.font = font;
  return B;
}
var board = { line: "#444", head: "#fff", accent: "#ff0", font: "Inter" };
function panel(x, y, w, h, n, label) {
  B.strokeStyle = board.line;
  B.lineWidth = 2;
  B.strokeRect(x + 0.5, y + 0.5, w, h);
  B.font = `900 30px ${board.font}`;
  B.fillStyle = board.head;
  B.textAlign = "left";
  B.letterSpacing = "6px";
  B.fillText(`${n}. ${label}`, x + 18, y + 42);
  B.letterSpacing = "0px";
  B.strokeStyle = board.line;
  B.beginPath();
  B.moveTo(x + 18, y + 58);
  B.lineTo(x + w - 18, y + 58);
  B.stroke();
  return { x: x + 18, y: y + 72, w: w - 36, h: h - 90 };
}
function frame(x, y, w, drawFn) {
  offCtx.setTransform(1, 0, 0, 1, 0, 0);
  offCtx.globalAlpha = 1;
  offCtx.clearRect(0, 0, 1080, 1920);
  setCtx(offCtx);
  offCtx.save();
  drawFn();
  offCtx.restore();
  setCtx(B);
  const h = w * 16 / 9;
  B.drawImage(off, x, y, w, h);
  B.strokeStyle = board.line;
  B.lineWidth = 2;
  B.strokeRect(x, y, w, h);
  return h;
}
function small(text, x, y, align = "left", size = 17, color = "#D7D9DE") {
  B.font = `800 ${size}px Inter`;
  B.fillStyle = color;
  B.textAlign = align;
  B.letterSpacing = "2px";
  text.split(`
`).forEach((l, i) => B.fillText(l, x, y + i * (size + 5)));
  B.letterSpacing = "0px";
}
function rules(box, list, y0 = box.y + 20, size = 19, step = 52) {
  list.forEach((r, i) => {
    B.beginPath();
    B.arc(box.x + 18, y0 + i * step - 6, 17, 0, 7);
    B.strokeStyle = board.head;
    B.lineWidth = 2;
    B.stroke();
    small(String(i + 1), box.x + 18, y0 + i * step + 1, "center", 17, board.head);
    small(r, box.x + 50, y0 + i * step, "left", size, "#E6E7EA");
  });
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

// src/core/world.ts
import * as THREE4 from "three";

// src/core/toon.ts
import * as THREE2 from "three";
var INK = "#1F1C33";
var LW = 9;
var R2D = Math.PI / 180;
var MOODS = ["neutral", "smug", "happy", "laugh", "shock", "worried", "angry", "sad", "sly", "proud", "tired", "dizzy", "determined", "embarrassed", "awe", "suspicious", "relieved", "puzzled", "deadpan"];
var HATS = ["none", "cap", "bowler", "boater", "beanie", "visor", "tophat", "bun", "flower", "bow", "mailcap"];
var NECKS = ["none", "bowtie", "tie", "scarf", "collar"];
var clamp2 = (v, a, b) => Math.max(a, Math.min(b, v));
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
      this.pivot.rotation.x = clamp2(pitchAng * 0.45, -0.38, 0.38);
    }
    if (this.multiPlane && this.headPivot)
      this.headPivot.rotation.y = (this.st?.headTurn || 0) * -0.22;
    this.syncBlob();
    this.syncAttachments();
    return this;
  }
  syncBlob() {
    const air = clamp2(this.group.position.y - this.groundY, 0, 3);
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
    const e = clamp2(Math.sqrt(Math.max(t.exposure, 0)), 0, 1.15);
    r *= e;
    gg *= e;
    b *= e;
    const c = this._envC.setRGB(r, gg, b);
    if (t.fogMix > 0)
      c.lerp(this._envFog.set(t.fogColor), clamp2(t.fogMix, 0, 1));
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

// src/core/props.ts
import * as THREE3 from "three";

// src/core/world.ts
var rnd = (i) => {
  const x = Math.sin(i * 127.1 + 311.7) * 43758.5453;
  return x - Math.floor(x);
};
var TEX = {};
function canvasTex(key, w, h, draw, rep = [1, 1]) {
  let t = TEX[key];
  if (!t) {
    const c = document.createElement("canvas");
    c.width = w;
    c.height = h;
    draw(c.getContext("2d"), w, h);
    t = new THREE4.CanvasTexture(c);
    t.colorSpace = THREE4.SRGBColorSpace;
    t.wrapS = t.wrapT = THREE4.RepeatWrapping;
    t.anisotropy = 8;
    TEX[key] = t;
  }
  const u = t.clone();
  u.repeat.set(rep[0], rep[1]);
  u.needsUpdate = true;
  return u;
}
var shade = (hex, k) => {
  const c = new THREE4.Color(hex);
  c.multiplyScalar(k);
  return "#" + c.getHexString();
};
var MATCACHE = {};
var mat = (o = {}) => {
  if (o.map || Object.values(o).some((v) => v !== null && typeof v === "object"))
    return new THREE4.MeshStandardMaterial({ roughness: 0.85, metalness: 0, ...o });
  const eff = { roughness: 0.85, metalness: 0, ...o };
  const key = Object.keys(eff).sort().map((k) => k + "=" + String(eff[k])).join("|");
  return MATCACHE[key] || (MATCACHE[key] = new THREE4.MeshStandardMaterial(eff));
};
var GEOCACHE = {};
var boxGeo = (w, h, d) => {
  const k = "b|" + w + "|" + h + "|" + d;
  return GEOCACHE[k] || (GEOCACHE[k] = new THREE4.BoxGeometry(w, h, d));
};
var cylGeo = (r1, r2, h, seg) => {
  const k = "c|" + r1 + "|" + r2 + "|" + h + "|" + seg;
  return GEOCACHE[k] || (GEOCACHE[k] = new THREE4.CylinderGeometry(r1, r2, h, seg));
};
var box = (w, h, d, m, x = 0, y = 0, z = 0, parent) => {
  const b = new THREE4.Mesh(boxGeo(w, h, d), m);
  b.position.set(x, y, z);
  b.castShadow = b.receiveShadow = true;
  if (parent)
    parent.add(b);
  return b;
};
var cyl = (r1, r2, h, m, x, y, z, parent, seg = 12) => {
  const c = new THREE4.Mesh(cylGeo(r1, r2, h, seg), m);
  c.position.set(x, y, z);
  c.castShadow = c.receiveShadow = true;
  if (parent)
    parent.add(c);
  return c;
};
var T = {
  planks: (col = "#A8693E", rep = [1, 1]) => canvasTex("planks" + col, 512, 512, (g, w, h) => {
    const n = 8, ph = h / n;
    for (let i = 0;i < n; i++) {
      g.fillStyle = shade(col, 0.86 + rnd(i * 3) * 0.24);
      g.fillRect(0, i * ph, w, ph);
      g.strokeStyle = shade(col, 0.7);
      g.globalAlpha = 0.35;
      g.lineWidth = 2;
      for (let k = 0;k < 4; k++) {
        g.beginPath();
        const y = i * ph + 8 + rnd(i * 7 + k) * (ph - 16);
        g.moveTo(0, y);
        g.bezierCurveTo(w * 0.3, y + 6, w * 0.6, y - 6, w, y + 2);
        g.stroke();
      }
      g.globalAlpha = 1;
      g.fillStyle = shade(col, 0.45);
      g.fillRect(0, i * ph, w, 4);
      const jx = rnd(i * 11) * w;
      g.fillRect(jx, i * ph, 4, ph);
      g.fillStyle = shade(col, 0.5);
      [jx - 14, jx + 18].forEach((x) => {
        g.beginPath();
        g.arc(x, i * ph + ph / 2, 3, 0, 7);
        g.fill();
      });
    }
  }, rep),
  wallpaper: (col = "#9E2E3A", motif = "#B8414C", rep = [1, 1]) => canvasTex("wp" + col + motif, 256, 256, (g, w, h) => {
    g.fillStyle = col;
    g.fillRect(0, 0, w, h);
    g.fillStyle = motif;
    g.strokeStyle = motif;
    g.lineWidth = 5;
    const fleur = (x, y, s) => {
      g.save();
      g.translate(x, y);
      g.scale(s, s);
      g.beginPath();
      g.moveTo(0, -40);
      g.bezierCurveTo(18, -20, 18, 0, 0, 14);
      g.bezierCurveTo(-18, 0, -18, -20, 0, -40);
      g.fill();
      g.beginPath();
      g.arc(-22, 4, 12, 0, 7);
      g.arc(22, 4, 12, 0, 7);
      g.fill();
      g.fillRect(-3, 10, 6, 26);
      g.restore();
    };
    [[64, 64], [192, 192], [192, 64 - 256], [64, 192]].forEach(([x, y], i) => i < 2 && fleur(x, y, 0.9));
    g.globalAlpha = 0.5;
    for (let i = 0;i < 4; i++) {
      g.beginPath();
      g.arc(i % 2 ? 192 : 64, i < 2 ? 192 : 64, 6, 0, 7);
      g.fill();
    }
    g.globalAlpha = 1;
  }, rep),
  brick: (col = "#B5553C", rep = [1, 1]) => canvasTex("brick" + col, 512, 512, (g, w, h) => {
    g.fillStyle = shade(col, 0.6);
    g.fillRect(0, 0, w, h);
    const bh = 32, bw = 85;
    for (let r = 0;r < h / bh; r++)
      for (let c = -1;c < w / bw + 1; c++) {
        const x = c * bw + r % 2 * bw / 2;
        g.fillStyle = shade(col, 0.85 + rnd(r * 31 + c) * 0.3);
        g.fillRect(x + 3, r * bh + 3, bw - 6, bh - 6);
      }
  }, rep),
  siding: (col = "#6FA3B8", rep = [1, 1]) => canvasTex("siding" + col, 256, 256, (g, w, h) => {
    for (let i = 0;i < 8; i++) {
      const y = i * 32;
      const gr = g.createLinearGradient(0, y, 0, y + 32);
      gr.addColorStop(0, shade(col, 1.06));
      gr.addColorStop(1, shade(col, 0.86));
      g.fillStyle = gr;
      g.fillRect(0, y, w, 32);
      g.fillStyle = shade(col, 0.6);
      g.fillRect(0, y + 30, w, 2);
    }
  }, rep),
  dirt: (col = "#C8763F", rep = [1, 1], tracks = true) => canvasTex("dirt" + col + tracks, 512, 512, (g, w, h) => {
    g.fillStyle = col;
    g.fillRect(0, 0, w, h);
    for (let i = 0;i < 2600; i++) {
      g.fillStyle = shade(col, 0.75 + rnd(i) * 0.5);
      g.globalAlpha = 0.5;
      g.beginPath();
      g.arc(rnd(i * 2.1) * w, rnd(i * 3.7) * h, 1 + rnd(i * 5.3) * 2.5, 0, 7);
      g.fill();
    }
    g.globalAlpha = 0.18;
    g.fillStyle = shade(col, 0.6);
    if (tracks)
      [0.3, 0.38, 0.62, 0.7].forEach((x) => g.fillRect(x * w, 0, 14, h));
    g.globalAlpha = 1;
  }, rep),
  grass: (col = "#6DBB4A", rep = [1, 1]) => canvasTex("grass" + col, 512, 512, (g, w, h) => {
    g.fillStyle = col;
    g.fillRect(0, 0, w, h);
    for (let i = 0;i < 5000; i++) {
      const x = rnd(i * 1.3) * w, y = rnd(i * 2.9) * h;
      g.strokeStyle = shade(col, 0.7 + rnd(i * 7.7) * 0.6);
      g.lineWidth = 2;
      g.beginPath();
      g.moveTo(x, y);
      g.lineTo(x + (rnd(i) - 0.5) * 6, y - 6 - rnd(i * 9) * 8);
      g.stroke();
    }
  }, rep),
  sand: (col = "#E3C078", rep = [1, 1]) => canvasTex("sand" + col, 512, 512, (g, w, h) => {
    g.fillStyle = col;
    g.fillRect(0, 0, w, h);
    for (let i = 0;i < 3200; i++) {
      g.fillStyle = shade(col, 0.78 + rnd(i) * 0.45);
      g.globalAlpha = 0.5;
      g.beginPath();
      g.arc(rnd(i * 2.3) * w, rnd(i * 3.1) * h, 1 + rnd(i * 5.7) * 2, 0, 7);
      g.fill();
    }
    g.globalAlpha = 0.16;
    g.strokeStyle = shade(col, 0.68);
    g.lineWidth = 3;
    for (let r = 0;r < 7; r++) {
      g.beginPath();
      for (let x = 0;x <= w; x += 16) {
        const y = 30 + r * 70 + Math.sin(x * 0.05 + r * 2.2) * 7;
        x === 0 ? g.moveTo(x, y) : g.lineTo(x, y);
      }
      g.stroke();
    }
    g.globalAlpha = 1;
  }, rep),
  snow: (col = "#F4F8FC", rep = [1, 1]) => canvasTex("snow" + col, 512, 512, (g, w, h) => {
    g.fillStyle = col;
    g.fillRect(0, 0, w, h);
    for (let i = 0;i < 2400; i++) {
      g.fillStyle = shade(col, 0.9 + rnd(i) * 0.09);
      g.globalAlpha = 0.55;
      g.beginPath();
      g.arc(rnd(i * 1.9) * w, rnd(i * 2.7) * h, 1 + rnd(i * 4.3) * 3, 0, 7);
      g.fill();
    }
    g.globalAlpha = 0.1;
    g.fillStyle = "#9FB8D8";
    for (let i = 0;i < 260; i++) {
      g.beginPath();
      g.arc(rnd(i * 3.3) * w, rnd(i * 6.1) * h, 1.5 + rnd(i * 2.2) * 2.5, 0, 7);
      g.fill();
    }
    g.globalAlpha = 1;
    g.fillStyle = "#FFFFFF";
    for (let i = 0;i < 130; i++)
      g.fillRect(rnd(i * 7.7) * w, rnd(i * 8.3) * h, 2.5, 2.5);
  }, rep),
  felt: (col = "#1F5A45") => canvasTex("felt" + col, 256, 256, (g, w, h) => {
    g.fillStyle = col;
    g.fillRect(0, 0, w, h);
    for (let i = 0;i < 3000; i++) {
      g.fillStyle = shade(col, 0.85 + rnd(i) * 0.3);
      g.fillRect(rnd(i * 3) * w, rnd(i * 7) * h, 2, 2);
    }
  })
};
function facade(name, { wall = "#C9603F", kind = "brick", trim = "#F2E6CF", sign = "#2E4E3F", signText = "#F2D37A", awning = null, w = 5, h = 7, seed = 1 } = {}) {
  const px = 110, cw = Math.round(w * px), ch = Math.round(h * px);
  return canvasTex("fac" + name + wall + w + h + seed, cw, ch, (g) => {
    if (kind === "brick") {
      const b = T.brick(wall).image;
      for (let y = 0;y < ch; y += 512)
        for (let x = 0;x < cw; x += 512)
          g.drawImage(b, x, y);
    } else {
      const b = T.siding(wall).image;
      for (let y = 0;y < ch; y += 256)
        for (let x = 0;x < cw; x += 256)
          g.drawImage(b, x, y);
    }
    g.fillStyle = trim;
    g.fillRect(0, 0, cw, 40);
    g.fillStyle = shade(trim, 0.7);
    g.fillRect(0, 40, cw, 10);
    for (let x = 10;x < cw; x += 34) {
      g.fillStyle = trim;
      g.fillRect(x, 50, 16, 16);
    }
    const nw = Math.max(2, Math.round(w / 1.6)), ww = cw / nw;
    for (let i = 0;i < nw; i++) {
      const x = i * ww + ww * 0.22, y = ch * 0.16, wd = ww * 0.56, ht = ch * 0.22;
      g.fillStyle = trim;
      g.fillRect(x - 10, y - 14, wd + 20, ht + 28);
      const gr = g.createLinearGradient(x, y, x + wd, y + ht);
      gr.addColorStop(0, "#7FB3D8");
      gr.addColorStop(0.5, "#3F6E9A");
      gr.addColorStop(1, "#2B4A70");
      g.fillStyle = gr;
      g.fillRect(x, y, wd, ht);
      g.fillStyle = "rgba(255,255,255,0.35)";
      g.beginPath();
      g.moveTo(x + wd * 0.1, y + ht);
      g.lineTo(x + wd * 0.45, y);
      g.lineTo(x + wd * 0.6, y);
      g.lineTo(x + wd * 0.25, y + ht);
      g.fill();
      g.fillStyle = trim;
      g.fillRect(x + wd / 2 - 4, y, 8, ht);
      g.fillRect(x, y + ht / 2 - 4, wd, 8);
      if (rnd(seed * 13 + i) > 0.5) {
        g.fillStyle = shade(wall, 0.8);
        g.fillRect(x, y, wd, ht * 0.35);
      }
    }
    const sy = ch * 0.47, sh = ch * 0.1;
    g.fillStyle = sign;
    g.fillRect(cw * 0.06, sy, cw * 0.88, sh);
    g.strokeStyle = signText;
    g.lineWidth = 5;
    g.strokeRect(cw * 0.06 + 8, sy + 8, cw * 0.88 - 16, sh - 16);
    g.fillStyle = signText;
    g.textAlign = "center";
    g.textBaseline = "middle";
    let fs = sh * 0.56;
    g.font = `700 ${fs}px Caladea, Georgia, serif`;
    while (g.measureText(name).width > cw * 0.78 && fs > 10) {
      fs -= 2;
      g.font = `700 ${fs}px Caladea, Georgia, serif`;
    }
    g.fillText(name, cw / 2, sy + sh / 2 + 2);
    const by = sy + sh + 16, bh = ch - by;
    g.fillStyle = shade(trim, 0.9);
    g.fillRect(0, by, cw, bh);
    const dw = cw * 0.2, dx = rnd(seed) > 0.5 ? cw * 0.08 : cw * 0.72;
    g.fillStyle = "#5A3A26";
    g.fillRect(dx, by + 14, dw, bh - 14);
    g.fillStyle = "#3F6E9A";
    g.fillRect(dx + 12, by + 30, dw - 24, bh * 0.4);
    g.fillStyle = "#E8C060";
    g.beginPath();
    g.arc(dx + dw - 18, by + bh * 0.62, 6, 0, 7);
    g.fill();
    const wx = dx < cw / 2 ? dx + dw + 20 : cw * 0.06, wwid = cw * 0.66 - 20;
    const gr = g.createLinearGradient(wx, by, wx + wwid, by + bh);
    gr.addColorStop(0, "#9CC7E4");
    gr.addColorStop(0.45, "#4C7FA8");
    gr.addColorStop(1, "#2A4565");
    g.fillStyle = "#4A2E1E";
    g.fillRect(wx - 8, by + 14, wwid + 16, bh * 0.7 + 16);
    g.fillStyle = gr;
    g.fillRect(wx, by + 22, wwid, bh * 0.7);
    for (let i = 0;i < 5; i++) {
      g.fillStyle = ["#E7B04A", "#D8584A", "#7DB36A", "#F2E6CF", "#8A6FC0"][(i + seed) % 5];
      g.fillRect(wx + 16 + i * (wwid - 32) / 5, by + bh * 0.55, (wwid - 32) / 5 - 10, bh * 0.2);
    }
    g.fillStyle = "rgba(255,255,255,0.3)";
    g.beginPath();
    g.moveTo(wx + wwid * 0.15, by + 22 + bh * 0.7);
    g.lineTo(wx + wwid * 0.4, by + 22);
    g.lineTo(wx + wwid * 0.52, by + 22);
    g.lineTo(wx + wwid * 0.27, by + 22 + bh * 0.7);
    g.fill();
    if (awning) {
      const ay = by - 4;
      for (let x = 0, i = 0;x < cw; x += 40, i++) {
        g.fillStyle = i % 2 ? "#F4EFE4" : awning;
        g.fillRect(x, ay, 40, 46);
        g.beginPath();
        g.arc(x + 20, ay + 46, 20, 0, Math.PI);
        g.fill();
      }
    }
  });
}
function signBoard(text, { w = 2, h = 0.5, bg = "#2E4E3F", fg = "#F2D37A", font = "Caladea" } = {}) {
  const t = canvasTex("sign" + text + bg + fg + w + h, Math.round(w * 200), Math.round(h * 200), (g, cw, ch) => {
    g.fillStyle = bg;
    g.fillRect(0, 0, cw, ch);
    g.strokeStyle = fg;
    g.lineWidth = 6;
    g.strokeRect(10, 10, cw - 20, ch - 20);
    g.fillStyle = fg;
    g.textAlign = "center";
    g.textBaseline = "middle";
    let fs = ch * 0.5;
    g.font = `700 ${fs}px ${font}, Georgia, serif`;
    while (g.measureText(text).width > cw * 0.86) {
      fs -= 2;
      g.font = `700 ${fs}px ${font}, Georgia, serif`;
    }
    g.fillText(text, cw / 2, ch / 2 + 3);
  });
  const m = new THREE4.Mesh(new THREE4.BoxGeometry(w, h, 0.05), [mat({ color: shade(bg, 0.7) }), mat({ color: shade(bg, 0.7) }), mat({ color: shade(bg, 0.7) }), mat({ color: shade(bg, 0.7) }), mat({ map: t }), mat({ color: shade(bg, 0.7) })]);
  m.castShadow = true;
  return m;
}
function sky(scene, { top = "#3D8EE0", horizon = "#BFE4F7", ground = "#BFE4F7", r = 380 } = {}) {
  const m = new THREE4.ShaderMaterial({
    side: THREE4.BackSide,
    depthWrite: false,
    fog: false,
    toneMapped: false,
    uniforms: { top: { value: new THREE4.Color(top) }, hor: { value: new THREE4.Color(horizon) }, gnd: { value: new THREE4.Color(ground) } },
    vertexShader: "varying vec3 vP; void main(){ vP = normalize(position); gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }",
    fragmentShader: "uniform vec3 top, hor, gnd; varying vec3 vP; void main(){ float y = vP.y; vec3 c = y > 0.0 ? mix(hor, top, pow(clamp(y * 1.6, 0.0, 1.0), 0.7)) : gnd; gl_FragColor = vec4(c, 1.0); }"
  });
  const s = new THREE4.Mesh(new THREE4.SphereGeometry(r, 32, 16), m);
  s.renderOrder = -10;
  scene.add(s);
  return s;
}
var cloudT = null;
function cloudTex() {
  if (cloudT)
    return cloudT;
  const c = document.createElement("canvas");
  c.width = 512;
  c.height = 256;
  const g = c.getContext("2d");
  const puffs = [[110, 175, 58], [185, 140, 80], [285, 118, 96], [380, 150, 70], [250, 178, 70], [440, 180, 44], [70, 190, 36]];
  g.filter = "blur(3px)";
  g.fillStyle = "#B9CCE6";
  puffs.forEach(([x, y, r]) => {
    g.beginPath();
    g.arc(x + 4, y + 14, r, 0, 7);
    g.fill();
  });
  g.fillStyle = "#FFFFFF";
  puffs.forEach(([x, y, r]) => {
    g.beginPath();
    g.arc(x - 2, y - 2, r * 0.92, 0, 7);
    g.fill();
  });
  g.globalCompositeOperation = "destination-out";
  g.filter = "blur(10px)";
  g.fillRect(0, 214, 512, 60);
  cloudT = new THREE4.CanvasTexture(c);
  cloudT.colorSpace = THREE4.SRGBColorSpace;
  return cloudT;
}
function clouds(scene, list, tint = "#FFFFFF") {
  const out = [];
  list.forEach(([x, y, z, s]) => {
    const sp = new THREE4.Sprite(new THREE4.SpriteMaterial({ map: cloudTex(), color: tint, fog: false, toneMapped: false, depthWrite: false }));
    sp.position.set(x, y, z);
    sp.scale.set(s * 2, s, 1);
    scene.add(sp);
    out.push(sp);
  });
  return out;
}
function sunlight(scene, E, { col = "#FFF0D6", i = 2.6, pos = [6, 10, 6], target = [0, 0, 0], box = 12, sky = "#CFE6FF", ground = "#C48A5A", hemi = 1.25, env = 0.25 } = {}) {
  scene.environment = E.envMap;
  scene.environmentIntensity = env;
  const r = lightRig(scene, { key: col, keyI: i, keyPos: pos, target, shadowBox: box, rimCol: "#FFFFFF", rimI: 0.4, sky, ground, hemiI: hemi });
  r.key.shadow.camera.far = 80;
  r.key.shadow.bias = -0.0006;
  r.key.shadow.camera.updateProjectionMatrix();
  return r;
}
function hills(scene, { col = "#79C25A", z = -90, list } = {}) {
  const g = new THREE4.Group;
  scene.add(g);
  (list || [[-40, z, 34, 12], [10, z - 10, 46, 16], [60, z + 4, 36, 11], [-90, z + 10, 40, 10]]).forEach(([x, zz, r, h], i) => {
    const m = new THREE4.Mesh(new THREE4.SphereGeometry(1, 32, 16), mat({ color: shade(col, 0.92 + rnd(i) * 0.16), map: T.grass(col, [6, 3]) }));
    m.scale.set(r, h, r * 0.6);
    m.position.set(x, -h * 0.25, zz);
    g.add(m);
    for (let k = 0;k < 6; k++) {
      const a = rnd(i * 9 + k) * 2 - 1;
      const t = tree(0.5 + rnd(k) * 0.4);
      t.traverse((o) => {
        if (o.isMesh)
          o.castShadow = false;
      });
      t.position.set(x + a * r * 0.6, h * (0.75 - Math.abs(a) * 0.45) - 1, zz + r * 0.25);
      t.scale.multiplyScalar(2.2);
      g.add(t);
    }
  });
  return g;
}
function tree(s = 1, col = "#4E9A3C", kind = "round") {
  const g = new THREE4.Group;
  cyl(0.12 * s, 0.18 * s, 1.6 * s, mat({ color: "#7A5234" }), 0, 0.8 * s, 0, g);
  const lm = mat({ color: col });
  if (kind === "pine") {
    [[1.15, 1.9], [0.85, 2.75], [0.55, 3.55]].forEach(([r, y], i) => {
      const c = new THREE4.Mesh(new THREE4.ConeGeometry(r * s, 1.7 * s, 10), mat({ color: shade(col, 0.88 + i * 0.09) }));
      c.position.y = y * s;
      c.castShadow = true;
      g.add(c);
    });
    return g;
  }
  if (kind === "cypress") {
    const c = new THREE4.Mesh(new THREE4.SphereGeometry(1, 16, 12), lm);
    c.scale.set(0.7 * s, 2.6 * s, 0.7 * s);
    c.position.y = 3.2 * s;
    c.castShadow = true;
    g.add(c);
  } else
    [[0, 2.3, 0, 1], [0.6, 2, 0.2, 0.75], [-0.6, 2, -0.1, 0.8], [0.1, 2.9, -0.2, 0.75], [0.2, 2.2, 0.6, 0.7]].forEach(([x, y, z, r], i) => {
      const m = new THREE4.Mesh(new THREE4.IcosahedronGeometry(r * s, 2), mat({ color: shade(col, 0.9 + rnd(i) * 0.2) }));
      m.position.set(x * s, y * s, z * s);
      m.castShadow = true;
      g.add(m);
    });
  return g;
}
function pole(h = 7) {
  const g = new THREE4.Group, w = mat({ color: "#6E4E36" });
  cyl(0.09, 0.12, h, w, 0, h / 2, 0, g, 8);
  box(1.8, 0.1, 0.1, w, 0, h - 0.6, 0, g);
  box(1.2, 0.08, 0.08, w, 0, h - 1.1, 0, g);
  [-0.8, -0.3, 0.3, 0.8].forEach((x) => {
    const ins = cyl(0.03, 0.03, 0.12, mat({ color: "#CFE3E8" }), x, h - 0.5, 0, g, 6);
    ins.castShadow = false;
  });
  return g;
}
function wire(a, b, sag = 0.6, col = "#2A2A33") {
  const pts = [];
  for (let i = 0;i <= 20; i++) {
    const t = i / 20;
    const p = new THREE4.Vector3().lerpVectors(a, b, t);
    p.y -= Math.sin(t * Math.PI) * sag;
    pts.push(p);
  }
  const m = new THREE4.Mesh(new THREE4.TubeGeometry(new THREE4.CatmullRomCurve3(pts), 20, 0.015, 4), new THREE4.MeshBasicMaterial({ color: col }));
  return m;
}
function lamp() {
  const g = new THREE4.Group, m = mat({ color: "#2E3A35", metalness: 0.3, roughness: 0.5 });
  cyl(0.05, 0.07, 3.2, m, 0, 1.6, 0, g, 8);
  cyl(0.18, 0.1, 0.3, m, 0, 3.3, 0, g, 8);
  const b = new THREE4.Mesh(new THREE4.SphereGeometry(0.14, 12, 8), new THREE4.MeshStandardMaterial({ color: "#FFF3C8", emissive: "#FFE7A0", emissiveIntensity: 0.6 }));
  b.position.y = 3.15;
  g.add(b);
  return g;
}
function barrel() {
  const g = new THREE4.Group;
  cyl(0.33, 0.33, 0.8, mat({ color: "#9A6337", map: T.planks("#9A6337") }), 0, 0.4, 0, g, 16);
  [0.12, 0.68].forEach((y) => cyl(0.345, 0.345, 0.05, mat({ color: "#3A3A3F", metalness: 0.4 }), 0, y, 0, g, 16));
  return g;
}
function crate(s = 0.6) {
  const g = new THREE4.Group;
  box(s, s, s, mat({ map: T.planks("#B07A48", [0.5, 0.5]) }), 0, s / 2, 0, g);
  return g;
}
function car(col = "#2C3A4A") {
  const g = new THREE4.Group, m = mat({ color: col, roughness: 0.4, metalness: 0.2 }), dark = mat({ color: "#1E1E24", roughness: 0.6 });
  box(1.5, 0.5, 3.2, m, 0, 0.75, 0, g);
  box(1.3, 0.8, 1.4, m, 0, 1.4, -0.3, g);
  box(1.2, 0.5, 0.05, mat({ color: "#9CC7E4", roughness: 0.1 }), 0, 1.5, 0.41, g);
  [[-0.78, 1.1], [0.78, 1.1], [-0.78, -1.1], [0.78, -1.1]].forEach(([x, z]) => {
    const w = cyl(0.36, 0.36, 0.22, dark, x, 0.36, z, g, 16);
    w.rotation.z = Math.PI / 2;
    const hub = cyl(0.12, 0.12, 0.24, mat({ color: "#D9D2C2" }), x, 0.36, z, g, 10);
    hub.rotation.z = Math.PI / 2;
  });
  [-0.5, 0.5].forEach((x) => {
    const l = new THREE4.Mesh(new THREE4.SphereGeometry(0.12, 10, 8), mat({ color: "#FFF3C8", emissive: "#FFE7A0", emissiveIntensity: 0.3 }));
    l.position.set(x, 0.95, 1.62);
    g.add(l);
  });
  return g;
}
function fence(len = 10, col = "#F4EFE4") {
  const g = new THREE4.Group, m = mat({ color: col });
  for (let x = 0;x <= len; x += 1.5)
    box(0.1, 1, 0.1, m, x, 0.5, 0, g);
  [0.45, 0.85].forEach((y) => box(len, 0.08, 0.06, m, len / 2, y, 0, g));
  return g;
}
function bunting(a, b, cols = ["#D8343F", "#F4EFE4", "#3C6FB6", "#F2C230"]) {
  const g = new THREE4.Group;
  g.add(wire(a, b, 0.5, "#3A2A2A"));
  const n = Math.round(a.distanceTo(b) / 0.5);
  for (let i = 1;i < n; i++) {
    const t = i / n, p = new THREE4.Vector3().lerpVectors(a, b, t);
    p.y -= Math.sin(t * Math.PI) * 0.5;
    const tri = new THREE4.Mesh(new THREE4.BufferGeometry().setFromPoints([new THREE4.Vector3(-0.16, 0, 0), new THREE4.Vector3(0.16, 0, 0), new THREE4.Vector3(0, -0.32, 0)]), new THREE4.MeshBasicMaterial({ color: cols[i % cols.length], side: THREE4.DoubleSide }));
    tri.position.copy(p);
    tri.lookAt(p.x + (b.z - a.z), p.y, p.z - (b.x - a.x));
    g.add(tri);
  }
  return g;
}
var LOOKS = [
  { body: "#5C8FD6", hat: "bowler", hatCol: "#3B3346" },
  { body: "#E8734A", hat: "cap", hatCol: "#6E8A4F" },
  { body: "#F2B8C6", hat: "flower", pattern: "apron" },
  { body: "#9BC46A", hat: "boater", hatCol: "#E8C77A" },
  { body: "#8C6FC9", hat: "tophat", hatCol: "#2A2638" },
  { body: "#F2C230", hat: "beanie", hatCol: "#D8343F", pattern: "overalls" },
  { body: "#4FB3A9", hat: "none", neck: "tie" },
  { body: "#D8584A", hat: "cap", hatCol: "#3B3346", pattern: "stripes" }
];
function extras(scene, spots, moodFn) {
  return spots.map(([x, z, k, mood], i) => {
    const p = new Toon(LOOKS[(k ?? i) % LOOKS.length], { height: 1.7, res: 0.5 });
    p.set({ mood: mood || "neutral", armL: [8, 6], armR: [8, 6] });
    p.at(x, 0, z);
    scene.add(p.group);
    return p;
  });
}
var SHOPS = [
  ["BAKERY", "#C9603F", "brick", "#D8343F"],
  ["GENERAL STORE", "#6FA3B8", "wood", null],
  ["BARBER", "#E3C16F", "wood", "#3C6FB6"],
  ["GRAND HOTEL", "#9E4A3A", "brick", null],
  ["POST OFFICE", "#7FA36A", "wood", null],
  ["TAILOR", "#B98ACF", "wood", "#2E4E3F"],
  ["HARDWARE", "#B5553C", "brick", "#F2C230"],
  ["CAFE", "#E58A6A", "wood", "#2E4E3F"],
  ["BANK", "#C7B79A", "brick", null],
  ["PHARMACY", "#5FA39A", "wood", "#D8343F"],
  ["SHOES", "#D9A34A", "wood", null],
  ["THEATER", "#A3485A", "brick", "#F2C230"]
];
function mainStreet(E, { len = 70, people = true, cars = true, dusk = false, shops = SHOPS } = {}) {
  const buildings = [];
  const scene = new THREE4.Scene;
  sky(scene, dusk ? { top: "#3B3F8F", horizon: "#F59C6B" } : {});
  const sun = sunlight(scene, E, dusk ? { col: "#FFB27A", i: 2, pos: [-8, 5, -14], sky: "#B9A2D8", ground: "#7A4A3A", hemi: 0.9 } : { pos: [9, 13, 5], box: 16 });
  clouds(scene, [[-30, 26, -100, 16], [20, 34, -110, 20], [55, 22, -90, 12], [-60, 30, -80, 14], [5, 20, -70, 9]], dusk ? "#FFC9B0" : "#FFFFFF");
  hills(scene, { z: -len - 30 });
  const road = new THREE4.Mesh(new THREE4.PlaneGeometry(8, len + 40), mat({ map: T.dirt("#C98A52", [2, (len + 40) / 8]) }));
  road.rotation.x = -Math.PI / 2;
  road.position.set(0, 0, -len / 2 + 10);
  road.receiveShadow = true;
  scene.add(road);
  const ground = new THREE4.Mesh(new THREE4.PlaneGeometry(300, 300), mat({ map: T.grass("#79C25A", [60, 60]) }));
  ground.rotation.x = -Math.PI / 2;
  ground.position.y = -0.02;
  ground.receiveShadow = true;
  scene.add(ground);
  [-1, 1].forEach((s) => {
    const walk = box(2.2, 0.18, len + 20, mat({ map: T.planks("#A8693E", [(len + 20) / 3, 1]) }), s * 5.1, 0.09, -len / 2 + 5, scene);
    walk.material.map.rotation = Math.PI / 2;
    let z = 8, k = s > 0 ? 0 : 5;
    while (z > -len) {
      const [name, wall, kind, aw] = shops[k % shops.length];
      const w = 5 + rnd(k) * 2.5, h = 6 + rnd(k * 3) * 3.5;
      k++;
      const front = mat({ map: facade(name, { wall, kind, awning: aw, w, h, seed: k }) }), side = mat({ color: shade(wall, 0.85) });
      const b = new THREE4.Mesh(new THREE4.BoxGeometry(6, h, w), s < 0 ? [front, side, side, side, side, side] : [side, front, side, side, side, side]);
      b.position.set(s * 9.2, h / 2, z - w / 2);
      b.castShadow = b.receiveShadow = true;
      scene.add(b);
      buildings.push({ name, side: s, z: z - w / 2, w, h, x: s * 6.2 });
      if (aw) {
        const a = box(1.4, 0.06, w - 0.4, mat({ color: aw }), s * 5.6, 2.9, z - w / 2, scene);
        a.rotation.z = s * 0.35;
      }
      z -= w + 0.15;
    }
    for (let zz = 4;zz > -len; zz -= 12) {
      const p = pole(7.5);
      p.position.set(s * 4.4, 0, zz);
      scene.add(p);
    }
    for (let zz = -2;zz > -len; zz -= 16) {
      const l = lamp();
      l.position.set(s * 4.3, 0.18, zz);
      scene.add(l);
    }
  });
  for (let zz = 4;zz > -len + 12; zz -= 12)
    [-1, 1].forEach((s) => [-0.8, -0.3, 0.3, 0.8].forEach((dx) => scene.add(wire(new THREE4.Vector3(s * 4.4 + dx, 6.95, zz), new THREE4.Vector3(s * 4.4 + dx, 6.95, zz - 12), 0.45))));
  for (let zz = -6;zz > -len; zz -= 22)
    scene.add(bunting(new THREE4.Vector3(-4.4, 6.2, zz), new THREE4.Vector3(4.4, 6.2, zz - 2)));
  [[-4.6, -3], [4.7, -9], [-4.8, -20], [4.6, -27]].forEach(([x, z], i) => {
    const o = i % 2 ? barrel() : crate();
    o.position.set(x, 0.18, z);
    scene.add(o);
  });
  const carsList = [];
  if (cars)
    [[2, -14, "#2C3A4A"], [-2, -30, "#7A2E2E"], [2, -44, "#2E5A44"]].forEach(([x, z, c]) => {
      const v = car(c);
      v.position.set(x, 0, z);
      if (x < 0)
        v.rotation.y = Math.PI;
      scene.add(v);
      carsList.push(v);
    });
  const crowd = people ? extras(scene, [[-5.3, -6], [5, -10, 2], [-5, -15], [5.4, -18, 4], [-5.4, -24, 3], [4.9, -31, 5], [-5, -36, 6], [5.3, -42, 7], [-5.2, -48, 1]]) : [];
  if (dusk)
    crowd.forEach((p) => p.tint("#D9B9C9"));
  return { scene, sun, people: crowd, cars: carsList, buildings, faceAll(cam) {
    crowd.forEach((p) => p.face(cam));
  } };
}
var INTERIOR_PRESETS = {
  shop: { wall: "#9E2E3A", motif: "#B8414C", floor: "#A8693E", signText: "NO CREDIT · CASH ONLY", counterOn: true, picture: true },
  home: { wall: "#7FA36A", motif: "#93B584", floor: "#8A5A36", signText: null, counterOn: false, picture: true },
  office: { wall: "#5F7A8A", motif: "#72909E", floor: "#A8693E", signText: "OFFICE", counterOn: true, picture: false },
  classroom: { wall: "#C9A86A", motif: "#D8BC82", floor: "#A8693E", signText: null, counterOn: false, picture: true }
};
function interior(E, opts = {}) {
  const P = INTERIOR_PRESETS[opts.preset ?? "shop"];
  const w = opts.w ?? 8, depth = opts.depth ?? 7, h = opts.h ?? 4.2, ceiling = opts.ceiling ?? "#24524A";
  const view = opts.view ?? null, bulbs = opts.bulbs ?? true, litter = opts.litter ?? true;
  const wall = opts.wall ?? P.wall, motif = opts.motif ?? P.motif, floor = opts.floor ?? P.floor;
  const signText = opts.signText !== undefined ? opts.signText : P.signText;
  const picture = opts.picture !== undefined ? opts.picture : P.picture;
  const counterOn = opts.counterOn !== undefined ? opts.counterOn : P.counterOn;
  const scene = new THREE4.Scene;
  scene.background = new THREE4.Color("#1A1418");
  scene.environment = E.envMap;
  scene.environmentIntensity = 0.35;
  const fl = new THREE4.Mesh(new THREE4.PlaneGeometry(w, depth + 2), mat({ map: T.planks(floor, [w / 2.5, (depth + 2) / 2.5]) }));
  fl.rotation.x = -Math.PI / 2;
  fl.position.z = -depth / 2 + 1;
  fl.receiveShadow = true;
  scene.add(fl);
  fl.material.map.rotation = Math.PI / 2;
  const wp = (len) => mat({ map: T.wallpaper(wall, motif, [len / 1.4, h / 1.4]) });
  const wain = (len) => mat({ map: T.planks("#6B3E26", [len / 2, 0.5]) });
  const ww = 3.2, wh = 1.9, wy = 1.1;
  const back = new THREE4.Group;
  back.position.z = -depth;
  scene.add(back);
  box((w - ww) / 2, h, 0.2, wp((w - ww) / 2), -(w + ww) / 4, h / 2, 0, back);
  box((w - ww) / 2, h, 0.2, wp((w - ww) / 2), (w + ww) / 4, h / 2, 0, back);
  box(ww, wy, 0.2, wain(ww), 0, wy / 2, 0, back);
  box(ww, h - wy - wh, 0.2, wp(ww), 0, h - (h - wy - wh) / 2, 0, back);
  const frameM = mat({ color: "#2F5C52" });
  box(ww + 0.2, 0.14, 0.3, frameM, 0, wy, 0.05, back);
  box(ww + 0.2, 0.14, 0.3, frameM, 0, wy + wh, 0.05, back);
  box(0.12, wh, 0.3, frameM, -ww / 2, wy + wh / 2, 0.05, back);
  box(0.12, wh, 0.3, frameM, ww / 2, wy + wh / 2, 0.05, back);
  box(0.06, wh, 0.1, frameM, 0, wy + wh / 2, 0.05, back);
  const viewPlane = new THREE4.Mesh(new THREE4.PlaneGeometry(ww * 2.6, wh * 2.6), new THREE4.MeshBasicMaterial({ color: "#BFE4F7", toneMapped: false }));
  viewPlane.position.set(0, wy + wh / 2, -2.5);
  back.add(viewPlane);
  if (view) {
    viewPlane.material.map = view;
    viewPlane.material.color.set("#FFFFFF");
  }
  [-1, 1].forEach((s) => {
    const sw = box(0.2, h, depth + 2, wp(depth + 2), s * w / 2, h / 2, -depth / 2 + 1, scene);
    sw.material.map.repeat.set((depth + 2) / 1.4, h / 1.4);
  });
  [-1, 1].forEach((s) => box(0.24, 1, depth + 2, wain(depth + 2), s * (w / 2 - 0.02), 0.5, -depth / 2 + 1, scene));
  box((w - ww) / 2, 1, 0.24, wain(2), -(w + ww) / 4, 0.5, -depth + 0.02, scene);
  box((w - ww) / 2, 1, 0.24, wain(2), (w + ww) / 4, 0.5, -depth + 0.02, scene);
  const ceil = box(w, 0.2, depth + 2, mat({ map: T.wallpaper(ceiling, shade(ceiling, 1.25), [w / 1.2, (depth + 2) / 1.2]) }), 0, h + 0.1, -depth / 2 + 1, scene);
  box(w, 0.16, 0.3, mat({ color: "#E8DCC0" }), 0, h - 0.08, -depth + 0.1, scene);
  const counter = new THREE4.Group;
  counter.position.set(w / 2 - 1.3, 0, -depth / 2 + 0.6);
  if (counterOn)
    scene.add(counter);
  box(1, 1.1, depth - 1.5, mat({ map: T.planks("#7A3E24", [1, 2]) }), 0, 0.55, 0, counter);
  box(1.2, 0.08, depth - 1.3, mat({ map: T.felt("#1F5A45") }), 0, 1.14, 0, counter);
  box(0.1, 0.06, depth - 1.3, mat({ color: "#C9A13B", metalness: 0.6, roughness: 0.3 }), -0.62, 1.12, 0, counter);
  const pic = canvasTex("pic", 256, 200, (g) => {
    g.fillStyle = "#E9DCC0";
    g.fillRect(0, 0, 256, 200);
    g.strokeStyle = "#3A2A22";
    g.lineWidth = 6;
    g.beginPath();
    g.moveTo(40, 140);
    g.quadraticCurveTo(90, 60, 140, 120);
    g.quadraticCurveTo(170, 80, 220, 140);
    g.stroke();
    g.fillStyle = "#C9603F";
    g.beginPath();
    g.arc(190, 60, 22, 0, 7);
    g.fill();
  });
  if (picture) {
    const pf = box(0.9, 0.7, 0.05, [mat({ color: "#C9A13B" }), mat({ color: "#C9A13B" }), mat({ color: "#C9A13B" }), mat({ color: "#C9A13B" }), mat({ map: pic }), mat({ color: "#C9A13B" })], -w / 2 + 0.13, 2.4, -depth / 2, scene);
    pf.rotation.y = Math.PI / 2;
  }
  if (signText) {
    const sb = signBoard(signText, { w: 2.2, h: 0.4, bg: "#3A2A2A", fg: "#F2E6CF" });
    sb.position.set(w / 2 - 0.13, 2.9, -depth / 2 + 0.6);
    sb.rotation.y = -Math.PI / 2;
    scene.add(sb);
  }
  const sun = new THREE4.DirectionalLight("#FFE4B8", 2.4);
  sun.position.set(-2, 6, -depth - 6);
  sun.target.position.set(0.5, 0, -1);
  sun.castShadow = true;
  sun.shadow.mapSize.set(2048, 2048);
  Object.assign(sun.shadow.camera, { left: -8, right: 8, top: 8, bottom: -8, near: 0.5, far: 30 });
  sun.shadow.bias = -0.0006;
  sun.shadow.camera.updateProjectionMatrix();
  sun.shadow.radius = 4;
  scene.add(sun, sun.target);
  scene.add(new THREE4.HemisphereLight("#FFE9D2", "#5A3A2A", 1.1));
  const fill = new THREE4.PointLight("#FFD9A8", 6, 12, 1.4);
  fill.position.set(0, 3.2, -1);
  scene.add(fill);
  const bulbLights = [];
  if (bulbs)
    [[-1.5, -2.2], [1.6, -4.5]].forEach(([x, z]) => {
      const cord = cyl(0.008, 0.008, 1, new THREE4.MeshBasicMaterial({ color: "#C9A13B" }), x, h - 0.5, z, scene, 4);
      const b = new THREE4.Mesh(new THREE4.SphereGeometry(0.1, 12, 8), new THREE4.MeshStandardMaterial({ color: "#FFF6DA", emissive: "#FFE7A0", emissiveIntensity: 2.2 }));
      b.position.set(x, h - 1.05, z);
      scene.add(b);
      const pl = new THREE4.PointLight("#FFD9A0", 3, 6, 1.6);
      pl.position.set(x, h - 1.15, z);
      scene.add(pl);
      bulbLights.push(pl);
    });
  const slips = new THREE4.InstancedMesh(new THREE4.PlaneGeometry(0.16, 0.1), new THREE4.MeshStandardMaterial({ color: "#F2E6CF", side: THREE4.DoubleSide }), 140);
  const m4 = new THREE4.Matrix4, q = new THREE4.Quaternion, e = new THREE4.Euler;
  for (let i = 0;i < 140; i++) {
    e.set(-Math.PI / 2, 0, rnd(i * 3) * 6.3);
    q.setFromEuler(e);
    m4.compose(new THREE4.Vector3((rnd(i) - 0.5) * (w - 1), 0.005 + i * 0.00005, -rnd(i * 7) * depth + 0.8), q, new THREE4.Vector3(1, 1, 1));
    slips.setMatrixAt(i, m4);
    slips.setColorAt(i, new THREE4.Color(["#F2E6CF", "#E9D7A5", "#F4C7B0", "#CFE3D0"][i % 4]));
  }
  slips.receiveShadow = true;
  if (litter)
    scene.add(slips);
  return { scene, sun: { key: sun, fills: [fill, ...bulbLights] }, counter, window: viewPlane, w, depth, h };
}
function meadow(E, { sunset = false, flowers = true, fenceOn = true, treesOn = true } = {}) {
  const scene = new THREE4.Scene;
  sky(scene, sunset ? { top: "#5B4FA8", horizon: "#FF9E6E" } : { top: "#3D8EE0", horizon: "#CBEAF8" });
  const sun = sunlight(scene, E, sunset ? { col: "#FFB27A", i: 2.4, pos: [-12, 4, -18], sky: "#C9A6E0", ground: "#6A4A3A", hemi: 1, box: 14 } : { pos: [8, 12, 6], box: 14 });
  clouds(scene, [[-30, 22, -90, 14], [25, 30, -100, 18], [60, 18, -80, 11], [-5, 16, -60, 8]], sunset ? "#FFB8A0" : "#FFFFFF");
  const geo = new THREE4.PlaneGeometry(200, 200, 100, 100);
  const p = geo.attributes.position;
  for (let i = 0;i < p.count; i++) {
    const x = p.getX(i), y = p.getY(i);
    p.setZ(i, Math.sin(x * 0.08) * 0.6 + Math.cos(y * 0.06 + 1) * 0.8 + (Math.abs(x) < 6 && Math.abs(y) < 10 ? 0 : 0));
  }
  geo.computeVertexNormals();
  const ground = new THREE4.Mesh(geo, mat({ map: T.grass(sunset ? "#7DB55A" : "#6DBB4A", [50, 50]) }));
  ground.rotation.x = -Math.PI / 2;
  ground.position.y = -0.5;
  ground.receiveShadow = true;
  scene.add(ground);
  const flat = new THREE4.Mesh(new THREE4.CircleGeometry(12, 48), mat({ map: T.grass(sunset ? "#7DB55A" : "#6DBB4A", [6, 6]) }));
  flat.rotation.x = -Math.PI / 2;
  flat.receiveShadow = true;
  scene.add(flat);
  hills(scene, { z: -70, col: sunset ? "#86B860" : "#79C25A" });
  (treesOn ? [[-7, -9, "round"], [8, -14, "round"], [-14, -20, "cypress"], [-11, -22, "cypress"], [14, -26, "cypress"], [17, -25, "cypress"]] : []).forEach(([x, z, k], i) => {
    const t = tree(1.1 + rnd(i) * 0.4, "#4E9A3C", k);
    t.position.set(x, 0, z);
    scene.add(t);
  });
  if (fenceOn) {
    const f = fence(16);
    f.position.set(-8, 0, -6);
    scene.add(f);
  }
  if (flowers) {
    const fl = new THREE4.InstancedMesh(new THREE4.IcosahedronGeometry(0.05, 0), new THREE4.MeshStandardMaterial({ roughness: 0.6 }), 1600), m4 = new THREE4.Matrix4;
    for (let i = 0;i < 1600; i++) {
      const a = rnd(i) * 6.28, r = 2 + rnd(i * 3.3) * 22;
      m4.makeTranslation(Math.cos(a) * r, 0.1, Math.sin(a) * r - 4);
      fl.setMatrixAt(i, m4);
      fl.setColorAt(i, new THREE4.Color(["#F2D23A", "#F29BB5", "#FFFFFF", "#B98ACF", "#F28C4A"][i % 5]));
    }
    scene.add(fl);
    const tufts = new THREE4.InstancedMesh(new THREE4.ConeGeometry(0.025, 0.16, 3), mat({ color: sunset ? "#6FA650" : "#5DA83E" }), 4000);
    for (let i = 0;i < 4000; i++) {
      const a = rnd(i * 1.7) * 6.28, r = 1 + rnd(i * 5.1) * 14;
      m4.makeTranslation(Math.cos(a) * r, 0.07, Math.sin(a) * r - 3);
      tufts.setMatrixAt(i, m4);
    }
    scene.add(tufts);
  }
  return { scene, sun };
}
function snapshot(E, scene, cam, w = 1024, h = 768) {
  const rt = new THREE4.WebGLRenderTarget(w, h, { samples: 4, type: THREE4.HalfFloatType });
  const r = E.renderer;
  const prev = r.getRenderTarget();
  r.setRenderTarget(rt);
  r.shadowMap.needsUpdate = true;
  r.render(scene, cam);
  r.setRenderTarget(prev);
  return rt.texture;
}

// src/core/timing.ts
var T2 = { dur: 0, lines: [], words: [] };
async function loadTiming() {
  const get = async (f) => {
    const r = await fetch(f, { cache: "no-store" });
    if (!r.ok)
      throw new Error(`missing ${f}: run voice.py + words.py (or estimate.py)`);
    return r.json();
  };
  const lines = await get("data/lines.json"), words = await get("data/words.json");
  T2.lines = lines.lines;
  T2.words = words;
  T2.dur = lines.duration;
  return T2;
}
var lineWords = (i) => T2.words.filter((w) => w.line === i);

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
    const next = caps[k + 1], lineEnd = T2.lines[c.line].end;
    c.e = next ? next.s - lineEnd > 0.6 && next.line !== c.line ? lineEnd + 0.25 : next.s : T2.dur;
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

// src/sheet.ts
await document.fonts.load("900 40px Inter");
await document.fonts.load("800 40px Inter");
await document.fonts.load("700 40px Caladea");
var GL = document.getElementById("gl");
var GL2 = document.getElementById("gl2");
GL.width = 648;
GL.height = 1152;
var E = makeEngine(GL, { w: 648, h: 1152, samples: 0 });
var E2 = makeEngine(GL2, { w: 1280, h: 860, samples: 0 });
var cam = camera(36);
var B2 = startBoard(document.getElementById("cv"), { title: "TALL TALE 3D: VIDEO STYLE SHEET", bg: "#1B1A22", line: "#43414F", head: "#FFF6E6", accent: "#F2C230" });
var shot = (x, y, w, renderFn, over) => frame(x, y, w, () => {
  renderFn();
  ctx.drawImage(GL, 0, 0, 1080, 1920);
  if (over)
    over();
});
var wide = (box, renderFn, over) => {
  renderFn();
  B2.save();
  B2.beginPath();
  B2.roundRect(box.x, box.y, box.w, box.h, 12);
  B2.clip();
  B2.drawImage(GL2, box.x, box.y, box.w, box.h);
  B2.restore();
  if (over)
    over();
};
var CAST = {
  hero: { body: "#E8734A", hat: "cap", hatCol: "#6E8A4F", neck: "scarf", neckCol: "#F2C230" },
  skeptic: { body: "#8C6FC9", hat: "bowler", hatCol: "#3B3346", neck: "tie", neckCol: "#E2B33C", pattern: "check", patCol: "#6A50A8", extra: ["mustache"] },
  clerk: { body: "#4FB3A9", hat: "visor", hatCol: "#2F8F5B", neck: "collar", pattern: "vest", patCol: "#2B5E73", extra: ["pencil"] },
  neighbor: { body: "#F2B8C6", hat: "flower", pattern: "apron", extra: ["glasses"] },
  kid: { body: "#F2C230", hat: "beanie", hatCol: "#D8343F", pattern: "overalls", extra: ["freckles"] },
  boss: { body: "#5C8FD6", hat: "tophat", hatCol: "#2A2638", neck: "bowtie", neckCol: "#D8343F", dots: 1, extra: ["monocle"] }
};
var S = mainStreet(E);
var hero = new Toon(CAST.hero);
S.scene.add(hero.group);
var b = panel(20, 80, 540, 520, 1, "HERO FRAME");
shot(b.x + 4, b.y, 246, () => {
  hero.at(-1, 0, -3);
  hero.set({ mood: "smug", armR: [30, -84], holdR: PROPS.mirror });
  cam.position.set(-0.45, 1.5, 0.1);
  cam.lookAt(-0.95, 1.3, -3);
  hero.face(cam);
  S.faceAll(cam);
  E.render(S.scene, cam, { focus: 3.1, aperture: 0.0012, maxblur: 0.008, bloom: 0.15 });
}, () => captionText("smiling,"));
[["2D CARTOON PEOPLE", "thick ink outlines"], ["INSIDE A 3D WORLD", "sun, shadows, depth"], ["CAMERA ALWAYS MOVES", "push-ins, pans, fly-overs"], ["A FUNNY TRUE STORY", "told like a tale"], ["ONE WORD AT A TIME", "white, centre screen"]].forEach(([a, c], i) => {
  small(a, b.x + 270, b.y + 34 + i * 86, "left", 16, "#FFF6E6");
  small(c.toUpperCase(), b.x + 270, b.y + 58 + i * 86, "left", 12, "#A9A6B8");
});
b = panel(570, 80, 690, 520, 2, "THE CAST");
{
  const g = B2;
  g.save();
  g.beginPath();
  g.roundRect(b.x, b.y, b.w, b.h - 8, 12);
  g.clip();
  const gr = g.createLinearGradient(0, b.y, 0, b.y + b.h);
  gr.addColorStop(0, "#9FD3F0");
  gr.addColorStop(0.72, "#CDEBF8");
  gr.addColorStop(0.72, "#E3B07A");
  gr.addColorStop(1, "#C98A52");
  g.fillStyle = gr;
  g.fillRect(b.x, b.y, b.w, b.h);
  const roles = [
    ["hero", "THE HERO", { mood: "smug", armR: [30, -84], holdR: PROPS.mirror }],
    ["skeptic", "THE SKEPTIC", { mood: "laugh", armL: [150, 20] }],
    ["clerk", "THE CLERK", { mood: "shock", armL: [40, -40], armR: [40, -40] }],
    ["neighbor", "NEIGHBOR", { mood: "worried", armR: [20, -110] }],
    ["kid", "THE KID", { mood: "happy", legs: "jump", armL: [140, 10], armR: [140, 10] }],
    ["boss", "THE BOSS", { mood: "proud", armL: [30, -70], holdL: PROPS.stack }]
  ];
  roles.forEach(([k, name, st], i) => {
    g.save();
    g.translate(b.x + 6 + i * 110, b.y + 40);
    g.scale(0.22, 0.22);
    drawToon(g, CAST[k], st);
    g.restore();
    small(name, b.x + 62 + i * 110, b.y + b.h - 30, "center", 13, "#2A2638");
  });
  g.restore();
  small("RECIPE: ROUND WHITE HEAD · BEAN BODY · STICK LIMBS · ONE HAT + ONE NECK THING", b.x + b.w / 2, b.y + b.h + 6, "center", 12, "#A9A6B8");
}
b = panel(1270, 80, 630, 520, 3, "FACES DO THE ACTING");
{
  const moods = ["neutral", "smug", "happy", "laugh", "shock", "worried", "angry", "sad", "sly", "proud", "tired", "dizzy"];
  moods.forEach((m, i) => {
    const cx = b.x + 52 + i % 6 * 100, cy = b.y + 20 + Math.floor(i / 6) * 215;
    B2.save();
    B2.beginPath();
    B2.roundRect(cx - 46, cy, 92, 150, 10);
    B2.fillStyle = "#2A2833";
    B2.fill();
    B2.clip();
    B2.translate(cx - 75, cy + 4);
    B2.scale(0.29, 0.29);
    drawToon(B2, CAST.hero, { mood: m, look: m === "sly" ? [1, 0] : [0, 0] });
    B2.restore();
    small(m.toUpperCase(), cx, cy + 176, "center", 13, "#FFF6E6");
  });
}
b = panel(20, 610, 760, 456, 4, "WORLDS");
var sw = 206;
var sx = (i) => b.x + 14 + i * (sw + 40);
var I = interior(E, { view: snapshot(E, S.scene, (() => {
  const c = new THREE5.PerspectiveCamera(40, 4 / 3, 0.1, 800);
  c.position.set(0, 1.8, 9);
  c.lookAt(0, 2.4, -20);
  S.faceAll(c);
  return c;
})()) });
shot(sx(0), b.y, sw, () => {
  hero.at(0.6, 0, -6);
  hero.set({ mood: "happy", legs: "walk", phase: 0.8 });
  cam.position.set(-1.6, 2.6, 4);
  cam.lookAt(0.6, 1.6, -14);
  hero.face(cam);
  S.faceAll(cam);
  E.render(S.scene, cam, { focus: 10, aperture: 0.0004, bloom: 0.15 });
});
{
  const clerk = new Toon(CAST.clerk), h3 = new Toon(CAST.hero);
  I.scene.add(clerk.group, h3.group);
  clerk.at(2.75, 0, -3.4);
  clerk.set({ mood: "shock", armL: [40, -40], armR: [40, -40] });
  h3.at(1.25, 0, -2.6);
  h3.set({ mood: "smug", armR: [26, -72], holdR: PROPS.stack });
  shot(sx(1), b.y, sw, () => {
    cam.position.set(0.1, 1.65, 2.2);
    cam.lookAt(1.9, 1.25, -3.2);
    clerk.face(cam);
    h3.face(cam);
    E.render(I.scene, cam, { focus: 4.6, aperture: 0.0008, bloom: 0.3 });
  });
}
var M = meadow(E, { sunset: true });
{
  const k = new Toon(CAST.kid);
  M.scene.add(k.group);
  k.at(0, 0, -2);
  k.set({ mood: "happy", armL: [150, 10] });
  k.tint("#FFE6DA");
  shot(sx(2), b.y, sw, () => {
    cam.position.set(0.9, 1, 2.6);
    cam.lookAt(0, 1.4, -2);
    k.face(cam);
    E.render(M.scene, cam, { focus: 4.8, aperture: 0.0012, bloom: 0.35 });
  });
}
["(A) MAIN STREET", "(B) INSIDE THE SHOP", "(C) THE MEADOW, SUNSET"].forEach((t, i) => small(t, sx(i) + sw / 2, b.y + sw * 16 / 9 + 24, "center", 14));
b = panel(790, 610, 520, 456, 5, "CAPTIONS");
wide({ x: b.x, y: b.y, w: b.w, h: b.h - 10 }, () => {
  hero.at(-0.2, 0, -3);
  hero.set({ mood: "shock", look: [0.6, 0] });
  const c3 = new THREE5.PerspectiveCamera(32, 1280 / 860, 0.05, 800);
  c3.position.set(0.5, 1.5, 0.6);
  c3.lookAt(-0.2, 1.4, -3);
  hero.face(c3);
  S.faceAll(c3);
  E2.render(S.scene, c3, { focus: 3.4, aperture: 0.003, maxblur: 0.012, bloom: 0.15 });
}, () => {
  const mid = b.x + b.w / 2;
  [["He bet", 60], ["everything", 160], ["a single", 260]].forEach(([w, y]) => {
    B2.save();
    B2.translate(mid, b.y + y);
    B2.scale(0.5, 0.5);
    captionText(w, 0, 0);
    B2.restore();
  });
  small("ONE WORD AT A TIME · WHITE · BLACK EDGE · CENTRE", mid, b.y + b.h - 34, "center", 13, "#FFFFFF");
});
b = panel(1320, 610, 580, 456, 6, "RULES");
rules(b, ['OPEN ON THE ENDING: "HE BET IT ALL..."', "THEN REWIND AND SHOW THE TRICK, STEP BY STEP.", "NEW SHOT EVERY 1.5-3 S; THE CAMERA NEVER STOPS.", "FACES REACT ON EVERY BEAT (SMUG, SHOCK, LAUGH).", "SHORT LINES OF DIALOGUE FROM THE CROWD.", "BRIGHT SUN BY DAY, PINK SUNSETS, WARM ROOMS.", "END ON A PUNCHLINE THAT FLIPS THE START."], b.y + 26, 16, 52);
window.SHEET_READY = true;
export {
  CAST
};
