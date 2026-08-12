import {
  EXTERNAL_URL_RE,
} from '../regexes';
import {
  filestem, dirname, join, extname,
} from './path';

// Returns true if the URL has a protocol prefix (https:, mailto:, data:, etc.)
export function isUrlExternal (path: string): boolean {
  return EXTERNAL_URL_RE.test(path);
}

// Known file extensions that should be treated as downloadable files, not page links
// Intentionally excludes .md and .html so they are treated as navigable pages
const DOWNLOADABLE_FILE_EXTENSIONS = new Set(
  (
    '3g2,3gp,aac,ai,apng,au,avif,bin,bmp,cer,class,conf,crl,css,csv,dll,'
    + 'doc,eps,epub,exe,gif,gz,ics,ief,jar,jpe,jpeg,jpg,js,json,jsonld,m4a,'
    + 'man,mid,midi,mjs,mov,mp2,mp3,mp4,mpe,mpeg,mpg,mpp,oga,ogg,ogv,ogx,'
    + 'opus,otf,p10,p7c,p7m,p7s,pdf,png,ps,qt,roff,rtf,rtx,ser,svg,t,tif,'
    + 'tiff,tr,ts,tsv,ttf,txt,vtt,wav,weba,webm,webp,woff,woff2,xhtml,xml,'
    + 'yaml,yml,zip'
  ).split(','),
);

export function getDirectoryUrl (urlPrefix: string, name: string): string {
  return `${urlPrefix}/${name}`;
}

export function getParentUrl (urlPath: string): string {
  return dirname(urlPath.replace(/\/$/, '')) || '/';
}

export function getTdContentUrl (filepath: string): string {
  const name = filestem(filepath);

  return '/' + join(dirname(filepath), name);
}

export function isUrlAncestorOf (directoryUrl: string, path: string): boolean {
  return path.startsWith(directoryUrl + '/') || path === directoryUrl;
}

// Returns true if the path looks like a page link (not a file download)
export function isUrlToPage (filename: string): boolean {
  const extension = extname(filename).slice(1);

  return extension === '' || !DOWNLOADABLE_FILE_EXTENSIONS.has(extension.toLowerCase());
}
