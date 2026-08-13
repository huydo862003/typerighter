import {
  useSiteConfig,
} from '..';

// Construct URLs that respect the configured base path
export function useUrl () {
  const config = useSiteConfig();
  const base = config.basePath.replace(/\/$/, '');

  function url (path: string): string {
    return base + path;
  }

  function homeUrl (): string {
    return url('/index');
  }

  function indexUrl (directoryPath: string): string {
    return url(directoryPath + '/index');
  }

  return {
    url,
    homeUrl,
    indexUrl,
  };
}
