import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';

const root = '/tmp/envcap';
const out = path.join(root, 'out');
fs.mkdirSync(out, { recursive: true });
const types = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript' };

const server = http.createServer((req, res) => {
  if (req.method === 'POST' && req.url.startsWith('/save/')) {
    const name = decodeURIComponent(req.url.slice(6));
    const chunks = [];
    req.on('data', (c) => chunks.push(c));
    req.on('end', () => {
      fs.writeFileSync(path.join(out, name), Buffer.concat(chunks));
      console.log('saved', name);
      res.end('ok');
      if (name === 'done') setTimeout(() => process.exit(0), 300);
    });
    return;
  }
  const p = path.join(root, req.url === '/' ? 'env-capture.html' : req.url);
  if (!fs.existsSync(p)) { res.writeHead(404); res.end('nf'); return; }
  res.writeHead(200, { 'content-type': types[path.extname(p)] || 'application/octet-stream' });
  fs.createReadStream(p).pipe(res);
});
server.listen(8971, () => console.log('listening 8971'));
