import type {
  ViteDevServer,
} from 'vite';
import {
  RESOLVED_THEME_ID,
} from '../constants';
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

    return generateThemeModule(config.layout ?? undefined, context.root);
  }

  invalidate (server: ViteDevServer): void {
    const module_ = server.moduleGraph.getModuleById(RESOLVED_THEME_ID);

    if (module_) server.moduleGraph.invalidateModule(module_);
  }
}
