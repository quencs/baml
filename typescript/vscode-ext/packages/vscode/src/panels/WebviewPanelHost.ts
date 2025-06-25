import type { StringSpan } from '@baml/common'
import { fromIni } from '@aws-sdk/credential-providers' // ES6 import
import { type Disposable, Uri, ViewColumn, type Webview, type WebviewPanel, window, workspace } from 'vscode'
import * as vscode from 'vscode'
import { getNonce } from '../utils/getNonce'
import { getUri } from '../utils/getUri'
import {
  EchoResponse,
  GetBamlSrcResponse,
  LoadEnvRequest,
  GetPlaygroundPortResponse,
  GetVSCodeSettingsResponse,
  GetWebviewUriResponse,
  WebviewToVscodeRpc,
  encodeBuffer,
  LoadEnvResponse,
} from '../vscode-rpc'

import { type Config, adjectives, animals, colors, uniqueNamesGenerator } from 'unique-names-generator'
import { URI } from 'vscode-uri'
import { getCurrentOpenedFile } from '../helpers/get-open-file'
import { bamlConfig, requestDiagnostics, getPlaygroundPort } from '../plugins/language-server-client'
import TelemetryReporter from '../telemetryReporter'
import { exec, fork } from 'child_process'
import { promisify } from 'util'
import { dirname, join } from 'path'
import * as dotenv from 'dotenv'
import * as fs from 'fs'
import { AwsCredentialIdentity } from '@smithy/types'
import { refreshBamlConfigSingleton } from '../plugins/language-server-client/bamlConfig'
import { GoogleAuth } from 'google-auth-library'
// import { CredentialsProviderError } from '@aws-sdk/credential-providers'
const customConfig: Config = {
  dictionaries: [adjectives, colors, animals],
  separator: '_',
  length: 2,
}

export const openPlaygroundConfig: { lastOpenedFunction: null | string } = {
  lastOpenedFunction: null,
}

const execAsync = promisify(exec)
const readFileAsync = promisify(fs.readFile)

/**
 * This class manages the state and behavior of HelloWorld webview panels.
 *
 * It contains all the data and methods for:
 *
 * - Creating and rendering HelloWorld webview panels
 * - Properly cleaning up and disposing of webview resources when the panel is closed
 * - Setting the HTML (and by proxy CSS/JavaScript) content of the webview panel
 * - Setting message listeners so data can be passed between the webview and extension
 */
export class WebviewPanelHost {
  public static currentPanel: WebviewPanelHost | undefined
  private readonly _panel: WebviewPanel
  private _disposables: Disposable[] = []
  private _port: () => number

  /**
   * The WebPanelView class private constructor (called only from the render method).
   *
   * @param panel A reference to the webview panel
   * @param extensionUri The URI of the directory containing the extension
   */
  private constructor(
    panel: WebviewPanel,
    extensionUri: Uri,
    portLoader: () => number,
    private reporter?: TelemetryReporter,
  ) {
    this._panel = panel
    this._port = portLoader

    // Set an event listener to listen for when the panel is disposed (i.e. when the user closes
    // the panel or when the panel is closed programmatically)
    this._panel.onDidDispose(() => this.dispose(), null, this._disposables)

    const playgroundPort = 3030
    if (playgroundPort) {
      // Add 3 second delay for debugging
      setTimeout(() => {
        // Use the same CSP approach as SimpleBrowser
        const nonce = getNonce()

        this._panel.webview.html = `<!DOCTYPE html>
          <html>
          <head>
              <meta http-equiv="Content-type" content="text/html;charset=UTF-8">
              
              <meta http-equiv="Content-Security-Policy" content="
                  default-src 'none';
                  font-src data:;
                  style-src ${this._panel.webview.cspSource} 'unsafe-inline';
                  script-src 'nonce-${nonce}';
                  frame-src *;
                  ">
              
              <style>
                  body, html {
                      margin: 0;
                      padding: 0;
                      width: 100%;
                      height: 100vh;
                      overflow: hidden;
                      background: #1e1e1e;
                      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                  }
                  .header {
                      background: #252526;
                      color: #cccccc;
                      padding: 8px 16px;
                      border-bottom: 1px solid #3e3e42;
                      font-size: 12px;
                      display: flex;
                      align-items: center;
                      justify-content: space-between;
                  }
                  .header-title {
                      display: flex;
                      align-items: center;
                      gap: 8px;
                  }
                  .status-indicator {
                      width: 8px;
                      height: 8px;
                      border-radius: 50%;
                      background: #16c60c;
                      animation: pulse 2s infinite;
                  }
                  @keyframes pulse {
                      0% { opacity: 1; }
                      50% { opacity: 0.5; }
                      100% { opacity: 1; }
                  }
                  .iframe-container {
                      height: calc(100vh - 33px);
                      position: relative;
                  }
                  iframe {
                      width: 100%;
                      height: 100%;
                      border: none;
                      display: block;
                  }
                  .loading {
                      position: absolute;
                      top: 50%;
                      left: 50%;
                      transform: translate(-50%, -50%);
                      color: #cccccc;
                      text-align: center;
                      transition: opacity 0.3s ease;
                  }
                  .hidden {
                      opacity: 0;
                      pointer-events: none;
                  }
                  .port-info {
                      background: #3e3e42;
                      padding: 2px 8px;
                      border-radius: 3px;
                      font-size: 11px;
                      opacity: 0.8;
                  }
              </style>
          </head>
          <body>
              <div class="header">
                  <div class="header-title">
                      <div class="status-indicator"></div>
                      <span>BAML Playground (LSP Mode)</span>
                  </div>
                  <div class="port-info">Port: ${playgroundPort}</div>
              </div>
              <div class="iframe-container">
                  <div class="loading" id="loading">
                      <p>Connecting to Language Server Playground...</p>
                      <p style="font-size: 12px; opacity: 0.7;">http://localhost:${playgroundPort}</p>
                  </div>
                  <iframe 
                      id="playground"
                      sandbox="allow-scripts allow-forms allow-same-origin allow-downloads"
                      src="http://localhost:${playgroundPort}/"
                  ></iframe>
              </div>
              
              <script nonce="${nonce}">
                  const iframe = document.getElementById('playground');
                  const loading = document.getElementById('loading');
                  
                  // Hide loading indicator when iframe loads
                  iframe.addEventListener('load', () => {
                      loading.classList.add('hidden');
                  });
                  
                  // Handle navigation attempts (optional)
                  iframe.addEventListener('error', () => {
                      loading.innerHTML = '<p style="color: #f48771;">Failed to connect to LSP playground server</p><p style="font-size: 12px;">Make sure the language server is running on port ${playgroundPort}</p>';
                      loading.classList.remove('hidden');
                  });
              </script>
          </body>
          </html>`
      }, 3000) // 3 second delay for debugging

      // Show loading message immediately
      this._panel.webview.html = `<!DOCTYPE html>
          <html>
          <head>
              <style>
                  body {
                      background: #1e1e1e;
                      color: #cccccc;
                      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                      display: flex;
                      align-items: center;
                      justify-content: center;
                      height: 100vh;
                      margin: 0;
                  }
              </style>
          </head>
          <body>
              <div>
                  <h2>🔧 LSP-Based BAML Playground</h2>
                  <p>Starting Language Server Protocol connection...</p>
                  <p style="font-size: 14px; opacity: 0.7;">Waiting 3 seconds for server to initialize (debug delay)</p>
                  <p style="font-size: 12px; opacity: 0.5;">Target port: ${playgroundPort}</p>
              </div>
          </body>
          </html>`
    } else {
      this._panel.webview.html = this._getWebviewContent(this._panel.webview, extensionUri)
    }

    // Set an event listener to listen for messages passed from the webview context
    this._setWebviewMessageListener(this._panel.webview)
  }

  /**
   * Renders the current webview panel if it exists otherwise a new webview panel
   * will be created and displayed.
   *
   * @param extensionUri The URI of the directory containing the extension.
   */
  public static render(extensionUri: Uri, portLoader: () => number, reporter: TelemetryReporter) {
    if (WebviewPanelHost.currentPanel) {
      // If the webview panel already exists reveal it
      WebviewPanelHost.currentPanel._panel.reveal(ViewColumn.Beside)
    } else {
      // If a webview panel does not already exist create and show a new one
      const panel = window.createWebviewPanel(
        // Panel view type
        'showHelloWorld',
        // Panel title
        'BAML Playground',
        // The editor column the panel should be displayed in
        // process.env.VSCODE_DEBUG_MODE === 'true' ? ViewColumn.Two : ViewColumn.Beside,
        { viewColumn: ViewColumn.Beside, preserveFocus: true },

        // Extra panel configurations
        {
          // Enable JavaScript in the webview
          enableScripts: true,

          // Restrict the webview to only load resources from the `out` and `web-panel/dist` directories
          localResourceRoots: [
            ...(vscode.workspace.workspaceFolders ?? []).map((f) => f.uri),
            Uri.joinPath(extensionUri, 'out'),
            Uri.joinPath(extensionUri, 'web-panel/dist'),
          ],
          retainContextWhenHidden: true,
          enableCommandUris: true,
        },
      )

      WebviewPanelHost.currentPanel = new WebviewPanelHost(panel, extensionUri, portLoader, reporter)
    }
  }

  public postMessage<T>(command: string, content: T) {
    this._panel.webview.postMessage({ command: command, content })
    this.reporter?.sendTelemetryEvent({
      event: `baml.webview.${command}`,
      properties: {},
    })
  }

  /**
   * Cleans up and disposes of webview resources when the webview panel is closed.
   */
  public dispose() {
    WebviewPanelHost.currentPanel = undefined

    // Dispose of the current webview panel
    this._panel.dispose()

    const config = workspace.getConfiguration()
    config.update('baml.bamlPanelOpen', false, true)

    // Dispose of all disposables (i.e. commands) for the current webview panel
    while (this._disposables.length) {
      const disposable = this._disposables.pop()
      if (disposable) {
        disposable.dispose()
      }
    }
  }

  /**
   * Defines and returns the HTML that should be rendered within the webview panel.
   *
   * @remarks This is also the place where references to the React webview dist files
   * are created and inserted into the webview HTML.
   *
   * @param webview A reference to the extension webview
   * @param extensionUri The URI of the directory containing the extension
   * @returns A template string literal containing the HTML that should be
   * rendered within the webview panel
   */
  private _getWebviewContent(webview: Webview, extensionUri: Uri) {
    // The CSS file from the React dist output
    const stylesUri = getUri(webview, extensionUri, ['web-panel', 'dist', 'assets', 'index.css'])
    // The JS file from the React dist output
    const scriptUri = getUri(webview, extensionUri, ['web-panel', 'dist', 'assets', 'index.js'])

    const nonce = getNonce()

    // Tip: Install the es6-string-html VS Code extension to enable code highlighting below
    return /*html*/ `
          <!DOCTYPE html>
          <html lang="en">
            <head>
              <meta charset="UTF-8" />
              <meta name="viewport" content="width=device-width, initial-scale=1.0" />
              <link rel="stylesheet" type="text/css" href="${stylesUri}">
              <title>Hello World</title>
            </head>
            <body>
              <div id="root">Waiting for react: ${scriptUri}</div>
              <script type="module" nonce="${nonce}" src="${scriptUri}"></script>
            </body>
          </html>`
  }

  /**
   * Sets up an event listener to listen for messages passed from the webview context and
   * executes code based on the message that is recieved.
   *
   * @param webview A reference to the extension webview
   * @param context A reference to the extension context
   */
  private _setWebviewMessageListener(webview: Webview) {
    console.log('_setWebviewMessageListener')

    const addProject = async () => {
      await requestDiagnostics()
      console.log('last opened func', openPlaygroundConfig.lastOpenedFunction)
      this.postMessage('select_function', {
        root_path: 'default',
        function_name: openPlaygroundConfig.lastOpenedFunction,
      })
      this.postMessage('baml_cli_version', bamlConfig.cliVersion)
      this.postMessage('baml_settings_updated', bamlConfig)
    }

    vscode.workspace.onDidChangeConfiguration((event) => {
      console.log('*** CLIENT DID CHANGE CONFIGURATION', event)
      if (event.affectsConfiguration('baml')) {
        setTimeout(() => {
          this.postMessage('baml_settings_updated', refreshBamlConfigSingleton())
        }, 1000)
      }
    })

    webview.onDidReceiveMessage(
      async (
        message:
          | {
              command: 'get_port' | 'add_project' | 'cancelTestRun' | 'removeTest'
            }
          | {
              command: 'set_flashing_regions'
              content: {
                spans: {
                  file_path: string
                  start_line: number
                  start_char: number
                  end_line: number
                  end_char: number
                }[]
              }
            }
          | {
              command: 'jumpToFile'
              span: StringSpan
            }
          | {
              command: 'telemetry'
              meta: {
                action: string
                data: Record<string, unknown>
              }
            }
          | {
              rpcId: number
              data: WebviewToVscodeRpc
            },
      ) => {
        console.log('DEBUG: webview message: ', message)
        if ('command' in message) {
          switch (message.command) {
            case 'add_project':
              console.log('webview add_project')
              addProject()

              return
            case 'jumpToFile': {
              try {
                console.log('jumpToFile', message.span)
                const span = message.span
                // span.source_file is a file:/// URI

                const uri = vscode.Uri.parse(span.source_file)
                await vscode.workspace.openTextDocument(uri).then((doc) => {
                  const range = new vscode.Range(doc.positionAt(span.start), doc.positionAt(span.end))
                  vscode.window.showTextDocument(doc, { selection: range, viewColumn: ViewColumn.One })
                })
              } catch (e: any) {
                console.log(e)
              }
              return
            }
            case 'telemetry': {
              const { action, data } = message.meta
              this.reporter?.sendTelemetryEvent({
                event: `baml.webview.${action}`,
                properties: data,
              })
              return
            }
            case 'set_flashing_regions': {
              // Call the command handler with the spans
              console.log('WEBPANELVIEW set_flashing_regions', message.content.spans)
              vscode.commands.executeCommand('baml.setFlashingRegions', { content: message.content })
              return
            }
          }
        }

        if (!('rpcId' in message)) {
          return
        }

        // console.log('message from webview, after above handlers:', message)
        const vscodeMessage = message.data
        const vscodeCommand = vscodeMessage.vscodeCommand

        // TODO: implement error handling in our RPC framework
        switch (vscodeCommand) {
          case 'ECHO':
            const echoresp: EchoResponse = { message: vscodeMessage.message }
            // also respond with rpc id
            this._panel.webview.postMessage({ rpcId: message.rpcId, rpcMethod: vscodeCommand, data: echoresp })
            return
          case 'SET_PROXY_SETTINGS':
            const { proxyEnabled } = vscodeMessage
            const config = vscode.workspace.getConfiguration()
            config.update('baml.enablePlaygroundProxy', proxyEnabled, vscode.ConfigurationTarget.Workspace)
            return
          case 'GET_WEBVIEW_URI':
            console.log('GET_WEBVIEW_URI', vscodeMessage)
            // This is 1:1 with the contents of `image.file` in a test file, e.g. given `image { file baml_src://path/to-image.png }`,
            // relpath will be 'baml_src://path/to-image.png'
            const relpath = vscodeMessage.path

            // NB(san): this is a violation of the "never URI.parse rule"
            // (see https://www.notion.so/gloochat/windows-uri-treatment-fe87b22abebb4089945eb8cd1ad050ef)
            // but this relpath is already a file URI, it seems...
            const uriPath = Uri.parse(relpath)
            const uri = this._panel.webview.asWebviewUri(uriPath).toString()

            console.log('GET_WEBVIEW_URI', { vscodeMessage, uri, parsed: uriPath })
            let webviewUriResp: GetWebviewUriResponse = {
              uri,
            }
            if (vscodeMessage.contents) {
              try {
                const contents = await workspace.fs.readFile(uriPath)
                webviewUriResp = {
                  ...webviewUriResp,
                  contents: encodeBuffer(contents),
                }
              } catch (e) {
                webviewUriResp = {
                  ...webviewUriResp,
                  readError: `${e}`,
                }
              }
            }
            this._panel.webview.postMessage({ rpcId: message.rpcId, rpcMethod: vscodeCommand, data: webviewUriResp })
            return
          case 'GET_PLAYGROUND_PORT':
            console.log('GET_PLAYGROUND_PORT', this._port(), Date.now())
            const response: GetPlaygroundPortResponse = {
              port: this._port(),
            }
            this._panel.webview.postMessage({ rpcId: message.rpcId, rpcMethod: vscodeCommand, data: response })
            return
          case 'LOAD_AWS_CREDS':
            ;(async () => {
              try {
                const profile = vscodeMessage.profile
                const credentialProvider = fromIni({
                  profile: profile ?? undefined,
                })
                const awsCreds = await credentialProvider()
                this._panel.webview.postMessage({
                  rpcId: message.rpcId,
                  rpcMethod: vscodeCommand,
                  data: { ok: awsCreds },
                })
              } catch (error) {
                console.error('Error loading aws creds:', error)
                if (error instanceof Error) {
                  this._panel.webview.postMessage({
                    rpcId: message.rpcId,
                    rpcMethod: vscodeCommand,
                    data: {
                      error: {
                        ...error,
                        name: error.name,
                        message: error.message,
                      },
                    },
                  })
                } else {
                  this._panel.webview.postMessage({
                    rpcId: message.rpcId,
                    rpcMethod: vscodeCommand,
                    data: { error },
                  })
                }
              }
            })()
            return
          case 'LOAD_GCP_CREDS':
            ;(async () => {
              try {
                const auth = new GoogleAuth({
                  scopes: ['https://www.googleapis.com/auth/cloud-platform'],
                })

                const client = await auth.getClient()
                const projectId = await auth.getProjectId()

                const tokenResponse = await client.getAccessToken()

                this._panel.webview.postMessage({
                  rpcId: message.rpcId,
                  rpcMethod: vscodeCommand,
                  data: {
                    ok: {
                      accessToken: tokenResponse.token,
                      projectId,
                    },
                  },
                })
              } catch (error) {
                console.error('Error loading gcp creds:', error)
                if (error instanceof Error) {
                  this._panel.webview.postMessage({
                    rpcId: message.rpcId,
                    rpcMethod: vscodeCommand,
                    data: {
                      error: {
                        ...error,
                        name: error.name,
                        message: error.message,
                      },
                    },
                  })
                } else {
                  this._panel.webview.postMessage({
                    rpcId: message.rpcId,
                    rpcMethod: vscodeCommand,
                    data: { error },
                  })
                }
              }
            })()
            return
          case 'INITIALIZED': // when the playground is initialized and listening for file changes, we should resend all project files.
            // request diagnostics, which updates the runtime and triggers a new project files update.
            addProject()
            console.log('initialized webview')
            this._panel.webview.postMessage({ rpcId: message.rpcId, rpcMethod: vscodeCommand, data: { ack: true } })
            return
        }
      },
      undefined,
      this._disposables,
    )
  }
}
