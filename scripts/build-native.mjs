import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { root, nativeEnvironment, argument } from './build-native-env.mjs';

if (Number(process.versions.node.split('.')[0]) < 24) {
  throw new Error('Focus builds require Node.js 24 or newer.');
}
const env = nativeEnvironment();
// DMG layout must not depend on Finder AppleEvents on a build machine.
env.CI = 'true';
delete env.TAURI_BUNDLER_DMG_IGNORE_CI;
function sourceState() {
  const commit = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' });
  const status = spawnSync('git', ['status', '--porcelain', '--untracked-files=normal'], { cwd: root, encoding: 'utf8' });
  return { commit: commit.status === 0 ? commit.stdout.trim() : null, clean: status.status === 0 && !status.stdout.trim() };
}
const before = sourceState();
function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, env, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
const target = argument('--target');
const bundles = argument('--bundles');
const options = new Set(['--target', '--bundles', '--skip-tests', '--debug', '--no-bundle']);
for (let index = 2; index < process.argv.length; index++) {
  const flag = process.argv[index];
  if (!options.has(flag)) throw new Error('Unknown build option: ' + flag);
  if (flag === '--target' || flag === '--bundles') index++;
}
run('cargo', ['fmt', '--manifest-path', 'src-tauri/Cargo.toml', '--', '--check']);
if (!process.argv.includes('--skip-tests')) {
  run('cargo', ['test', '--manifest-path', 'src-tauri/Cargo.toml', '--locked', '--lib']);
}
const args = [path.join(root, 'node_modules', '@tauri-apps', 'cli', 'tauri.js'), 'build', '--ci'];
if (target) args.push('--target', target);
if (bundles) args.push('--bundles', bundles);
if (process.argv.includes('--debug')) args.push('--debug');
if (process.argv.includes('--no-bundle')) args.push('--no-bundle');
args.push('--', '--locked');
run(process.execPath, args);

const after = sourceState();
const profile = process.argv.includes('--debug') ? 'debug' : 'release';
const buildDirectory = path.join(env.CARGO_TARGET_DIR ?? path.join(root, 'src-tauri', 'target'), target ?? '', profile);
mkdirSync(buildDirectory, { recursive: true });
const configuration = JSON.parse(readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'));
writeFileSync(path.join(buildDirectory, '.focus-build-source.json'), JSON.stringify({
  version: configuration.version,
  target: target ?? null,
  sourceCommit: before.clean && after.clean && before.commit === after.commit ? before.commit : null,
}, null, 2) + '\n');
