import * as vscode from "vscode";
import * as fs from "fs";

import { Executable, LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";

import { runPpe } from "./run";

let client: LanguageClient | undefined;

/// The server is looked for where the user said, then in the environment, then
/// in the extension itself, then on the PATH.
function serverCommand(context: vscode.ExtensionContext): string {
  const configured = vscode.workspace.getConfiguration("icyboardPpl").get<string>("serverPath")?.trim();
  if (configured) {
    return configured;
  }
  if (process.env.SERVER_PATH) {
    return process.env.SERVER_PATH;
  }
  const name = process.platform === "win32" ? "icyboard-ppl.exe" : "icyboard-ppl";
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
  return "icyboard-ppl";
}

export async function activate(context: vscode.ExtensionContext) {
  const output = vscode.window.createOutputChannel("IcyBoard PPL");
  const traceOutputChannel = vscode.window.createOutputChannel("PPL Language Server trace");
  context.subscriptions.push(output, traceOutputChannel);
  context.subscriptions.push(vscode.commands.registerCommand("icyboard-ppl.run", () => runPpe(output)));

  const command = serverCommand(context);
  const run: Executable = { command };
  const serverOptions: ServerOptions = { run, debug: run };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "ppl" }],
    traceOutputChannel,
  };

  client = new LanguageClient("icyboard-ppl", "IcyBoard PPL language server", serverOptions, clientOptions);
  context.subscriptions.push(client);

  try {
    await client.start();
  } catch (error) {
    output.appendLine(`${error}`);
    const openSettings = "Open settings";
    const answer = await vscode.window.showErrorMessage(
      `IcyBoard PPL: could not start '${command}'. Install a build for your platform, or set icyboardPpl.serverPath to a server you built yourself.`,
      openSettings,
    );
    if (answer === openSettings) {
      await vscode.commands.executeCommand("workbench.action.openSettings", "icyboardPpl.serverPath");
    }
    return;
  }

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (!event.affectsConfiguration("icyboardPpl.serverPath")) {
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
