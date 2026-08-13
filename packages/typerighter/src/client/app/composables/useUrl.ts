import {
  useSiteConfig,
} from '..';
import {
  getIndexUrl,
} from '@/shared';

// Construct URLs that respect the configured base path
export function useUrl () {
  const config = useSiteConfig();
  const base = config.basePath.replace(/\/$/, '');

  function url (path: string): string {
    return base + path;
  }

  function homeUrl (): string {
    return url(getIndexUrl('/'));
  }

  function directoryUrl (directoryPath: string): string {
    return url(getIndexUrl(directoryPath));
  }

  return {
    url,
    homeUrl,
    directoryUrl,
  };
}
