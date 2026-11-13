import {
  cancel, intro, isCancel, log, select,
} from '@clack/prompts';
import {
  detectNvimBinary, setupNeovim,
} from './neovim';
import {
  detectVscodeBinary, setupVscode,
} from './vscode';
import {
  detectZedBinary, setupZed,
} from './zed';

// Entry point for `typerighter setup-editor [editor]`
export async function setupEditor (
  editorArg: string | undefined,
): Promise<void> {
  intro('typerighter setup-editor');

  const detectedEditor = detectInstalledEditors();
  const editor = editorArg
    ? parseEditorArg(editorArg)
    : await promptForEditor(detectedEditor);

  switch (editor) {
    case 'neovim':
      await setupNeovim();
      break;
    case 'vscode':
      setupVscode(
        detectedEditor.find((entry) => entry.editor === 'vscode')?.bin
        ?? 'code',
      );
      break;
    case 'zed':
      setupZed();
      break;
  }
}

interface DetectedEditor {
  bin: string;
  editor: Editor;
  label: string;
}
type Editor = 'neovim' | 'vscode' | 'zed';

// Checks which supported editors are installed on the current machine
function detectInstalledEditors (): DetectedEditor[] {
  const editors: DetectedEditor[] = [];

  const nvimBin = detectNvimBinary();

  if (nvimBin) {
    editors.push({
      bin: nvimBin,
      editor: 'neovim',
      label: 'Neovim',
    });
  }

  const vscodeBin = detectVscodeBinary();

  if (vscodeBin) {
    editors.push({
      bin: vscodeBin,
      editor: 'vscode',
      label: vscodeBin === 'code' ? 'VS Code' : `VS Code (${vscodeBin})`,
    });
  }

  const zedBin = detectZedBinary();

  if (zedBin) {
    editors.push({
      bin: zedBin,
      editor: 'zed',
      label: 'Zed',
    });
  }

  return editors;
}

// Normalizes common editor aliases (nvim, code, codium, ...) to a canonical Editor value
function parseEditorArg (argument: string): Editor {
  const normalized = argument.toLowerCase().trim();

  if (normalized === 'neovim' || normalized === 'nvim') return 'neovim';
  if (
    normalized === 'vscode'
    || normalized === 'code'
    || normalized === 'codium'
  )
    return 'vscode';
  if (normalized === 'zed') return 'zed';

  log.error(`Unknown editor: ${argument}. Supported: neovim, vscode, zed`);
  process.exit(1);
}

// Prompts the user to choose from the detected editors, or exits if none are found
async function promptForEditor (detected: DetectedEditor[]): Promise<Editor> {
  if (detected.length === 0) {
    log.error('No supported editors detected (neovim, vscode, zed)');
    log.info(
      'Install one of these editors or pass the editor name explicitly:',
    );
    log.info('  typerighter setup-editor neovim');
    process.exit(1);
  }

  if (detected.length === 1) {
    log.info(`Detected ${detected[0].label}`);

    return detected[0].editor;
  }

  const choice = await select({
    message: 'Which editor would you like to set up?',
    options: detected.map((entry) => ({
      label: entry.label,
      value: entry.editor,
    })),
  });

  if (isCancel(choice)) {
    cancel('Cancelled.');
    process.exit(0);
  }

  return choice;
}
