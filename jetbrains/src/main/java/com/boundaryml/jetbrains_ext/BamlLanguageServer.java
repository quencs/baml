package com.boundaryml.jetbrains_ext;

import com.intellij.execution.configurations.GeneralCommandLine;
import com.redhat.devtools.lsp4ij.server.OSProcessStreamConnectionProvider;

public class BamlLanguageServer extends OSProcessStreamConnectionProvider {

    public BamlLanguageServer() {
        System.out.printf("baml language server started via osprocess\n");
        GeneralCommandLine commandLine = new GeneralCommandLine("/Users/sam/baml3/engine/target/debug/baml-cli", "lsp");
        super.setCommandLine(commandLine);
    }
}
