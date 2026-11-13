import {
  intro, outro,
} from '@clack/prompts';
import {
  updateNeovim,
} from './neovim';
import {
  updateVault,
} from './vault';
import {
  updateVscode,
} from './vscode';

// Entry point for `typerighter update`. Updates the vault dep, VS Code extension, and prints Neovim instructions
export async function update (root: string): Promise<void> {
  intro('typerighter update');

  updateVault(root);
  updateVscode();
  await updateNeovim();

  outro('Done.');
}
