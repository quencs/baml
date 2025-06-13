package com.boundaryml.jetbrains_ext;

import com.intellij.openapi.project.Project;
import com.redhat.devtools.lsp4ij.LanguageServerFactory;
import com.redhat.devtools.lsp4ij.client.features.LSPClientFeatures;
import com.redhat.devtools.lsp4ij.server.StreamConnectionProvider;
import org.jetbrains.annotations.NotNull;

public class BamlLanguageServerFactory2 implements LanguageServerFactory {

    @Override
    public @NotNull StreamConnectionProvider createConnectionProvider(@NotNull Project project) {
        return new BamlLanguageServer2();
    }

    @Override
    public LSPClientFeatures createClientFeatures() {
//        return null;
        var features = new LSPClientFeatures();
        features.setServerInstaller(new BamlLanguageServerInstaller2()); // customize language server installer
        return features;
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


