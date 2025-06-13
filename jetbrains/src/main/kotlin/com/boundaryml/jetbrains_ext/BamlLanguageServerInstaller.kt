package com.boundaryml.jetbrains_ext

import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.progress.ProgressManager
import com.redhat.devtools.lsp4ij.installation.LanguageServerInstallerBase
import org.apache.commons.compress.archivers.tar.TarArchiveEntry
import org.apache.commons.compress.archivers.tar.TarArchiveInputStream
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream
import org.jetbrains.annotations.NotNull
import java.io.File
import java.io.IOException
import java.io.InputStream
import java.io.OutputStream
import java.net.HttpURLConnection
import java.net.URL
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardOpenOption

class BamlLanguageServerInstaller : LanguageServerInstallerBase() {

    private val BAML_CACHE_DIR: Path = Path.of(System.getProperty("user.home"), ".baml")
    private val BREADCRUMB_PATH: Path = Path.of(System.getProperty("user.home"), ".baml", "baml-cli-0.89.0-from-github")
    private val TAR_GZ_PATH: Path = Path.of(System.getProperty("user.home"), ".baml", "baml-cli-0.89.0-from-github.tar.gz")

    override fun checkServerInstalled(indicator: ProgressIndicator): Boolean {
        super.progress("Checking if the language server is installed...", indicator)
        ProgressManager.checkCanceled()
        return Files.exists(BREADCRUMB_PATH)
    }

    override fun install(indicator: ProgressIndicator) {
        super.progress("Downloading server components...", 0.25, indicator)
        Thread.sleep(1000)
        ProgressManager.checkCanceled()

        super.progress("Configuring server...", 0.5, indicator)
        Thread.sleep(1000)
        ProgressManager.checkCanceled()

        super.progress("Finalizing installation...", 0.75, indicator)
        Thread.sleep(1000)
        ProgressManager.checkCanceled()

        Files.createDirectories(BREADCRUMB_PATH.parent)
        val urlStr = "https://github.com/BoundaryML/baml/releases/download/0.89.0/baml-cli-0.89.0-aarch64-apple-darwin.tar.gz"

        this.downloadFile(urlStr, TAR_GZ_PATH)
        this.extractTarGz(TAR_GZ_PATH, BAML_CACHE_DIR)

        super.progress("Installation complete!", 1.0, indicator)
        Thread.sleep(1000)
        ProgressManager.checkCanceled()
    }

    private fun downloadFile(urlStr: String, outputPath: Path) {
        val url = URL(urlStr)
        val conn = url.openConnection() as HttpURLConnection
        conn.instanceFollowRedirects = true
        conn.connect()

        conn.inputStream.use { input ->
            Files.newOutputStream(outputPath).use { output ->
                input.transferTo(output)
            }
        }
        println("Downloaded to $outputPath")
    }

    private fun extractTarGz(tarGzPath: Path, destDir: Path) {
        Files.newInputStream(tarGzPath).use { fileIn ->
            GzipCompressorInputStream(fileIn).use { gzipIn ->
                TarArchiveInputStream(gzipIn).use { tarIn ->
                    var entry: TarArchiveEntry? = tarIn.nextTarEntry
                    while (entry != null) {
                        val outPath = destDir.resolve(entry.name)
                        if (entry.isDirectory) {
                            Files.createDirectories(outPath)
                        } else {
                            Files.createDirectories(outPath.parent)
                            Files.newOutputStream(outPath).use { out ->
                                tarIn.transferTo(out)
                            }
                        }
                        entry = tarIn.nextTarEntry
                    }
                }
            }
        }
        println("Extraction complete.")
    }
}
