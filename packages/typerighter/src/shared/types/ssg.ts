import type {
  Component, Ref,
} from 'vue';
import type {
  ContentIcon, FileMetadata,
} from './content';

export interface MarkdownHeading {
  /** 1 to 6 for h1 to h6 */
  level: number;
  title: string;
  /** The id attr of the header anchor */
  slug: string;
  /** Anchor link, typically #slug */
  link: string;
  children: MarkdownHeading[];
}

export interface PageData {
  schema?: string;
  schemaLabel?: string;
  label?: string;
  icon?: ContentIcon;
  frontmatter: Record<string, unknown>;
  headings: MarkdownHeading[];
  title: string;
  metadata?: FileMetadata;
}

export interface PageModule {
  __pageData: PageData;
  default: Component;
}

export interface TypedownData {
  /** Page-level data from the .td file */
  page: Ref<PageData>;
  /** Frontmatter fields */
  frontmatter: Ref<Record<string, unknown>>;
  /** Page title */
  title: Ref<string>;
  /** Dark mode state */
  isDark: Ref<boolean>;
}
