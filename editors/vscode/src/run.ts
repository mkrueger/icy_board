import * as vscode from "vscode";
import { execFile } from "child_process";
import * as path from "path";

function setting(name: string, fallback: string): string {
  const value = vscode.workspace.getConfiguration("icyboardPpl").get<string>(name)?.trim();
  return value ? value : fallback;
}

/// What the board is called with, with the places a path goes filled in.
function runArguments(replacements: Record<string, string>): string[] {
  const configured = vscode.workspace.getConfiguration("icyboardPpl").get<string[]>("runArguments");
  const template = configured?.length ? configured : ["--ppe", "${ppe}"];
  return template.map((argument) => argument.replace(/\$\{(\w+)\}/g, (whole, name) => replacements[name] ?? whole));
}

function compile(compiler: string, source: string): Promise<string> {
  return new Promise((resolve, reject) => {
    execFile(compiler, [source], { cwd: path.dirname(source) }, (error, stdout, stderr) => {
      const output = `${stdout}${stderr}`.trim();
      if (error) {
        reject(new Error(output || error.message));
        return;
      }
      resolve(output);
    });
  });
}

/// The terminal takes a command line, so anything with a space needs quoting.
function commandLine(parts: string[]): string {
  const quote = (part: string) =>
    process.platform === "win32" ? (/[\s&|<>^]/.test(part) ? `"${part}"` : part) : `'${part.replace(/'/g, `'\\''`)}'`;
  return parts.map(quote).join(" ");
}

/// Builds the open source and runs what came out of it.
///
/// The board takes over the terminal it runs in - a PPE asks its caller
/// questions - so this gets a real terminal rather than a task.
export async function runPpe(output: vscode.OutputChannel): Promise<void> {
  const document = vscode.window.activeTextEditor?.document;
  if (!document || document.uri.scheme !== "file" || !document.fileName.toLowerCase().endsWith(".pps")) {
    vscode.window.showErrorMessage("IcyBoard PPL: open the .pps you want to run.");
    return;
  }
  if (!(await document.save())) {
    return;
  }

  const source = document.fileName;
  const compiler = setting("compilerPath", "pplc");
  try {
    const built = await compile(compiler, source);
    if (built) {
      output.appendLine(built);
    }
  } catch (error) {
    output.appendLine(`${error}`);
    output.show(true);
    vscode.window.showErrorMessage(`IcyBoard PPL: '${compiler}' did not build ${path.basename(source)}.`);
    return;
  }

  const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri)?.uri;
  const executable = source.replace(/\.pps$/i, ".ppe");
  const config = setting("boardConfig", "");
  const parts = [
    setting("boardPath", "icboard"),
    ...runArguments({
      ppe: executable,
      source,
      workspaceFolder: workspaceFolder?.fsPath ?? path.dirname(source),
    }),
    ...(config ? [config] : []),
  ];

  const name = "IcyBoard PPL";
  const terminal =
    vscode.window.terminals.find((candidate) => candidate.name === name) ??
    vscode.window.createTerminal({
      name,
      cwd: workspaceFolder ?? vscode.Uri.file(path.dirname(source)),
    });

  terminal.show();
  terminal.sendText(commandLine(parts));
}
