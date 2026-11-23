import { existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const root = fileURLToPath(new URL('../', import.meta.url));

/** Local machine paths stay in ignored .data, never in source or CI. */
export function nativeEnvironment() {
  const env = { ...process.env };
  const manifestPath = path.join(root, '.data', 'native-build.json');
  if (process.platform === 'darwin' && existsSync(manifestPath) && !env.CARGO_TARGET_DIR) {
    const local = JSON.parse(readFileSync(manifestPath, 'utf8'));
    if (!path.isAbsolute(local.mount) || !existsSync(local.mount)
        || statSync(local.mount).dev === statSync(root).dev) {
      throw new Error('The dedicated build volume is not mounted. Mount it or set CARGO_TARGET_DIR explicitly.');
    }
    for (const key of ['cargoTarget', 'tmp', 'artifacts']) {
      if (!path.resolve(local[key]).startsWith(path.resolve(local.mount) + path.sep)) {
        throw new Error('Invalid local build path: ' + key);
      }
    }
    env.CARGO_TARGET_DIR = local.cargoTarget;
    env.TMPDIR = local.tmp;
    env.FOCUS_ARTIFACT_DIR ??= local.artifacts;
  }
  return env;
}

export function argument(name, fallback) {
  const index = process.argv.indexOf(name);
  if (index === -1) return fallback;
  const value = process.argv[index + 1];
  if (!value || value.startsWith('--')) throw new Error('Missing value for ' + name);
  return value;
}
