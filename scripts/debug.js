#!/usr/bin/env node
const { spawn } = require('child_process');

function run(cmd, args, opts) {
  const p = spawn(cmd, args, { stdio: 'inherit', shell: true, ...opts });
  p.on('exit', (code) => {
    if (code !== 0) process.exitCode = code;
  });
  return p;
}

const env = {
  ...process.env,
  RUST_LOG: process.env.RUST_LOG || 'reqwest=debug,oauth2=debug,gmail_tray_notifier=debug,tauri=info',
  RUST_BACKTRACE: process.env.RUST_BACKTRACE || '1',
};

// Start Angular dev server
const ng = run(process.platform === 'win32' ? 'npm.cmd' : 'npm', ['start'], { cwd: 'frontend', env });

// Start Tauri dev in parallel with verbose logs
const tauri = run('cargo', ['tauri', 'dev'], { cwd: 'src-tauri', env });

function shutdown() {
  if (ng && !ng.killed) try { if (process.platform === 'win32') run('taskkill', ['/PID', ng.pid, '/T', '/F']); else ng.kill('SIGINT'); } catch {}
  if (tauri && !tauri.killed) try { if (process.platform === 'win32') run('taskkill', ['/PID', tauri.pid, '/T', '/F']); else tauri.kill('SIGINT'); } catch {}
}

process.on('SIGINT', shutdown);
process.on('SIGTERM', shutdown);

