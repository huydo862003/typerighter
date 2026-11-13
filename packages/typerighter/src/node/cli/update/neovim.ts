import {
  execSync,
} from 'node:child_process';
import {
  log,
} from '@clack/prompts';
import {
  type PluginManager,
} from '../types';
import {
  detectNeovimPluginManager,
  resolveNeovimConfigDirectory,
} from '../setup-editor/neovim';

// Tries to update via headless nvim; falls back to printing the update command
export async function updateNeovim (): Promise<void> {
  const configDirectory = resolveNeovimConfigDirectory();

  if (!configDirectory) return;

  const pluginManager = await detectNeovimPluginManager(configDirectory);
  const headlessCommand = neovimHeadlessUpdateCommand(pluginManager);

  if (headlessCommand) {
    log.info('Updating Neovim plugin...');

    try {
      execSync(headlessCommand, {
        stdio: 'inherit',
      });

      log.success('Neovim plugin updated');

      return;
    } catch {
      // Fall through to instructions
    }
  }

  const command = neovimUpdateCommand(pluginManager);

  log.info('To update the Neovim plugin, run inside Neovim:');
  log.info(`  ${command}`);
}

// Returns the headless nvim command to update the plugin, or undefined if not supported
function neovimHeadlessUpdateCommand (pluginManager: PluginManager): string | undefined {
  switch (pluginManager) {
    case 'lazy.nvim':
    case 'unknown':
      return 'nvim --headless "+Lazy! update typerighter" +qa';
    case 'packer.nvim':
      return 'nvim --headless -c "autocmd User PackerComplete quitall" -c "PackerUpdate"';
    case 'vim-plug':
      return 'nvim --headless "+PlugUpdate typerighter" +qa';
    default:
      return undefined;
  }
}

// Returns the Neovim command to run in order to update the typerighter plugin
function neovimUpdateCommand (pluginManager: PluginManager): string {
  switch (pluginManager) {
    case 'mini.deps':
      return ':lua MiniDeps.update("huydo862003/typerighter")';
    case 'packer.nvim':
      return ':PackerUpdate typerighter';
    case 'vim-plug':
      return ':PlugUpdate typerighter';
    default:
      return ':Lazy update typerighter';
  }
}
