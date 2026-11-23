import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
import path from 'node:path';
import os from 'node:os';
import { root, nativeEnvironment } from './build-native-env.mjs';

const debug = process.argv.includes('--debug');
const release = process.argv.includes('--release');
const test = process.argv.includes('--test');
if (debug === release || process.argv.slice(2).some(value => !['--debug', '--release', '--test'].includes(value))) {
  throw new Error('Usage: node scripts/android-build.mjs (--debug [--test] | --release)');
}
const env = nativeEnvironment();
env.FOCUS_NODE = process.execPath;
env.PATH = [path.dirname(process.execPath), path.join(os.homedir(), '.cargo', 'bin'), env.PATH].filter(Boolean).join(path.delimiter);
env.ANDROID_HOME ??= env.ANDROID_SDK_ROOT ?? (process.platform === 'darwin'
  ? path.join(os.homedir(), 'Library', 'Android', 'sdk')
  : path.join(os.homedir(), 'Android', 'Sdk'));
env.NDK_HOME ??= env.ANDROID_NDK_HOME ?? path.join(env.ANDROID_HOME, 'ndk', '29.0.14206865');
env.ANDROID_NDK_HOME = env.NDK_HOME;
if (!env.JAVA_HOME && process.platform === 'darwin') {
  const studio = '/Applications/Android Studio.app/Contents/jbr/Contents/Home';
  if (existsSync(studio)) env.JAVA_HOME = studio;
}
if (env.JAVA_HOME) env.PATH = path.join(env.JAVA_HOME, 'bin') + path.delimiter + env.PATH;
if (!env.GRADLE_USER_HOME && env.CARGO_TARGET_DIR) {
  env.GRADLE_USER_HOME = path.join(path.dirname(env.CARGO_TARGET_DIR), 'focus-gradle');
}
env.GRADLE_OPTS = [env.GRADLE_OPTS, '-Dorg.gradle.daemon=false'].filter(Boolean).join(' ');
const toolSuffix = process.platform === 'win32' ? '.bat' : '';
const buildTools = path.join(env.ANDROID_HOME, 'build-tools', '36.1.0');
for (const required of [
  path.join(env.ANDROID_HOME, 'platforms', 'android-36', 'android.jar'),
  path.join(env.NDK_HOME, 'source.properties'),
  path.join(buildTools, 'apksigner' + toolSuffix),
]) {
  if (!existsSync(required)) throw new Error('Missing Android dependency: ' + required + '. Install the documented SDK component and review any license first.');
}
const ndk = readFileSync(path.join(env.NDK_HOME, 'source.properties'), 'utf8');
if (!/Pkg.Revision\s*=\s*29\.0\.14206865/.test(ndk)) throw new Error('Focus requires Android NDK 29.0.14206865.');
function run(command, args, options = {}) {
  const result = spawnSync(command, args, { cwd: root, env, stdio: 'inherit', ...options });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(path.basename(command) + ' failed (' + result.status + ').');
}
function capture(command, args, options = {}) {
  return execFileSync(command, args, { cwd: root, env, encoding: 'utf8', ...options });
}
let sourceCommit = null;
try { sourceCommit = capture('git', ['rev-parse', '--verify', 'HEAD'], { stdio: ['ignore', 'pipe', 'ignore'] }).trim(); }
catch { if (release) throw new Error('A reviewed Git commit is required for release builds.'); }
const status = () => capture('git', ['status', '--porcelain', '--untracked-files=normal']).trim();
if (release && status()) throw new Error('Commit the reviewed source before building a release; the working tree must be clean.');
const config = JSON.parse(readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'));
const version = config.version;
const android = path.join(root, 'src-tauri', 'gen', 'android');
const outputRoot = path.join(android, 'app', 'build', 'outputs', 'apk');
const profile = debug ? 'debug' : 'release';
const outputDirectory = path.join(outputRoot, 'universal', profile);
// Only this build variant is removed, so stale APKs cannot be mistaken for a new build.
rmSync(outputDirectory, { recursive: true, force: true });
const args = [path.join(root, 'node_modules', '@tauri-apps', 'cli', 'tauri.js'),
  'android', 'build', '--apk', '--target', 'aarch64', '--ci'];
if (debug) args.push('--debug');
if (env.FOCUS_ANDROID_VERBOSE === '1') args.push('--verbose');
run(process.execPath, args);
if (test) {
  const gradle = path.join(android, process.platform === 'win32' ? 'gradlew.bat' : 'gradlew');
  run(gradle, [':app:testArm64DebugUnitTest', '-x', ':app:rustBuildArm64Debug',
    '-PabiList=arm64-v8a', '-ParchList=arm64', '-PtargetList=aarch64', '--no-daemon'],
    { cwd: android });
}
if (release && (status() || capture('git', ['rev-parse', 'HEAD']).trim() !== sourceCommit)) {
  throw new Error('The source commit or working tree changed during the build. Review and rebuild before signing.');
}
const apks = readdirSync(outputDirectory).filter(name => name.endsWith('.apk'));
if (apks.length !== 1) throw new Error('Expected one arm64-only APK, found ' + apks.length);
const sourceApk = path.join(outputDirectory, apks[0]);
const evidence = path.join(root, 'native-evidence', 'android');
mkdirSync(evidence, { recursive: true });
if (debug) {
  const debugInfo = { version, profile: 'debug', platform: 'android-arm64', target: 'aarch64-linux-android',
    sourceCommit, unitTests: test, apk: path.relative(root, sourceApk) };
  writeFileSync(path.join(evidence, 'debug-build.json'), JSON.stringify(debugInfo, null, 2) + '\n');
  console.log('Debug APK (never publish): ' + sourceApk);
} else {
  const signingFile = env.FOCUS_ANDROID_SIGNING_CONFIG;
  if (!signingFile || !path.isAbsolute(signingFile)) {
    throw new Error('Set FOCUS_ANDROID_SIGNING_CONFIG to a private absolute JSON path. No debug-signing fallback is permitted.');
  }
  if (path.resolve(signingFile).startsWith(path.resolve(root) + path.sep)) throw new Error('Keep the signing configuration outside the repository.');
  const signingStat = statSync(signingFile);
  if (process.platform !== 'win32' && (signingStat.mode & 0o077) !== 0) throw new Error('Signing config must be private (chmod 600).');
  let signing;
  try { signing = JSON.parse(readFileSync(signingFile, 'utf8')); }
  catch { throw new Error('Could not read the private signing configuration.'); }
  for (const key of ['keystore', 'alias', 'storePassword', 'keyPassword']) {
    if (typeof signing[key] !== 'string' || !signing[key]) throw new Error('Incomplete private signing configuration.');
  }
  if (!path.isAbsolute(signing.keystore) || !existsSync(signing.keystore)) throw new Error('Private Focus keystore not found.');
  if (path.resolve(signing.keystore).startsWith(path.resolve(root) + path.sep)) throw new Error('Keep the signing key outside the repository.');
  if (process.platform !== 'win32' && (statSync(signing.keystore).mode & 0o077) !== 0) throw new Error('Keystore must be private (chmod 600).');
  const output = path.join(env.FOCUS_ARTIFACT_DIR ?? path.join(root, 'release-assets'), 'android-arm64');
  mkdirSync(output, { recursive: true });
  const name = 'Focus_' + version + '_android-arm64.apk';
  const destination = path.join(output, name);
  if (existsSync(destination)) throw new Error('Refusing to replace an existing release asset: ' + destination);
  const aligned = path.join(outputDirectory, 'focus-aligned.apk');
  const zipalign = path.join(buildTools, process.platform === 'win32' ? 'zipalign.exe' : 'zipalign');
  run(zipalign, ['-P', '16', '-f', '4', sourceApk, aligned]);
  const secretEnv = { ...env, FOCUS_SIGN_STORE_PASSWORD: signing.storePassword, FOCUS_SIGN_KEY_PASSWORD: signing.keyPassword };
  const apksigner = path.join(buildTools, 'apksigner' + toolSuffix);
  try {
    run(apksigner, ['sign', '--ks', signing.keystore, '--ks-key-alias', signing.alias,
      '--ks-pass', 'env:FOCUS_SIGN_STORE_PASSWORD', '--key-pass', 'env:FOCUS_SIGN_KEY_PASSWORD',
      '--out', destination, aligned], { env: secretEnv, stdio: 'pipe' });
    const verification = capture(apksigner, ['verify', '--verbose', '--print-certs', destination]);
    const certificateSha256 = /Signer #1 certificate SHA-256 digest: ([0-9a-f]{64})/i.exec(verification)?.[1]?.toLowerCase();
    if (!certificateSha256 || /Android Debug/i.test(verification)) throw new Error('Release certificate verification failed.');
    run(zipalign, ['-c', '-P', '16', '4', destination]);
    const asset = { name, bytes: statSync(destination).size, sha256: createHash('sha256').update(readFileSync(destination)).digest('hex') };
    const manifest = { version, platform: 'android-arm64', target: 'aarch64-linux-android', sourceCommit,
      assets: [asset], signing: { type: 'release', certificateSha256 } };
    writeFileSync(path.join(output, 'Focus_' + version + '_android-arm64.manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
    writeFileSync(path.join(output, 'SHA256SUMS-android-arm64.txt'), asset.sha256 + '  ' + name + '\n');
    if (status() || capture('git', ['rev-parse', 'HEAD']).trim() !== sourceCommit) throw new Error('Source changed during signing.');
    console.log(JSON.stringify(manifest, null, 2));
    console.log('Signed release APK: ' + destination);
  } catch (error) {
    rmSync(destination, { force: true });
    // Do not echo child-process options, private configuration, or environment values.
    throw new Error('Android release signing or verification failed; no publishable APK was retained.');
  } finally {
    rmSync(aligned, { force: true });
    delete secretEnv.FOCUS_SIGN_STORE_PASSWORD;
    delete secretEnv.FOCUS_SIGN_KEY_PASSWORD;
  }
}
