package com.boundaryml.jetbrains_ext

import com.intellij.openapi.diagnostic.thisLogger
import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.ui.content.ContentFactory
import com.intellij.ui.jcef.JBCefBrowser
import java.awt.BorderLayout
import java.awt.Container
import javax.swing.JPanel

class BamlToolWindowFactory : ToolWindowFactory {

    init {
        thisLogger().warn("Don't forget to remove all non-needed sample code files with their corresponding registration entries in `plugin.xml`.")
    }

    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val browser = JBCefBrowser()
        val panel   = JPanel(BorderLayout()).apply {
            add(browser.component, BorderLayout.CENTER)
        }

        //-- subscribe once per tool-window instance
        val connection = project.messageBus.connect(toolWindow.disposable)
        connection.subscribe(BamlPortService.TOPIC, BamlPortService.Listener { port ->
            browser.loadURL("http://localhost:$port/")
        })

        // if LS started before the tool window was opened
        project.getService(BamlPortService::class.java).port?.let { port ->
            browser.loadURL("http://localhost:$port/")
        }

        val content = ContentFactory.getInstance().createContent(panel, null, false)
        toolWindow.contentManager.addContent(content)
    }

    override fun shouldBeAvailable(project: Project) = true

    class BamlToolWindow(toolWindow: ToolWindow) {

        private val browser = JBCefBrowser()

        init {
            browser.loadHTML(
                """
                <!DOCTYPE html>
                <html lang="en">
                  <head><meta charset="UTF-8"/><title>Loading…</title></head>
                  <body><div id="root">Loading BAML applications…</div></body>
                </html>
                """.trimIndent()
            )

        }

        fun getContent(): JPanel {
            return JPanel(BorderLayout()).apply {
                add(browser.component, BorderLayout.CENTER)
            }
        }

        // This approach doesn't work.
        // We need to follow instructions here and implement resource loaders
        // https://plugins.jetbrains.com/docs/intellij/embedded-browser-jcef.html#loading-resources-from-plugin-distribution
        private fun loadHtmlFromResources(): String {
            // Load the HTML file from the `resources/web-panel/index.html`
            val stylesUri = javaClass.getResource("/web-panel/index.css")!!.toURI()
            val scriptUri = javaClass.getResource("/web-panel/index.js")!!.toURI()

            val htmlContent = """
                          <!DOCTYPE html>
                          <html lang="en">
                            <head>
                              <meta charset="UTF-8" />
                              <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                              <title>Hello World</title>
                            </head>
                            <body>
                              <div id="root">Waiting for react (unimplemented)</div>
                            </body>
                          </html>
            """.trimIndent()

            return htmlContent;
        }
    }
}