import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { root, nativeEnvironment, argument } from './build-native-env.mjs';

const env = nativeEnvironment();
const changes = execFileSync('git', ['status', '--porcelain', '--untracked-files=normal'], { cwd: root, encoding: 'utf8' });
if (changes.trim()) throw new Error('Commit the reviewed source before collecting release assets; the working tree must be clean.');
const sourceCommit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
const platform = argument('--platform');
const target = argument('--target');
const extensions = {
  'macos-universal': ['.dmg'],
  'windows-x64': ['.exe'],
  'linux-x64': ['.AppImage', '.deb', '.rpm'],
};
if (!Object.hasOwn(extensions, platform)) throw new Error('Choose --platform macos-universal, windows-x64, or linux-x64.');
if (!target) throw new Error('--target must identify the exact build target.');
const configuration = JSON.parse(readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'));
const version = configuration.version;
const targetRoot = env.CARGO_TARGET_DIR ?? path.join(root, 'src-tauri', 'target');
const buildDirectory = path.join(targetRoot, target, 'release');
const provenance = JSON.parse(readFileSync(path.join(buildDirectory, '.focus-build-source.json'), 'utf8'));
if (provenance.sourceCommit !== sourceCommit || provenance.version !== version || provenance.target !== target) {
  throw new Error('These installers were not built from the current clean commit. Rebuild with scripts/build-native.mjs.');
}
const bundleRoot = path.join(buildDirectory, 'bundle');
const output = path.join(env.FOCUS_ARTIFACT_DIR ?? path.join(root, 'release-assets'), platform);
mkdirSync(output, { recursive: true });

function walk(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const filename = path.join(directory, entry.name);
    if (entry.isSymbolicLink()) return [];
    if (entry.isDirectory()) {
      return entry.name.endsWith('.app') ? [] : walk(filename);
    }
    return entry.isFile() ? [filename] : [];
  });
}
const allFiles = walk(bundleRoot);
const assets = extensions[platform].map(extension => {
  const matches = allFiles.filter(file => file.endsWith(extension));
  if (matches.length !== 1) throw new Error('Expected exactly one ' + extension + ' installer, found ' + matches.length);
  const suffix = platform === 'windows-x64' ? '-setup' : '';
  const name = 'Focus_' + version + '_' + platform + suffix + extension;
  const destination = path.join(output, name);
  if (existsSync(destination)) throw new Error('Refusing to replace an existing release asset: ' + destination);
  copyFileSync(matches[0], destination);
  const sha256 = createHash('sha256').update(readFileSync(destination)).digest('hex');
  return { name, bytes: statSync(destination).size, sha256 };
});
const manifest = { version, platform, target, sourceCommit, assets };
writeFileSync(path.join(output, 'Focus_' + version + '_' + platform + '.manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
writeFileSync(path.join(output, 'SHA256SUMS-' + platform + '.txt'), assets.map(asset => asset.sha256 + '  ' + asset.name).join('\n') + '\n');
console.log(JSON.stringify(manifest, null, 2));
console.log('Release files: ' + output);
