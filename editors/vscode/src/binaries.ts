import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";

function binaryDirectory(): string | undefined {
  const value = vscode.workspace.getConfiguration("icyboardPpl").get<string>("binPath")?.trim();
  return value ? value : undefined;
}

/// `pplc`, `icboard` and the language server are built and installed together, so
/// one directory finds all three. Without it they are looked up on the PATH.
export function binary(name: string): string {
  const directory = binaryDirectory();
  if (!directory) {
    return name;
  }
  return path.join(directory, process.platform === "win32" ? `${name}.exe` : name);
}

/// Where the command would be found, or nothing when it is nowhere.
export function locate(command: string): string | undefined {
  if (command.includes(path.sep) || path.isAbsolute(command)) {
    return fs.existsSync(command) ? command : undefined;
  }
  const suffixes = process.platform === "win32" ? (process.env.PATHEXT ?? ".EXE;.CMD;.BAT").split(";") : [""];
  for (const directory of (process.env.PATH ?? "").split(path.delimiter)) {
    for (const suffix of suffixes) {
      const candidate = path.join(directory, command + suffix);
      if (fs.existsSync(candidate)) {
        return candidate;
      }
    }
  }
  return undefined;
}

/// Says which program is missing and offers the setting that would find it.
export async function reportMissing(command: string, whatItIs: string): Promise<void> {
  const openSettings = "Open settings";
  const where = binaryDirectory() ? `'${command}' is not there` : `'${command}' is not on your PATH`;
  const answer = await vscode.window.showErrorMessage(
    `IcyBoard PPL: ${where}. Point icyboardPpl.binPath at the directory holding ${whatItIs}.`,
    openSettings,
  );
  if (answer === openSettings) {
    await vscode.commands.executeCommand("workbench.action.openSettings", "icyboardPpl.binPath");
  }
}
