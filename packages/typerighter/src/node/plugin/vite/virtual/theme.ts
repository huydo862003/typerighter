import type {
  ViteDevServer,
} from 'vite';
import {
  RESOLVED_THEME_ID,
} from '../constants';
import {
  invalidateVirtualModule,
} from './utils';
import type {
  AppContext,
} from '@/node/context';
import {
  generateThemeModule,
} from '@/node/lib/codegen';

export class VirtualTheme {
  async load (context: AppContext): Promise<string> {
    const tdContext = await context.getTdContext();
    const config = await tdContext.getConfig();

    return generateThemeModule(
      config.layout ?? undefined,
      context.root,
    );
  }

  invalidate (server: ViteDevServer): void {
    invalidateVirtualModule(server, RESOLVED_THEME_ID);
  }
}
