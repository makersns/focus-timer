import { existsSync, mkdirSync, chmodSync, writeFileSync, rmSync, statSync } from 'node:fs';
import { randomBytes } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import os from 'node:os';

const index = process.argv.indexOf('--directory');
if (index !== -1 && (!process.argv[index + 1] || !path.isAbsolute(process.argv[index + 1]))) {
  throw new Error('--directory requires a private absolute directory path.');
}
const directory = index === -1
  ? path.join(os.homedir(), ...(process.platform === 'darwin'
    ? ['Library', 'Application Support', 'FocusPublisher', 'keys']
    : ['.local', 'share', 'FocusPublisher', 'keys']))
  : path.resolve(process.argv[index + 1] ?? '');
if (!path.isAbsolute(directory) || process.argv.slice(2).some((x, i, list) => x !== '--directory' && list[i - 1] !== '--directory')) {
  throw new Error('Usage: node scripts/android-create-signing-key.mjs [--directory PRIVATE_ABSOLUTE_PATH]');
}
const keystore = path.join(directory, 'focus-release.p12');
const configFile = path.join(directory, 'focus-signing.json');
if (existsSync(keystore) || existsSync(configFile)) throw new Error('Focus signing material already exists; never replace the key used for published APKs.');
if (existsSync(directory) && process.platform !== 'win32' && (statSync(directory).mode & 0o077) !== 0) {
  throw new Error('The signing directory must already be private (chmod 700), or choose a new dedicated directory.');
}
const previousMask = process.umask(0o077);
mkdirSync(directory, { recursive: true, mode: 0o700 });
const password = randomBytes(36).toString('base64url');
const env = { ...process.env, FOCUS_NEW_KEY_PASSWORD: password };
const keytool = process.env.JAVA_HOME
  ? path.join(process.env.JAVA_HOME, 'bin', process.platform === 'win32' ? 'keytool.exe' : 'keytool') : 'keytool';
try {
  const result = spawnSync(keytool, ['-genkeypair', '-keystore', keystore, '-storetype', 'PKCS12',
    '-alias', 'focus-release', '-keyalg', 'RSA', '-keysize', '3072', '-validity', '10000',
    '-dname', 'CN=Focus, OU=Focus Timer, O=Sugar',
    '-storepass:env', 'FOCUS_NEW_KEY_PASSWORD', '-keypass:env', 'FOCUS_NEW_KEY_PASSWORD'], { env, stdio: 'pipe' });
  if (result.error || result.status !== 0) throw new Error('Key generation failed. Check JAVA_HOME and directory access.');
  chmodSync(keystore, 0o600);
  writeFileSync(configFile, JSON.stringify({ keystore, alias: 'focus-release', storePassword: password, keyPassword: password }, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
  console.log('Focus release key created in: ' + directory);
  console.log('Private signing config: ' + configFile);
  console.log('Back up this directory securely before publishing. Future updates require the same key. Never add it to Git or upload it as a release asset.');
} catch (error) {
  if (!existsSync(configFile)) rmSync(keystore, { force: true });
  throw new Error('Focus signing setup did not complete; no existing key was replaced.');
} finally { delete env.FOCUS_NEW_KEY_PASSWORD; process.umask(previousMask); }
