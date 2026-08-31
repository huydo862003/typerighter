import {
  path,
} from '@/shared';

export interface ResolvedRef {
  url: string;
  name: string;
  format?: string;
  icon?: {
    name: string;
  };
  isImage: boolean;
}

const IMAGE_EXTENSIONS = new Set([
  'png',
  'jpg',
  'jpeg',
  'gif',
  'webp',
  'svg',
  'avif',
  'bmp',
  'ico',
  'tiff',
]);

export function extractRef (value: unknown): ResolvedRef | undefined {
  if (!value) return undefined;

  if (typeof value === 'string') {
    const url = value.trim();

    if (!url || !isImageRef({
      url,
    })) return undefined;

    return {
      url,
      name: path.basename(url),
      isImage: true,
    };
  }

  if (typeof value !== 'object') return undefined;

  const targetObject = value as Record<string, unknown>;
  const rawRef = (targetObject.$ref ?? targetObject) as Record<string, unknown>;

  if (typeof rawRef.url !== 'string') return undefined;

  const url = rawRef.url;
  const name = (rawRef.name ?? path.basename(url)) as string;
  const format = (rawRef.format ?? targetObject.format) as string | undefined;
  const icon = rawRef.icon as {
    name: string;
  } | undefined;
  const isImage = isImageRef({
    url,
    name,
    format,
  });

  return {
    url,
    name,
    format,
    icon,
    isImage,
  };
}

export function isImageRef (ref: {
  url: string;
  name?: string;
  format?: string;
}): boolean {
  if (ref.format && IMAGE_EXTENSIONS.has(ref.format.toLowerCase())) {
    return true;
  }
  const extension = path.extname(ref.name || ref.url).slice(1);

  return extension ? IMAGE_EXTENSIONS.has(extension.toLowerCase()) : false;
}
