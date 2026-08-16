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

/// The name a board configuration has when only its directory is known.
const BOARD_FILE = "icboard.toml";

function isDirectory(candidate: string): boolean {
  try {
    return fs.statSync(candidate).isDirectory();
  } catch {
    return false;
  }
}

/// Mirrors how the board looks for its configuration: what it was told, taking a
/// directory to mean the file in it, and then ICB_PATH.
export function locateBoardConfig(configured: string | undefined, from: string): string | undefined {
  const named = configured ? path.resolve(from, configured) : from;
  const file = isDirectory(named) ? path.join(named, BOARD_FILE) : named;
  const extension = path.extname(file);
  const candidate = extension ? `${file.slice(0, -extension.length)}.toml` : `${file}.toml`;
  if (fs.existsSync(candidate)) {
    return candidate;
  }

  const fromEnvironment = process.env.ICB_PATH;
  if (fromEnvironment) {
    const path_ = isDirectory(fromEnvironment) ? path.join(fromEnvironment, BOARD_FILE) : fromEnvironment;
    if (fs.existsSync(path_)) {
      return path_;
    }
  }
  return undefined;
}

/// A PPE runs on a board, so there is nothing to run it on without one.
export async function reportMissingBoard(): Promise<void> {
  const openSettings = "Open settings";
  const answer = await vscode.window.showErrorMessage(
    `IcyBoard PPL: no board to run the PPE on. Point icyboardPpl.boardConfig at an ${BOARD_FILE}, or at the directory holding one.`,
    openSettings,
  );
  if (answer === openSettings) {
    await vscode.commands.executeCommand("workbench.action.openSettings", "icyboardPpl.boardConfig");
  }
}

/// The manifest that makes a directory a PPL package.
const MANIFEST = "ppl.toml";

/// The project a file belongs to, looked for upwards from it. The search stops at
/// the workspace folder so a stray manifest further up cannot claim the file.
export function findManifest(from: string, stopAt?: string): string | undefined {
  const last = stopAt ? path.resolve(stopAt) : undefined;
  let directory = path.resolve(from);
  for (;;) {
    const candidate = path.join(directory, MANIFEST);
    if (fs.existsSync(candidate)) {
      return candidate;
    }
    const parent = path.dirname(directory);
    if (directory === last || parent === directory) {
      return undefined;
    }
    directory = parent;
  }
}

