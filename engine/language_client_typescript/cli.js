#!/usr/bin/env node

import { invoke_runtime_cli } from './native.js'

if (!process.env.BAML_LOG) {
  process.env.BAML_LOG = 'info'
}

invoke_runtime_cli(process.argv.slice(1))
