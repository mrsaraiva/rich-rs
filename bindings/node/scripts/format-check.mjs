import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { execFileSync } from 'node:child_process';

const root = path.resolve(import.meta.dirname, '..');
const extensions = new Set([
  '.cc',
  '.h',
  '.inc',
  '.js',
  '.mjs',
  '.ts',
  '.json',
  '.md',
  '.gyp',
  '.supp',
]);
const excluded = new Set(['build', 'node_modules', 'coverage', 'include']);
const failures = [];

function visit(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (excluded.has(entry.name) || entry.name.endsWith('.tgz')) continue;
    const filename = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      visit(filename);
    } else if (extensions.has(path.extname(entry.name)) || entry.name === 'binding.gyp') {
      const contents = fs.readFileSync(filename, 'utf8');
      const relative = path.relative(root, filename);
      if (!contents.endsWith('\n')) failures.push(`${relative}: missing final newline`);
      if (/\r/.test(contents)) failures.push(`${relative}: CRLF is not allowed`);
      if (/\t/.test(contents)) failures.push(`${relative}: tabs are not allowed`);
      contents.split('\n').forEach((line, index) => {
        if (/\s+$/.test(line)) failures.push(`${relative}:${index + 1}: trailing whitespace`);
      });
    }
  }
}

visit(root);
for (const filename of ['package.json', 'binding.gyp']) {
  JSON.parse(fs.readFileSync(path.join(root, filename), 'utf8'));
}

const clangCandidates = process.env.CLANG_FORMAT
  ? [process.env.CLANG_FORMAT]
  : ['clang-format', 'clang-format-22', '/home/linuxbrew/.linuxbrew/bin/clang-format'];
const clangFormat = clangCandidates.find((candidate) => {
  try {
    execFileSync(candidate, ['--version'], { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
});
if (clangFormat === undefined) {
  failures.push('clang-format is required but no working executable was found');
} else {
  try {
    execFileSync(
      clangFormat,
      [
        '--dry-run',
        '--Werror',
        path.join(root, 'src/addon.cc'),
        path.join(root, 'src/common.cc'),
        path.join(root, 'src/common.h'),
        path.join(root, 'test/foreign-addon/foreign_owner.cc'),
      ],
      { stdio: 'inherit' },
    );
  } catch (error) {
    failures.push(`clang-format check failed (${error.message})`);
  }
}

if (failures.length > 0) {
  process.stderr.write(`${failures.join('\n')}\n`);
  process.exit(1);
}
process.stdout.write('Node binding formatting checks passed\n');
