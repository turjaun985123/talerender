// Tiny static server + result receiver for the M6 telemetry
// harness. Serves the scratch project copy it is run from; the
// harness page POSTs each result to /save/<name> and provenance
// to /done, at which point we exit.
// Usage: node serve.mjs <out-dir> [port]
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';

const OUT = process.argv[2];
const PORT = Number(process.argv[3] || 8973);
fs.mkdirSync(OUT, { recursive: true });

const MIME = {
  '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript',
  '.json': 'application/json', '.otf': 'font/otf', '.ttf': 'font/ttf',
  '.woff2': 'font/woff2', '.wav': 'audio/wav', '.mp3': 'audio/mpeg',
  '.png': 'image/png', '.jpg': 'image/jpeg',
};

const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://x');
  if (req.method === 'POST' && (url.pathname.startsWith('/save/') || url.pathname === '/done')) {
    const chunks = [];
    req.on('data', (c) => chunks.push(c));
    req.on('end', () => {
      const body = Buffer.concat(chunks);
      if (url.pathname === '/done') {
        fs.writeFileSync(path.join(OUT, 'provenance.json'), body);
        console.log('DONE');
        res.end('ok');
        setTimeout(() => process.exit(0), 300);
      } else {
        const name = path.basename(url.pathname.slice(6));
        fs.writeFileSync(path.join(OUT, name), body);
        console.log('saved', name, body.length, 'bytes');
        res.end('ok');
      }
    });
    return;
  }
  const rel = url.pathname === '/' ? 'harness.html' : url.pathname.slice(1);
  const file = path.join(process.cwd(), rel);
  if (!file.startsWith(process.cwd()) || !fs.existsSync(file) || !fs.statSync(file).isFile()) {
    res.writeHead(404);
    res.end();
    return;
  }
  res.writeHead(200, { 'Content-Type': MIME[path.extname(file)] || 'application/octet-stream', 'Cache-Control': 'no-store' });
  fs.createReadStream(file).pipe(res);
});
server.listen(PORT, '127.0.0.1', () => console.log('serving on', PORT));
