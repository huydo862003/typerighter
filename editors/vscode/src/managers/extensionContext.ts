import type {
  ExtensionContext,
} from 'vscode';

/**
 * Singleton providing global access to the VSCode ExtensionContext
 */
export class ExtensionContextManager {
  private static instance: ExtensionContextManager | undefined;
  private readonly context: ExtensionContext;

  private constructor (context: ExtensionContext) {
    this.context = context;
  }

  /** Must be called once during extension activation before any context access */
  static initialize (context: ExtensionContext): void {
    if (!ExtensionContextManager.instance) {
      ExtensionContextManager.instance = new ExtensionContextManager(context);
    }
  }

  // Returns the stored ExtensionContext
  // throws if initialize() was not called
  static get context (): ExtensionContext {
    if (!ExtensionContextManager.instance) {
      throw new Error('ExtensionContextManager not initialized. Call initialize() first.');
    }

    return ExtensionContextManager.instance.context;
  }
}
