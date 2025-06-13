package com.boundaryml.jetbrains_ext

import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.LanguageServerFactory
import com.redhat.devtools.lsp4ij.client.features.LSPClientFeatures
import com.redhat.devtools.lsp4ij.server.StreamConnectionProvider

class BamlLanguageServerFactory : LanguageServerFactory {

    override fun createConnectionProvider(project: Project): StreamConnectionProvider {
        return BamlLanguageServer()
    }

    override fun createClientFeatures(): LSPClientFeatures {
        val features = LSPClientFeatures()
        features.setServerInstaller(BamlLanguageServerInstaller()) // customize language server installer
        return features
    }

//    // If you need to provide client specific features
//    override fun createLanguageClient(project: Project): LanguageClientImpl {
//        return BamlLanguageServerFactory(project)
//    }

//    // If you need to expose a custom server API
//    override fun getServerInterface(): Class<out LanguageServer> {
//        return MyCustomServerAPI::class.java
//    }
}
