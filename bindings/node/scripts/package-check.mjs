import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const root = path.resolve(import.meta.dirname, '..');
const monorepoHeader = path.resolve(root, '../../ffi/include/rich.h');
const packagedHeader = path.join(root, 'include/rich.h');
const headerPath = fs.existsSync(monorepoHeader) ? monorepoHeader : packagedHeader;
if (!fs.existsSync(headerPath)) throw new Error('canonical rich.h is unavailable');

const header = fs.readFileSync(headerPath, 'utf8');
const inventory = fs.readFileSync(path.join(root, 'src/native_symbols.inc'), 'utf8');
const headerNames = [...header.matchAll(/^(?:RichResult|void|uint32_t) (rich_[a-z0-9_]+)\s*\(/gm)]
  .map((match) => match[1])
  .sort();
const inventoryNames = [...inventory.matchAll(/RICH_NATIVE_SYMBOL\((rich_[a-z0-9_]+)\)/g)]
  .map((match) => match[1])
  .sort();

if (headerNames.length !== 125) {
  throw new Error(`expected 125 canonical symbols, found ${headerNames.length}`);
}
if (JSON.stringify(headerNames) !== JSON.stringify(inventoryNames)) {
  const missing = headerNames.filter((name) => !inventoryNames.includes(name));
  const extra = inventoryNames.filter((name) => !headerNames.includes(name));
  throw new Error(`native inventory drift; missing=${missing.join(',')} extra=${extra.join(',')}`);
}

const dryRun = JSON.parse(
  execFileSync('npm', ['pack', '--dry-run', '--json', '--ignore-scripts'], {
    cwd: root,
    encoding: 'utf8',
  }),
)[0];
const files = dryRun.files.map((file) => file.path);
for (const forbidden of ['node_modules/', 'build/', 'coverage/']) {
  if (files.some((file) => file.startsWith(forbidden))) {
    throw new Error(`package includes forbidden path ${forbidden}`);
  }
}
for (const required of [
  'binding.gyp',
  'examples/demo.js',
  'index.js',
  'index.d.ts',
  'src/addon.cc',
  'src/common.cc',
]) {
  if (!files.includes(required)) throw new Error(`package is missing ${required}`);
}
process.stdout.write(`Package and ${headerNames.length}-symbol inventory checks passed\n`);
