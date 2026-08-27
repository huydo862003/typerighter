import {
  window,
} from 'vscode';
import type {
  LanguageClient,
} from 'vscode-languageclient/node';

// Resolve prompts in command args before sending to the LSP server
export async function resolvePromptsAndExecute (
  client: LanguageClient,
  command: string,
  args: unknown[],
): Promise<void> {
  const commandArgs = args[0] as CommandArgs | undefined;

  if (!commandArgs) return;

  const prompts = commandArgs.prompts ?? [];

  for (const prompt of prompts) {
    if (prompt.kind === 'input') {
      const value = await window.showInputBox({
        prompt: prompt.prompt,
        value: prompt.default,
      });

      if (value === undefined) return;
      commandArgs[prompt.field] = value;
    } else if (prompt.kind === 'select') {
      const value = await window.showQuickPick(prompt.choices ?? [], {
        placeHolder: prompt.prompt,
      });

      if (value === undefined) return;
      commandArgs[prompt.field] = value;
    }
  }

  // Clear prompts before sending
  delete commandArgs.prompts;

  await client.sendRequest('workspace/executeCommand', {
    command,
    arguments: args,
  });
}

interface CommandArgs {
  prompts?: Prompt[];
  [key: string]: unknown;
}

interface Prompt {
  kind: 'input' | 'select';
  field: string;
  prompt: string;
  default?: string;
  choices?: string[];
}
