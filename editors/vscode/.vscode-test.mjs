import { defineConfig } from '@vscode/test-cli';
import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';

// On NixOS, downloaded VSCode binaries are dynamically linked and won't run.
// Detect NixOS and use the system-installed codium/code instead.
function detectVscodeExecutable() {
	if (!existsSync('/etc/NIXOS')) return undefined;
	for (const bin of ['codium', 'code']) {
		try {
			const path = execSync(`which ${bin}`, { encoding: 'utf-8' }).trim();
			if (path) return path;
		} catch {}
	}
	return undefined;
}

const vscodeExecutablePath = detectVscodeExecutable();

const shared = {
	mocha: { ui: 'tdd', timeout: 15000 },
	launchArgs: ['--disable-extensions'],
	...(vscodeExecutablePath ? { useInstallation: { fromPath: vscodeExecutablePath } } : {}),
};

export default defineConfig([
	{
		label: 'diagnostics',
		files: 'out/tests/diagnostics/**/*.test.js',
		workspaceFolder: './src/tests/fixtures/diagnostics-vault',
		...shared,
	},
	{
		label: 'completion',
		files: 'out/tests/completion/**/*.test.js',
		workspaceFolder: './src/tests/fixtures/basic-vault',
		...shared,
	},
	{
		label: 'definition',
		files: 'out/tests/definition/**/*.test.js',
		workspaceFolder: './src/tests/fixtures/basic-vault',
		...shared,
	},
	{
		label: 'references',
		files: 'out/tests/references/**/*.test.js',
		workspaceFolder: './src/tests/fixtures/basic-vault',
		...shared,
	},
	{
		label: 'hover',
		files: 'out/tests/hover/**/*.test.js',
		workspaceFolder: './src/tests/fixtures/basic-vault',
		...shared,
	},
	{
		label: 'rename',
		files: 'out/tests/rename/**/*.test.js',
		workspaceFolder: './src/tests/fixtures/rename-vault',
		...shared,
	},
	{
		label: 'code-action',
		files: 'out/tests/code-action/**/*.test.js',
		workspaceFolder: './src/tests/fixtures/basic-vault',
		...shared,
	},
	{
		label: 'syntax-highlighting',
		files: 'out/tests/syntax-highlighting/**/*.test.js',
		workspaceFolder: './src/tests/fixtures/basic-vault',
		...shared,
	},
	{
		label: 'inlay-hints',
		files: 'out/tests/inlay-hints/**/*.test.js',
		workspaceFolder: './src/tests/fixtures/basic-vault',
		...shared,
	},
	{
		label: 'snippets',
		files: 'out/tests/snippets/**/*.test.js',
		workspaceFolder: './src/tests/fixtures/basic-vault',
		...shared,
	},
	{
		label: 'formatting',
		files: 'out/tests/formatting/**/*.test.js',
		workspaceFolder: './src/tests/fixtures/basic-vault',
		...shared,
	},
]);
