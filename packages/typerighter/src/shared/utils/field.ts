export function isBuiltinField (key: string): boolean {
  return key.startsWith('_');
}
