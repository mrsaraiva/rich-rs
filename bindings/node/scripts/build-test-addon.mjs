import { execFileSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';

const root = path.resolve(import.meta.dirname, '..');
const executable = path.join(
  root,
  'node_modules',
  '.bin',
  process.platform === 'win32' ? 'node-gyp.cmd' : 'node-gyp',
);
execFileSync(executable, ['rebuild', '--directory', path.join(root, 'test/foreign-addon')], {
  stdio: 'inherit',
});
