const shellLangs = [
  'shellscript',
  'shell',
  'bash',
  'sh',
  'zsh',
];

// Check if a language identifier is a shell language
export function isShellLanguage (language: string): boolean {
  return shellLangs.includes(language);
}

const LANGUAGE_RE = /^[a-zA-Z0-9-_]+/;

// Extract the language name from the fence info string
export function extractLanguage (info: string): string {
  return LANGUAGE_RE.exec(info)?.[0] || '';
}
