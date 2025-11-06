"use client";
// import * as vscode from 'vscode'

import { vscode } from "@baml/playground-common";
import {
  diagnosticsAtom,
  wasmAtom,
  selectedFunctionAtom,
  orchIndexAtom,
} from "@baml/playground-common";
import { useDebounceCallback } from "@react-hook/debounce";
import { atom, useAtom, useAtomValue, useSetAtom } from "jotai";
import { atomWithStorage } from "jotai/utils";
import { useEffect } from "react";
import { vscodeLocalStorageStore } from "./JotaiProvider";
import { type BamlConfigAtom, bamlConfig } from "./bamlConfig";
import { VscodeToWebviewCommand } from "./vscode-to-webview-rpc";
import { useBAMLSDK } from "../sdk/provider";
import { handleIDEMessage, handleLSPMessage } from "./message-handlers";

export const hasClosedEnvVarsDialogAtom = atomWithStorage<boolean>(
  "has-closed-env-vars-dialog",
  false,
  vscodeLocalStorageStore,
);
export const bamlCliVersionAtom = atom<string | null>(null);

export const showIntroToChecksDialogAtom = atom(false);
export const hasClosedIntroToChecksDialogAtom = atomWithStorage<boolean>(
  "has-closed-intro-to-checks-dialog",
  false,
  vscodeLocalStorageStore,
);

export const versionAtom = atom((get) => {
  const wasm = get(wasmAtom);

  if (wasm === undefined) {
    return "Loading...";
  }

  return wasm.version();
});

export const numErrorsAtom = atom((get) => {
  const errors = get(diagnosticsAtom);

  const warningCount = errors.filter((e: any) => e.type === "warning").length;

  return { errors: errors.length - warningCount, warnings: warningCount };
});

export const isConnectedAtom = atom(true);

export const EventListener: React.FC = () => {
  // Get SDK instance
  const sdk = useBAMLSDK();
  const isVSCodeWebview = vscode.isVscode();

  // Non-core state atoms (not managed by SDK)
  const setBamlCliVersion = useSetAtom(bamlCliVersionAtom);
  const setBamlConfig = useSetAtom(bamlConfig);

  // Platform quirk: Debounce file updates to prevent excessive WASM recompilation
  const debouncedUpdateFiles = useDebounceCallback(
    (files: Record<string, string>) => {
      console.debug('[EventListener] Debounced file update');
      sdk.files.update(files);
    },
    50,
    true // Leading edge
  );

  const [selectedFunc] = useAtom(selectedFunctionAtom);
  const wasm = useAtomValue(wasmAtom);
  useEffect(() => {
    if (wasm) {
      console.debug("wasm ready!");
      try {
        vscode.markInitialized();
      } catch (e) {
        console.error("Error marking initialized", e);
      }
    }
  }, [wasm]);

  const setOrchestratorIndex = useSetAtom(orchIndexAtom);

  useEffect(() => {
    if (selectedFunc) {
      // todo: maybe we use a derived atom to reset it. But for now this useeffect works.
      setOrchestratorIndex(0);
    }
  }, [selectedFunc]);

  useEffect(() => {
    // Only open websocket if not in VSCode webview
    if (isVSCodeWebview) {
      return;
    }

    const scheme = window.location.protocol === "https:" ? "wss" : "ws";
    const ws = new WebSocket(`${scheme}://${window.location.host}/ws`);

    ws.onmessage = (e) => {
      try {
        const payload = JSON.parse(e.data);
        window.postMessage(payload, "*");
      } catch (err) {
        console.error("invalid WS payload", err);
      }
    };

    return () => ws.close();
  }, [isVSCodeWebview]);

  // Main message handler - routes IDE/LSP messages to SDK methods
  useEffect(() => {
    const handler = async (event: MessageEvent<VscodeToWebviewCommand>) => {
      const { source, payload } = event.data;
      console.debug('[EventListener] Handling command', { source, payload });

      try {
        switch (source) {
          case 'ide_message':
            // Handle IDE messages via SDK
            await handleIDEMessage(sdk, payload, setBamlCliVersion, setBamlConfig);
            break;

          case 'lsp_message':
            // Handle LSP messages via SDK
            await handleLSPMessage(sdk, payload, debouncedUpdateFiles, setBamlConfig);
            break;

          default:
            console.warn('[EventListener] Unknown message source:', source);
        }
      } catch (error) {
        // Log error but don't crash EventListener - continue processing other messages
        console.error('[EventListener] Error handling message:', {
          source,
          payload,
          error,
        });
      }
    };

    window.addEventListener('message', handler);

    return () => window.removeEventListener('message', handler);
  }, [sdk, debouncedUpdateFiles, setBamlCliVersion, setBamlConfig]);

  return (
    <>
      {/* EventListener handles background events - no UI needed since StatusBar handles display */}
    </>
  );
};
