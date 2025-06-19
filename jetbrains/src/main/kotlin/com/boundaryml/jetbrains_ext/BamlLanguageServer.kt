package com.boundaryml.jetbrains_ext

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.server.OSProcessStreamConnectionProvider
import java.nio.file.Path

class BamlLanguageServer(private val project: Project) : OSProcessStreamConnectionProvider() {

    init {
        // val commandLine = GeneralCommandLine(Path.of(System.getProperty("user.home"), ".baml/jetbrains", "baml-cli-0.89.0-aarch64-apple-darwin", "baml-cli").toString(), "lsp")
        val commandLine = GeneralCommandLine(
            Path.of(System.getProperty("user.home"),
                "/Documents/baml/engine/target/debug", "baml-cli").toString(), "lsp")
        super.setCommandLine(commandLine)
    }

}
