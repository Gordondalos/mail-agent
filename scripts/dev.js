#!/usr/bin/env node
const { spawn } = require('child_process');

function run(cmd, args, opts) {
  const p = spawn(cmd, args, { stdio: 'inherit', shell: true, ...opts });
  p.on('exit', (code) => {
    if (code !== 0) process.exitCode = code;
  });
  return p;
}

// Start Angular dev server
const ng = run(process.platform === 'win32' ? 'npm.cmd' : 'npm', ['start'], { cwd: 'frontend' });

// Start Tauri dev in parallel
const tauri = run('cargo', ['tauri', 'dev'], { cwd: 'src-tauri' });

function shutdown() {
  if (ng && !ng.killed) try { if (process.platform === 'win32') run('taskkill', ['/PID', ng.pid, '/T', '/F']); else ng.kill('SIGINT'); } catch {}
  if (tauri && !tauri.killed) try { if (process.platform === 'win32') run('taskkill', ['/PID', tauri.pid, '/T', '/F']); else tauri.kill('SIGINT'); } catch {}
}

process.on('SIGINT', shutdown);
process.on('SIGTERM', shutdown);

