import { createHash } from 'node:crypto';
import { readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { root } from './build-native-env.mjs';

const options = process.argv.slice(2);
const includeAndroid = options.includes('--include-android');
const directories = options.filter(value => value !== '--include-android');
if (directories.length > 1 || directories.some(value => value.startsWith('--'))) {
  throw new Error('Usage: node scripts/release-verify.mjs [directory] [--include-android]');
}
const directory = path.resolve(directories[0] ?? path.join(root, 'release-assets'));
const expectedCommit = process.env.GITHUB_SHA ?? execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
const version = JSON.parse(readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8')).version;
const required = {
  'macos-universal': ['Focus_' + version + '_macos-universal.dmg'],
  'windows-x64': ['Focus_' + version + '_windows-x64-setup.exe'],
  'linux-x64': ['Focus_' + version + '_linux-x64.AppImage', 'Focus_' + version + '_linux-x64.deb', 'Focus_' + version + '_linux-x64.rpm'],
};
if (includeAndroid) required['android-arm64'] = ['Focus_' + version + '_android-arm64.apk'];
const targets = {
  'macos-universal': 'universal-apple-darwin',
  'windows-x64': 'x86_64-pc-windows-msvc',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'android-arm64': 'aarch64-linux-android',
};
const files = readdirSync(directory);
const manifests = files.filter(name => name.endsWith('.manifest.json'));
if (manifests.length !== Object.keys(required).length) throw new Error('The release needs exactly the expected platform manifests.');
const seen = new Set();
const checked = new Set();
for (const filename of manifests) {
  const manifest = JSON.parse(readFileSync(path.join(directory, filename), 'utf8'));
  if (!Object.hasOwn(required, manifest.platform) || seen.has(manifest.platform)) throw new Error('Invalid or duplicate platform manifest.');
  if (manifest.version !== version || manifest.sourceCommit !== expectedCommit) throw new Error('Release assets have a different source version.');
  if (manifest.target !== targets[manifest.platform]) throw new Error('Unexpected release architecture.');
  if (filename !== 'Focus_' + version + '_' + manifest.platform + '.manifest.json') throw new Error('Unexpected manifest name.');
  if (manifest.platform === 'android-arm64') {
    if (manifest.signing?.type !== 'release' || !/^[a-f0-9]{64}$/i.test(manifest.signing?.certificateSha256 ?? '')) {
      throw new Error('Android requires verified release signing metadata; debug or unsigned APKs cannot be published.');
    }
    const expectedCertificate = process.env.FOCUS_ANDROID_CERT_SHA256?.replaceAll(':', '').toLowerCase();
    if (expectedCertificate && manifest.signing.certificateSha256.toLowerCase() !== expectedCertificate) {
      throw new Error('Android signing certificate does not match the publisher certificate.');
    }
  }
  if (JSON.stringify(manifest.assets.map(asset => asset.name).sort()) !== JSON.stringify([...required[manifest.platform]].sort())) {
    throw new Error('Platform installers are missing or unexpected.');
  }
  for (const asset of manifest.assets) {
    const assetPath = path.join(directory, asset.name);
    if (asset.name !== path.basename(asset.name)) throw new Error('Invalid asset name.');
    const hash = createHash('sha256').update(readFileSync(assetPath)).digest('hex');
    if (hash !== asset.sha256 || statSync(assetPath).size !== asset.bytes) throw new Error('Installer checksum mismatch: ' + asset.name);
    checked.add(asset.name);
  }
  checked.add(filename);
  seen.add(manifest.platform);
}
const lines = [...checked].sort().map(name => createHash('sha256').update(readFileSync(path.join(directory, name))).digest('hex') + '  ' + name);
writeFileSync(path.join(directory, 'SHA256SUMS.txt'), lines.join('\n') + '\n');
console.log('Verified ' + Object.values(required).flat().length + ' installers from source ' + expectedCommit + ' and generated SHA256SUMS.txt.');
