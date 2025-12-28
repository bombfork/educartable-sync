import { spawn, spawnSync } from 'child_process'
import { env } from 'process'

export const config = {
  specs: ['./test/specs/**/*.js'],
  exclude: [],
  maxInstances: 1,
  hostname: '127.0.0.1',
  port: 4444,
  capabilities: [
    {
      maxInstances: 1,
      'tauri:options': {
        application: '../src-tauri/target/debug/educartable-sync',
      },
    },
  ],
  reporters: ['spec'],
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },

  // Ensure we are running x64 architecture on Apple Silicon macOS
  onWorkerStart: () => {
    if (process.platform === 'darwin' && process.arch === 'arm64') {
      env.TAURI_ARCH = 'x86_64'
    }
  },

  // Build the Tauri app in debug mode before running tests
  onPrepare: () => {
    const result = spawnSync('cargo', ['build', '--manifest-path=../src-tauri/Cargo.toml'])
    if (result.status !== 0) {
      console.error('Failed to build Tauri application')
      throw new Error('Build failed')
    }
  },

  // Start tauri-driver before the session
  beforeSession: () => {
    globalThis.tauriDriver = spawn('tauri-driver', [], {
      stdio: [null, process.stdout, process.stderr],
    })
    // Give tauri-driver time to start
    return new Promise((resolve) => setTimeout(resolve, 2000))
  },

  // Stop tauri-driver after the session
  afterSession: () => {
    if (globalThis.tauriDriver) {
      globalThis.tauriDriver.kill()
    }
  },
}
