/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

import * as vscode from "vscode";
import * as fs from "fs";

import {
  Disposable,
  Executable,
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient;
// type a = Parameters<>;

/// The server is looked for where the user said, then in the environment, then
/// in the extension itself, then on the PATH.
function serverCommand(context: vscode.ExtensionContext): string {
  const configured = vscode.workspace.getConfiguration("ppl").get<string>("serverPath")?.trim();
  if (configured) {
    return configured;
  }
  if (process.env.SERVER_PATH) {
    return process.env.SERVER_PATH;
  }
  const name = process.platform === "win32" ? "ppl-language-server.exe" : "ppl-language-server";
  const bundled = vscode.Uri.joinPath(context.extensionUri, "server", name).fsPath;
  if (fs.existsSync(bundled)) {
    // A vsix is a zip, and not every unpacker keeps the executable bit.
    if (process.platform !== "win32") {
      try {
        fs.chmodSync(bundled, 0o755);
      } catch {
        // Read-only install, the bit is either already there or nothing helps.
      }
    }
    return bundled;
  }
  return "ppl-language-server";
}

export async function activate(context: vscode.ExtensionContext) {

  const traceOutputChannel = vscode.window.createOutputChannel("PPL Language Server trace");
  const command = serverCommand(context);
  const run: Executable = {
    command,
    options: {
      env: {
        ...process.env,
        // eslint-disable-next-line @typescript-eslint/naming-convention
        RUST_LOG: "debug",
      },
    },
  };
  const serverOptions: ServerOptions = {
    run,
    debug: run,
  };
  // If the extension is launched in debug mode then the debug server options are used
  // Otherwise the run options are used
  // Options to control the language client
  let clientOptions: LanguageClientOptions = {
    // Register the server for plain text documents
    documentSelector: [{ scheme: "file", language: "ppl" }],
    synchronize: {
      // Notify the server about file changes to '.clientrc files contained in the workspace
      fileEvents: vscode.workspace.createFileSystemWatcher("**/.clientrc"),
    },
    traceOutputChannel,
  };
  // Create the language client and start the client.
  client = new LanguageClient("ppl-language-server", "ppl language server", serverOptions, clientOptions);
  // activateInlayHints(context);
  try {
    await client.start();
  } catch (error) {
    const openSettings = "Open settings";
    const answer = await vscode.window.showErrorMessage(
      `PPL: could not start '${command}'. Install a build for your platform, or set ppl.serverPath to a server you built yourself.`,
      openSettings,
    );
    if (answer === openSettings) {
      await vscode.commands.executeCommand("workbench.action.openSettings", "ppl.serverPath");
    }
    return;
  }

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (!event.affectsConfiguration("ppl.serverPath")) {
        return;
      }
      const reload = "Reload window";
      const answer = await vscode.window.showInformationMessage("PPL: the server path changed.", reload);
      if (answer === reload) {
        await vscode.commands.executeCommand("workbench.action.reloadWindow");
      }
    }),
  );
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

export function activateInlayHints(ctx: vscode.ExtensionContext) {
  const maybeUpdater = {
    hintsProvider: null as Disposable | null,
    updateHintsEventEmitter: new vscode.EventEmitter<void>(),

    async onConfigChange() {
      this.dispose();

      const event = this.updateHintsEventEmitter.event;
      // this.hintsProvider = languages.registerInlayHintsProvider(
      //   { scheme: "file", language: "nrs" },
      //   // new (class implements InlayHintsProvider {
      //   //   onDidChangeInlayHints = event;
      //   //   resolveInlayHint(hint: InlayHint, token: CancellationToken): ProviderResult<InlayHint> {
      //   //     const ret = {
      //   //       label: hint.label,
      //   //       ...hint,
      //   //     };
      //   //     return ret;
      //   //   }
      //   //   async provideInlayHints(
      //   //     document: TextDocument,
      //   //     range: Range,
      //   //     token: CancellationToken
      //   //   ): Promise<InlayHint[]> {
      //   //     const hints = (await client
      //   //       .sendRequest("custom/inlay_hint", { path: document.uri.toString() })
      //   //       .catch(err => null)) as [number, number, string][];
      //   //     if (hints == null) {
      //   //       return [];
      //   //     } else {
      //   //       return hints.map(item => {
      //   //         const [start, end, label] = item;
      //   //         let startPosition = document.positionAt(start);
      //   //         let endPosition = document.positionAt(end);
      //   //         return {
      //   //           position: endPosition,
      //   //           paddingLeft: true,
      //   //           label: [
      //   //             {
      //   //               value: `${label}`,
      //   //               // location: {
      //   //               //   uri: document.uri,
      //   //               //   range: new Range(1, 0, 1, 0)
      //   //               // }
      //   //               command: {
      //   //                 title: "hello world",
      //   //                 command: "helloworld.helloWorld",
      //   //                 arguments: [document.uri],
      //   //               },
      //   //             },
      //   //           ],
      //   //         };
      //   //       });
      //   //     }
      //   //   }
      //   // })()
      // );
    },

    onDidChangeTextDocument({ contentChanges, document }: vscode.TextDocumentChangeEvent) {
      // debugger
      // this.updateHintsEventEmitter.fire();
    },

    dispose() {
      this.hintsProvider?.dispose();
      this.hintsProvider = null;
      this.updateHintsEventEmitter.dispose();
    },
  };

  vscode.workspace.onDidChangeConfiguration(maybeUpdater.onConfigChange, maybeUpdater, ctx.subscriptions);
  vscode.workspace.onDidChangeTextDocument(maybeUpdater.onDidChangeTextDocument, maybeUpdater, ctx.subscriptions);
  vscode.workspace.onDidCloseTextDocument(maybeUpdater.onConfigChange, maybeUpdater, ctx.subscriptions);

  maybeUpdater.onConfigChange().catch(console.error);
}
