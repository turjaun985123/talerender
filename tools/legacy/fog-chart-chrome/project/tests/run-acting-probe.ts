// @ts-nocheck — Bun-runtime script (Bun/process globals); the template's tsc project covers
// src/ only, and acting.probe.ts itself is fully typed. This wrapper is never bundled into
// a project build.
// run-acting-probe.ts — entry wrapper for tests/acting.probe.ts.
// The engine imports the bare specifier 'three', which in the browser resolves through the
// page importmap to template/vendor/ and for the project build is --external; a plain
// `bun run` of the probe has neither. This wrapper bundles the probe with Bun.build plus a
// plugin mapping 'three' (+ addons) onto the vendored copy — the same module the browser
// loads — then imports the bundle. Usage:  bun run tests/run-acting-probe.ts
import { fileURLToPath } from 'url';

const vendor = (p: string) => fileURLToPath(new URL(`../vendor/${p}`, import.meta.url));
const result = await Bun.build({
  entrypoints: [fileURLToPath(new URL('./acting.probe.ts', import.meta.url))],
  target: 'bun',
  plugins: [{
    name: 'three-vendor-shim',
    setup(build) {
      build.onResolve({ filter: /^three$/ }, () => ({ path: vendor('three.module.js') }));
      build.onResolve({ filter: /^three\/addons\/(.*)$/ }, (args) => ({ path: vendor(`addons/${args.path.slice('three/addons/'.length)}`) }));
    },
  }],
});
if (!result.success) { for (const m of result.logs) console.error(m); process.exit(1); }
const bundle = `/tmp/acting-probe-${process.pid}.mjs`;
await Bun.write(bundle, await result.outputs[0].text());
await import(bundle);
