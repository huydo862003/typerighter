import {
  execSync,
} from 'node:child_process';
import fs from 'node:fs/promises';
import {
  existsSync,
} from 'node:fs';
import path from 'node:path';
import {
  log, outro,
} from '@clack/prompts';
import {
  type PluginManager,
} from '../types';
import {
  commandExists,
} from '../../sys-utils/command-exists';

// Reads init.lua / init.vim to identify which plugin manager the user has installed
export async function detectNeovimPluginManager (
  configDirectory: string,
): Promise<PluginManager> {
  try {
    const initializeLua = await fs.readFile(
      path.join(configDirectory, 'init.lua'),
      'utf-8',
    );

    if (
      initializeLua.includes('require("lazy')
      || initializeLua.includes('require(\'lazy')
      || initializeLua.includes('require("lazy.nvim')
      || initializeLua.includes('folke/lazy.nvim')
    )
      return 'lazy.nvim';

    if (initializeLua.includes('MiniDeps') || initializeLua.includes('mini.deps'))
      return 'mini.deps';

    if (initializeLua.includes('packer') || initializeLua.includes('wbthomason/packer.nvim'))
      return 'packer.nvim';
  } catch {
    // No init.lua
  }

  try {
    const initializeVim = await fs.readFile(
      path.join(configDirectory, 'init.vim'),
      'utf-8',
    );

    if (initializeVim.includes('call plug#begin') || initializeVim.includes('Plug '))
      return 'vim-plug';

    if (initializeVim.includes('packer')) return 'packer.nvim';
  } catch {
    // No init.vim
  }

  if (existsSync(path.join(configDirectory, 'lazy-lock.json')))
    return 'lazy.nvim';

  return 'unknown';
}

// Verifies `nvim` is in PATH and that it is actually Neovim, not an alias to something else
export function detectNvimBinary (): string | undefined {
  if (!commandExists('nvim')) return undefined;

  try {
    const output = execSync('nvim --version', {
      encoding: 'utf-8',
      stdio: [
        'ignore',
        'pipe',
        'ignore',
      ],
    });

    return output.trimStart().startsWith('NVIM v') ? 'nvim' : undefined;
  } catch {
    return undefined;
  }
}

// Generates the Lua snippet the user should add to their config for the detected plugin manager
export function generateNeovimSnippet (pluginManager: PluginManager): string {
  if (pluginManager === 'mini.deps') {
    return `MiniDeps.add("huydo862003/typerighter")
local base = vim.fn.stdpath("data") .. "/site/pack/deps/opt/typerighter/editors/nvim"
vim.opt.runtimepath:prepend(base)
vim.filetype.add({ extension = { td = "typedown" } })
for _, f in ipairs(vim.fn.glob(base .. "/plugin/*.lua", false, true)) do
  vim.cmd.source(f)
end`;
  }

  if (pluginManager === 'packer.nvim') {
    return `use {
  "huydo862003/typerighter",
  tag = "v${__VERSION__}",
  config = function()
    local base = vim.fn.stdpath("data") .. "/site/pack/packer/opt/typerighter/editors/nvim"
    vim.opt.runtimepath:prepend(base)
    vim.filetype.add({ extension = { td = "typedown" } })
    for _, f in ipairs(vim.fn.glob(base .. "/plugin/*.lua", false, true)) do
      vim.cmd.source(f)
    end
  end,
}`;
  }

  if (pluginManager === 'vim-plug') {
    return `" In your init.vim, inside plug#begin/plug#end:
Plug 'huydo862003/typerighter', { 'tag': 'v${__VERSION__}' }

" After plug#end, add:
lua << EOF
local base = vim.fn.stdpath("data") .. "/plugged/typerighter/editors/nvim"
vim.opt.runtimepath:prepend(base)
vim.filetype.add({ extension = { td = "typedown" } })
for _, f in ipairs(vim.fn.glob(base .. "/plugin/*.lua", false, true)) do
  vim.cmd.source(f)
end
EOF`;
  }

  // Default: lazy.nvim
  return `{
  "huydo862003/typerighter",
  tag = "v${__VERSION__}",
  lazy = false,
  submodules = false,
  init = function()
    local base = vim.fn.stdpath("data") .. "/lazy/typerighter/editors/nvim"
    vim.opt.runtimepath:prepend(base)
    vim.filetype.add({ extension = { td = "typedown" } })
    for _, f in ipairs(vim.fn.glob(base .. "/plugin/*.lua", false, true)) do
      vim.cmd.source(f)
    end
  end,
}`;
}

// Resolves the Neovim config directory, checking XDG, standard, and Windows paths
export function resolveNeovimConfigDirectory (): string | undefined {
  const xdg = process.env.XDG_CONFIG_HOME;
  const home = process.env.HOME ?? process.env.USERPROFILE ?? '~';
  const candidates = [
    xdg ? path.join(xdg, 'nvim') : undefined,
    path.join(home, '.config', 'nvim'),
    process.platform === 'win32'
      ? path.join(home, 'AppData', 'Local', 'nvim')
      : undefined,
  ].filter(Boolean) as string[];

  return candidates.find((directory) => existsSync(directory));
}

// Tries to auto-configure and install; falls back to printing the config snippet
export async function setupNeovim (): Promise<void> {
  const configDirectory = resolveNeovimConfigDirectory();

  if (!configDirectory) {
    log.error('Could not find Neovim config directory');
    log.info('Create ~/.config/nvim/ and try again, or set up manually:');
    log.info('  https://github.com/huydo862003/typerighter#neovim');
    process.exit(1);
  }

  const pluginManager = await detectNeovimPluginManager(configDirectory);

  log.info(`Neovim config: ${configDirectory}`);
  log.info(
    `Plugin manager: ${pluginManager === 'unknown' ? 'not detected' : pluginManager}`,
  );

  if (await autoInstallNeovim(configDirectory, pluginManager)) {
    outro('Neovim plugin installed.');

    return;
  }

  const snippet = generateNeovimSnippet(pluginManager);

  log.step('Add this to your Neovim config:\n');
  console.log(snippet);
  console.log('');

  outro('Neovim setup instructions printed above.');
}

// Appends the config snippet to init.lua and triggers a headless install
// For vim-plug (init.vim), only attempts headless install if already configured
// Returns true if auto-install succeeded, false if the user should be shown the snippet
async function autoInstallNeovim (
  configDirectory: string,
  pluginManager: PluginManager,
): Promise<boolean> {
  if (pluginManager === 'vim-plug') {
    return triggerNeovimHeadlessInstall(pluginManager);
  }

  const initializeLuaPath = path.join(configDirectory, 'init.lua');

  try {
    let existing = '';

    try {
      existing = await fs.readFile(initializeLuaPath, 'utf-8');
    } catch {
      // File doesn't exist yet - will be created by appendFile
    }

    if (existing.includes('huydo862003/typerighter')) {
      log.info('typerighter already in Neovim config');
    } else {
      const snippet = generateNeovimSnippet(pluginManager);

      await fs.appendFile(initializeLuaPath, `\n\n-- typerighter\n${snippet}\n`);
      log.success('Added typerighter to init.lua');
    }
  } catch {
    return false;
  }

  return triggerNeovimHeadlessInstall(pluginManager);
}

// Returns the headless nvim command to install the plugin, or undefined if not supported
function neovimHeadlessInstallCommand (pluginManager: PluginManager): string | undefined {
  switch (pluginManager) {
    case 'lazy.nvim':
    case 'unknown':
      return 'nvim --headless "+Lazy! sync" +qa';
    case 'packer.nvim':
      return 'nvim --headless -c "autocmd User PackerComplete quitall" -c "PackerSync"';
    case 'vim-plug':
      return 'nvim --headless +PlugInstall +qa';
    default:
      return undefined;
  }
}

// Runs the headless nvim install command for the detected plugin manager
function triggerNeovimHeadlessInstall (pluginManager: PluginManager): boolean {
  const command = neovimHeadlessInstallCommand(pluginManager);

  if (!command) return false;

  log.info('Running Neovim headless install...');

  try {
    execSync(command, {
      stdio: 'inherit',
    });

    log.success('Neovim plugin installed');

    return true;
  } catch {
    return false;
  }
}
