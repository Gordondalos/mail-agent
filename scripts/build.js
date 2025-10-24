#!/usr/bin/env node
const { spawnSync } = require('child_process');

function run(cmd, args, opts) {
  const res = spawnSync(cmd, args, { stdio: 'inherit', shell: true, ...opts });
  if (res.status !== 0) process.exit(res.status || 1);
}

// 1) Build Angular app for production
run(process.platform === 'win32' ? 'npm.cmd' : 'npm', ['run', 'build', '--', '--configuration', 'production'], { cwd: 'frontend' });

// 2) Build Tauri app and copy artifacts to release/
if (process.platform === 'win32') {
  run('pwsh', ['-NoProfile', '-File', '.\\scripts\\setup.ps1', '-Auto', '-Build']);
} else {
  run('bash', ['scripts/setup.sh', '--auto', '--build']);
}

console.log('\nRelease artifacts should be in ./release');

