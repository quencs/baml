package com.boundaryml.jetbrains_ext

import com.intellij.execution.configurations.GeneralCommandLine
import com.redhat.devtools.lsp4ij.server.OSProcessStreamConnectionProvider

class BamlLanguageServer : OSProcessStreamConnectionProvider() {

    init {
        println("baml language server started via osprocess")
        val commandLine = GeneralCommandLine("/Users/sam/baml3/engine/target/debug/baml-cli", "lsp")
        super.setCommandLine(commandLine)
    }
}
