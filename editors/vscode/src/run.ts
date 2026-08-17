import * as vscode from "vscode";
import { execFile } from "child_process";
import * as path from "path";

import { binary, findManifest, locate, locateBoardConfig, reportMissing, reportMissingBoard } from "./binaries";

/// What the board is called with, with the places a path goes filled in.
function runArguments(replacements: Record<string, string>): string[] {
  const configured = vscode.workspace.getConfiguration("icyboardPpl").get<string[]>("runArguments");
  const template = configured?.length ? configured : ["--ppe", "${ppe}"];
  return template.map((argument) => argument.replace(/\$\{(\w+)\}/g, (whole, name) => replacements[name] ?? whole));
}

interface CompilerConfig {
  output: string;
}

/// Asks the compiler what it would build and where it would put it, so the
/// target directory layout is known in one place only.
function configOf(compiler: string, target: string): Promise<CompilerConfig> {
  return new Promise((resolve, reject) => {
    execFile(compiler, ["--print-config-json", target], { cwd: path.dirname(target) }, (error, stdout, stderr) => {
      if (error) {
        reject(new Error(`${stdout}${stderr}`.trim() || error.message));
        return;
      }
      try {
        resolve(JSON.parse(stdout) as CompilerConfig);
      } catch {
        reject(new Error(`the compiler configuration could not be read:\n${stdout}`));
      }
    });
  });
}

function compile(compiler: string, target: string): Promise<string> {
  return new Promise((resolve, reject) => {
    // The output pane shows escapes rather than colour, so ask for plain text.
    execFile(compiler, ["--mono", target], { cwd: path.dirname(target) }, (error, stdout, stderr) => {
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

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    // One we may not signal is still one that is running.
    return (error as NodeJS.ErrnoException).code === "EPERM";
  }
}

async function waitForExit(pid: number): Promise<void> {
  for (let waited = 0; waited < 2000 && alive(pid); waited += 50) {
    await sleep(50);
  }
  if (!alive(pid)) {
    return;
  }
  try {
    process.kill(pid, "SIGKILL");
  } catch {
    // It ended between the question and the signal.
  }
  for (let waited = 0; waited < 1000 && alive(pid); waited += 50) {
    await sleep(50);
  }
}

/// A board holds the lock on its directory for as long as it runs, so the next
/// run waits for the process itself rather than only for its terminal, which
/// closes without having reaped it yet.
async function stopPreviousRun(name: string): Promise<void> {
  const previous = vscode.window.terminals.filter((terminal) => terminal.name === name);
  // Only a terminal that has not reported an exit still owns its process id; a
  // finished one may name a number the system has given away since.
  const running = previous.filter((terminal) => terminal.exitStatus === undefined);
  const pids = await Promise.all(running.map((terminal) => terminal.processId));
  previous.forEach((terminal) => terminal.dispose());
  for (const pid of pids) {
    if (pid !== undefined) {
      await waitForExit(pid);
    }
  }
}

/// Builds the open source and runs what came out of it.
///
/// The board takes over the terminal it runs in - a PPE asks its caller
/// questions - so this gets a real terminal rather than a task.
export async function runPpe(output: vscode.OutputChannel, options: { singleFile?: boolean } = {}): Promise<void> {
  const document = vscode.window.activeTextEditor?.document;
  if (!document || document.uri.scheme !== "file" || !document.fileName.toLowerCase().endsWith(".pps")) {
    vscode.window.showErrorMessage("IcyBoard PPL: open the .pps you want to run.");
    return;
  }
  if (!(await document.save())) {
    return;
  }

  const compiler = binary("pplc");
  const board = binary("icboard");
  // Both are looked for before anything runs, so a missing one is a dialog
  // rather than a shell error nobody reads.
  if (!locate(compiler)) {
    await reportMissing(compiler, "the IcyBoard programs");
    return;
  }
  if (!locate(board)) {
    await reportMissing(board, "the IcyBoard programs");
    return;
  }

  const source = document.fileName;
  const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri)?.uri;
  // A file that belongs to a project is built as part of it, so the manifest's
  // language version, defines and data files apply instead of bare defaults.
  const manifest = options.singleFile ? undefined : findManifest(path.dirname(source), workspaceFolder?.fsPath);
  const target = manifest ?? source;

  const report = async (reason: unknown, what: string) => {
    output.appendLine(`${reason}`);
    const showOutput = "Show output";
    const answer = await vscode.window.showErrorMessage(`IcyBoard PPL: ${what}`, showOutput);
    if (answer === showOutput) {
      output.show(true);
    }
  };

  let plan: CompilerConfig;
  try {
    plan = await configOf(compiler, target);
  } catch (error) {
    await report(error, `the configuration of ${path.basename(target)} could not be read.`);
    return;
  }

  if (manifest) {
    output.appendLine(`Building project ${manifest}`);
  }
  try {
    const built = await compile(compiler, target);
    if (built) {
      output.appendLine(built);
    }
  } catch (error) {
    await report(error, `${path.basename(target)} did not build.`);
    return;
  }

  const from = workspaceFolder?.fsPath ?? path.dirname(source);
  const configured = vscode.workspace.getConfiguration("icyboardPpl").get<string>("boardConfig")?.trim();
  const boardConfig = locateBoardConfig(configured || undefined, from);
  if (!boardConfig) {
    await reportMissingBoard();
    return;
  }

  const parts = [
    board,
    ...runArguments({
      ppe: plan.output,
      source,
      workspaceFolder: from,
    }),
    boardConfig,
  ];

  const name = "IcyBoard PPL";
  await stopPreviousRun(name);

  output.appendLine(`Running ${commandLine(parts)}`);
  // Started without a shell, so the terminal's process id is the board's own.
  const terminal = vscode.window.createTerminal({
    name,
    cwd: workspaceFolder ?? vscode.Uri.file(path.dirname(source)),
    shellPath: parts[0],
    shellArgs: parts.slice(1),
  });

  terminal.show();
}
