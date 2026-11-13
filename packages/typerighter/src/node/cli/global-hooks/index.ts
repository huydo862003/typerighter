import {
  checkForUpdates,
} from './update-check';

/**
 * Run all global CLI hooks, called once at the start of every CLI invocation
 * Hooks must be non-blocking and never throw
 */
export function runGlobalHooks (): void {
  checkForUpdates();
}
