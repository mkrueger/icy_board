import * as vscode from "vscode";
import * as fs from "fs";

import { Executable, LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";

import { runPpe } from "./run";
import { binary } from "./binaries";

let client: LanguageClient | undefined;

/// The server is looked for where the user said, then in the environment, then
/// in the extension itself, then on the PATH.
function serverCommand(context: vscode.ExtensionContext): string {
  const configured = binary("ppl-lsp");
  if (configured !== "ppl-lsp") {
    return configured;
  }
  if (process.env.SERVER_PATH) {
    return process.env.SERVER_PATH;
  }
  const name = process.platform === "win32" ? "ppl-lsp.exe" : "ppl-lsp";
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
  return "ppl-lsp";
}

export async function activate(context: vscode.ExtensionContext) {
  const output = vscode.window.createOutputChannel("PPL");
  const traceOutputChannel = vscode.window.createOutputChannel("PPL Language Server trace");
  context.subscriptions.push(output, traceOutputChannel);
  context.subscriptions.push(vscode.commands.registerCommand("ppl.run", () => runPpe(output)));
  context.subscriptions.push(
    vscode.commands.registerCommand("ppl.runFile", () => runPpe(output, { singleFile: true })),
  );

  const command = serverCommand(context);
  const run: Executable = { command };
  const serverOptions: ServerOptions = { run, debug: run };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "ppl" }],
    traceOutputChannel,
  };

  client = new LanguageClient("ppl", "PPL language server", serverOptions, clientOptions);
  context.subscriptions.push(client);

  try {
    await client.start();
  } catch (error) {
    output.appendLine(`${error}`);
    const openSettings = "Open settings";
    const answer = await vscode.window.showErrorMessage(
      `PPL: could not start '${command}'. Install a build for your platform, or point ppl.binPath at the directory holding the IcyBoard programs.`,
      openSettings,
    );
    if (answer === openSettings) {
      await vscode.commands.executeCommand("workbench.action.openSettings", "ppl.binPath");
    }
    return;
  }

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (!event.affectsConfiguration("ppl.binPath")) {
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
  return client?.stop();
}
