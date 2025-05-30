package com.boundaryml.jetbrains_ext;

import com.intellij.openapi.project.Project;
import com.redhat.devtools.lsp4ij.LanguageServerFactory;
import com.redhat.devtools.lsp4ij.client.LanguageClientImpl;
import com.redhat.devtools.lsp4ij.server.StreamConnectionProvider;
import org.jetbrains.annotations.NotNull;

public class BamlLanguageServerFactory implements LanguageServerFactory {

    @Override
    public @NotNull StreamConnectionProvider createConnectionProvider(@NotNull Project project) {
        return new BamlLanguageServer();
    }
//
//    @Override // If you need to provide client specific features
//    public @NotNull LanguageClientImpl createLanguageClient(@NotNull Project project) {
//        return new BamlLanguageServerFactory(project);
//    }

//    @Override // If you need to expose a custom server API
//    public @NotNull Class<? extends LanguageServer> getServerInterface() {
//        return MyCustomServerAPI.class;
//    }

}
