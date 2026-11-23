import assert from 'node:assert/strict';
import { createWriteStream, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { spawn, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

if (process.platform !== 'win32' || process.env.CI !== 'true') {
  throw new Error('Native foreground tests run only on the disposable Windows CI desktop.');
}
const root = fileURLToPath(new URL('..', import.meta.url));
const app = path.resolve(process.env.FOCUS_TEST_BINARY ?? path.join(root, 'src-tauri/target/debug/focus-timer.exe'));
assert(existsSync(app), 'Build the native Focus executable before running this test.');
const evidence = path.join(root, 'native-evidence', 'windows');
mkdirSync(evidence, { recursive: true });
const report = { sourceCommit: process.env.GITHUB_SHA ?? null, platform: 'windows', checks: [], passed: false };
const log = createWriteStream(path.join(evidence, 'tauri-driver.log'));
const driverArgs = ['--port', '4444'];
if (process.env.FOCUS_NATIVE_DRIVER) driverArgs.push('--native-driver', process.env.FOCUS_NATIVE_DRIVER);
const driver = spawn('tauri-driver.exe', driverArgs, { cwd: root, stdio: ['ignore', 'pipe', 'pipe'] });
driver.stdout.pipe(log, { end: false });
driver.stderr.pipe(log, { end: false });
let driverError;
driver.on('error', error => { driverError = error; });
const endpoint = 'http://127.0.0.1:4444';
let session;
let mainHandle;
let miniHandle;

const sleep = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds));
async function until(callback, timeout = 15000) {
  const start = Date.now();
  let lastError;
  while (Date.now() - start < timeout) {
    if (driverError) throw driverError;
    try {
      const result = await callback();
      if (result) return result;
    } catch (error) { lastError = error; }
    await sleep(150);
  }
  throw lastError ?? new Error('Native UI did not reach the expected state.');
}
async function request(method, route, body) {
  const response = await fetch(endpoint + route, {
    method,
    headers: { 'Content-Type': 'application/json' },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    signal: AbortSignal.timeout(35000),
  });
  const data = await response.json();
  if (!response.ok || data.value?.error) {
    throw new Error(method + ' ' + route + ': ' + JSON.stringify(data.value ?? data));
  }
  return data.value ?? data;
}
const command = (method, route, body) => request(method, '/session/' + session + route, body);
const execute = (script, args = []) => command('POST', '/execute/sync', { script, args });
async function invoke(name, args = {}) {
  const answer = await command('POST', '/execute/async', {
    script: 'const done = arguments[arguments.length - 1]; window.__TAURI_INTERNALS__.invoke(arguments[0], arguments[1]).then(value => done({value}), error => done({error: String(error)}));',
    args: [name, args],
  });
  if (answer.error) throw new Error(name + ': ' + answer.error);
  return answer.value;
}
const snapshot = () => invoke('get_snapshot');
const switchTo = handle => command('POST', '/window', { handle });
async function findView(view) {
  const handles = await command('GET', '/window/handles');
  for (const handle of handles) {
    await switchTo(handle);
    const current = await execute('const p = new URLSearchParams(location.search); return p.get("view") ?? p.get("window") ?? "main";');
    if (current === view) return handle;
  }
  return null;
}
async function click(testId) {
  const element = await until(async () => {
    const found = await command('POST', '/element', { using: 'css selector', value: '[data-testid="' + testId + '"]' });
    const id = found['element-6066-11e4-a52e-4f735466cecf'];
    return await command('GET', '/element/' + id + '/enabled') ? id : null;
  });
  await command('POST', '/element/' + element + '/click', {});
}
async function screenshot(name) {
  const data = await command('GET', '/screenshot');
  writeFileSync(path.join(evidence, name + '.png'), Buffer.from(data, 'base64'));
}
function passed(name) {
  report.checks.push({ name, passed: true });
  console.log('PASS ' + name);
}
async function focusGuard(pinned) {
  const output = path.join(evidence, pinned ? 'focus-pinned.json' : 'focus-unpinned.json');
  const child = spawn('powershell.exe', [
    '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
    path.join(root, 'scripts/test-native-focus-guard.ps1'),
    '-ApplicationPath', app, '-Output', output, '-Pinned', String(pinned), '-Seconds', '6',
  ], { cwd: root, stdio: 'inherit' });
  const [code] = await once(child, 'exit');
  assert.equal(code, 0, 'Foreground and z-order probe failed.');
  const samples = JSON.parse(readFileSync(output, 'utf8').replace(/^\uFEFF/, '')).observations;
  assert(samples.length >= 40, 'The foreground probe did not observe enough native timer ticks.');
}

try {
  await until(async () => { const status = await request('GET', '/status'); return status.ready ? status : null; }, 30000);
  const created = await request('POST', '/session', {
    capabilities: { alwaysMatch: { 'tauri:options': { application: app } } },
  });
  session = created.sessionId;
  assert(session, 'tauri-driver did not return a session.');
  await command('POST', '/timeouts', { implicit: 0, script: 15000, pageLoad: 30000 });
  mainHandle = await until(() => findView('main'));
  await until(() => execute('return !!window.__TAURI_INTERNALS__ && !!document.querySelector("[data-testid=main-timer-toggle]");'));
  const info = await invoke('get_runtime_info');
  assert.equal(info.platform, 'windows');
  assert.equal(info.desktopWidget, true);
  assert.equal(info.homeScreenWidget, false);
  let current = await snapshot();
  assert.equal(current.timer.status, 'idle', 'A disposable runner should start with an unused timer.');
  await invoke('update_preferences', { preferences: {
    ...current.preferences,
    language: 'en',
    theme: 'light',
    notificationsEnabled: false,
    closeToTray: true,
    miniTimerEnabled: false,
    miniTimerAlwaysOnTop: true,
  } });
  await click('main-timer-toggle');
  const started = await until(async () => {
    const state = await snapshot();
    return state.timer.status === 'running' ? state : null;
  });
  assert(started.timer.runId);
  await screenshot('main-running');
  passed('native application starts and runs the Rust timer');

  await click('mini-toggle');
  miniHandle = await until(() => findView('mini'));
  await until(() => execute('return !!document.querySelector("[data-testid=mini-timer-toggle]");'));
  const fromMini = await snapshot();
  assert.equal(fromMini.timer.runId, started.timer.runId);
  assert.equal(fromMini.timer.status, 'running');
  assert.equal(fromMini.preferences.miniTimerEnabled, true);
  await screenshot('mini-running');
  await click('mini-timer-toggle');
  const paused = await until(async () => {
    const state = await snapshot();
    return state.timer.status === 'paused' ? state : null;
  });
  await switchTo(mainHandle);
  current = await snapshot();
  assert.equal(current.timer.runId, paused.timer.runId);
  assert.equal(current.timer.status, 'paused');
  assert.equal(current.timer.remainingMs, paused.timer.remainingMs);
  await screenshot('main-paused-from-mini');
  passed('mini timer pauses the same run shown in the main window');

  await click('main-timer-toggle');
  await until(async () => (await snapshot()).timer.status === 'running');
  await switchTo(miniHandle);
  current = await snapshot();
  if (current.preferences.miniTimerAlwaysOnTop) await click('mini-pin');
  await until(async () => !(await snapshot()).preferences.miniTimerAlwaysOnTop);
  await focusGuard(false);
  passed('an unpinned widget never steals foreground focus or raises itself across ticks');
  await switchTo(miniHandle);
  await click('mini-pin');
  await until(async () => (await snapshot()).preferences.miniTimerAlwaysOnTop);
  await focusGuard(true);
  passed('a pinned widget stays on top without stealing foreground focus');

  await switchTo(miniHandle);
  await click('mini-close');
  await switchTo(mainHandle);
  current = await until(async () => {
    const state = await snapshot();
    return !state.preferences.miniTimerEnabled ? state : null;
  });
  assert.equal(current.timer.status, 'running');
  assert.equal(current.timer.runId, started.timer.runId);
  const beforeHidden = current.timer.remainingMs;
  await sleep(1600);
  current = await snapshot();
  assert(current.timer.remainingMs < beforeHidden, 'Closing the widget must not pause the timer.');
  passed('closing the mini timer only hides it and preserves the running session');

  await click('mini-toggle');
  await switchTo(miniHandle);
  await until(async () => (await snapshot()).preferences.miniTimerEnabled);
  await invoke('update_preferences', { preferences: { ...(await snapshot()).preferences, theme: 'dark' } });
  await until(() => execute('return document.documentElement.dataset.theme === "dark" || document.documentElement.classList.contains("dark");'));
  await screenshot('mini-dark');
  await switchTo(mainHandle);
  await screenshot('main-dark');
  if (info.tray) {
    await click('window-close');
    await switchTo(miniHandle);
    const afterMainClose = await snapshot();
    assert.equal(afterMainClose.timer.status, 'running');
    assert.equal(afterMainClose.timer.runId, started.timer.runId);
    passed('closing the main window to the tray leaves the shared mini timer running');
    await invoke('show_main_window');
    await switchTo(mainHandle);
  }
  await invoke('timer_action', { action: { type: 'pause' } });
  current = await snapshot();
  assert.equal(current.timer.status, 'paused');
  assert.equal(current.persistenceError, false);
  passed('native state checkpoints successfully after multi-window actions');
  report.passed = true;
} catch (error) {
  report.error = String(error?.stack ?? error);
  if (session) {
    await screenshot('failure').catch(() => {});
    await execute('return {url: location.href, text: document.body.innerText.slice(0,8000)};')
      .then(value => writeFileSync(path.join(evidence, 'failure-ui.json'), JSON.stringify(value, null, 2)))
      .catch(() => {});
  }
  throw error;
} finally {
  writeFileSync(path.join(evidence, 'native-smoke.json'), JSON.stringify(report, null, 2) + '\n');
  if (session) {
    await invoke('quit_app').catch(() => {});
    await request('DELETE', '/session/' + session).catch(() => {});
  }
  driver.stdout.unpipe(log);
  driver.stderr.unpipe(log);
  driver.kill();
  log.end();
  // A failed WebDriver handshake can leave its launched child running.
  // Limit cleanup to this one test executable on this disposable CI runner.
  const encoded = Buffer.from(
    '$focusPath = ' + "'" + app.replaceAll("'", "''") + "'" + '; Get-Process -Name focus-timer -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $focusPath } | Stop-Process -Force',
    'utf16le',
  ).toString('base64');
  try { execFileSync('powershell.exe', ['-NoProfile', '-EncodedCommand', encoded], { stdio: 'ignore' }); } catch {}
}
