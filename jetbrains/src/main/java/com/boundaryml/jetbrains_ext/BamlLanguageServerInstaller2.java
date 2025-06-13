package com.boundaryml.jetbrains_ext;

import com.intellij.openapi.progress.ProgressIndicator;
import com.intellij.openapi.progress.ProgressManager;
import com.redhat.devtools.lsp4ij.installation.LanguageServerInstallerBase;
import org.apache.commons.compress.archivers.tar.TarArchiveEntry;
import org.apache.commons.compress.archivers.tar.TarArchiveInputStream;
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream;
import org.jetbrains.annotations.NotNull;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.Path;

public class BamlLanguageServerInstaller2 extends LanguageServerInstallerBase {

    private static final Path BAML_CACHE_DIR = Path.of(System.getProperty("user.home"), ".baml");
    private static final Path BREADCRUMB_PATH = Path.of(System.getProperty("user.home"), ".baml", "baml-cli-0.89.0-from-github");
    private static final Path TAR_GZ_PATH = Path.of(System.getProperty("user.home"), ".baml", "baml-cli-0.89.0-from-github.tar.gz");

    @Override
    protected boolean checkServerInstalled(@NotNull ProgressIndicator indicator) {
        progress("Checking if the language server is installed...", indicator);
        ProgressManager.checkCanceled();
        return Files.exists(BREADCRUMB_PATH);
    }

    @Override
    protected void install(@NotNull ProgressIndicator indicator) throws Exception {
        progress("Downloading server components...", 0.25, indicator);
        Thread.sleep(1000);
        ProgressManager.checkCanceled();

        progress("Configuring server...", 0.5, indicator);
        Thread.sleep(1000);
        ProgressManager.checkCanceled();

        progress("Finalizing installation...", 0.75, indicator);
        Thread.sleep(1000);
        ProgressManager.checkCanceled();

        // Simulate writing a breadcrumb file to mark successful installation
        Files.createDirectories(BREADCRUMB_PATH.getParent());
        String urlStr = "https://github.com/BoundaryML/baml/releases/download/0.89.0/baml-cli-0.89.0-aarch64-apple-darwin.tar.gz";
        // Step 1: Download the file
        downloadFile(urlStr, TAR_GZ_PATH);
        // Step 2: Extract the archive
        extractTarGz(TAR_GZ_PATH, BAML_CACHE_DIR);
        // TODO: make extracted files executable
//        Files.writeString(BREADCRUMB_PATH, "baml-cli installed", StandardOpenOption.CREATE, StandardOpenOption.APPEND);

        progress("Installation complete!", 1.0, indicator);
        Thread.sleep(1000);
        ProgressManager.checkCanceled();
    }




    private static void downloadFile(String urlStr, Path outputPath) throws IOException {
        HttpURLConnection conn = (HttpURLConnection) new URL(urlStr).openConnection();
        conn.setInstanceFollowRedirects(true);
        conn.connect();

        try (InputStream in = conn.getInputStream();
             OutputStream out = Files.newOutputStream(outputPath)) {
            in.transferTo(out);
        }
        System.out.println("Downloaded to " + outputPath);
    }

    private static void extractTarGz(Path tarGzPath, Path destDir) throws IOException {
        try (InputStream fileIn = Files.newInputStream(tarGzPath);
             GzipCompressorInputStream gzipIn = new GzipCompressorInputStream(fileIn);
             TarArchiveInputStream tarIn = new TarArchiveInputStream(gzipIn)) {

            TarArchiveEntry entry;
            while ((entry = tarIn.getNextTarEntry()) != null) {
                Path outPath = destDir.resolve(entry.getName());
                if (entry.isDirectory()) {
                    Files.createDirectories(outPath);
                } else {
                    Files.createDirectories(outPath.getParent());
                    try (OutputStream out = Files.newOutputStream(outPath)) {
                        tarIn.transferTo(out);
                    }
                }
            }
        }
        System.out.println("Extraction complete.");
    }
}
