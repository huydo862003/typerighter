import {
  log, outro,
} from '@clack/prompts';

// Prints instructions for installing the Typedown extension from the Zed marketplace
export function setupZed (): void {
  log.info('Zed extensions are installed from the Zed marketplace');
  log.info('Open Zed and run: "zed: extensions" from the command palette');
  log.info('Search for "typedown" and click Install');

  outro('Follow the instructions above to install in Zed.');
}
