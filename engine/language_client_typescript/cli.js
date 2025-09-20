#!/usr/bin/env node

import { invoke_runtime_cli } from './native.js'

if (!process.env.BAML_LOG) {
  process.env.BAML_LOG = 'info'
}

try {
  await invoke_runtime_cli(process.argv.slice(1))
} catch (error) {
  console.error(error)
  process.exitCode = 1
}
