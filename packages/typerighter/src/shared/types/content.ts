export interface ContentIcon {
  /** Lucide icon name */
  name: string;
}

export interface ContentSummary {
  /** File path relative to content dir, with extension */
  filepath: string;
  /** Schema type name */
  schema?: string;
  /** Human-readable schema label */
  schemaLabel?: string;
  /** Display label */
  label?: string;
  /** Page icon */
  icon?: ContentIcon;
  /** Frontmatter header */
  header: Record<string, unknown>;
  /** First paragraph of the body content */
  excerpt?: string;
  /** File metadata */
  metadata: FileMetadata;
}

export interface ContentTree {
  /** Interleaved files and directories at the root, sorted by numeric prefix */
  entries: ContentTreeEntry[];
}

export type ContentTreeEntry =
  | {
    kind: 'file';
    item: ContentSummary;
  }
  | {
    kind: 'dir';
    node: ContentTreeNode;
  };

export interface ContentTreeNode {
  name: string;
  /** Interleaved files and directories, sorted by numeric prefix */
  entries: ContentTreeEntry[];
}

export interface DirectoryEntry {
  name: string;
  url: string;
  /** First sentence of body or description field */
  description?: string;
  /** Tags from frontmatter */
  tags?: string[];
  /** Last modification time as seconds since UNIX epoch */
  mtime?: number;
  /** Schema type name */
  schema?: string;
}

export interface DirectoryListing {
  title: string;
  url: string;
  /** Interleaved subdirectories and items, sorted by numeric prefix */
  entries: DirectoryListingEntry[];
}

export type DirectoryListingEntry =
  | {
    kind: 'file';
    item: DirectoryEntry;
  }
  | {
    kind: 'dir';
    sub: SubdirectoryEntry;
  };

export interface FileMetadata {
  /** Last modification time as seconds since UNIX epoch */
  mtime: number;
  /** Creation time as seconds since UNIX epoch */
  ctime: number;
}

export interface PropertyDescriptor {
  /** UI widget hint for rendering this property */
  widget: PropertyWidget;
  /** Whether this property is optional */
  optional?: boolean;
  /** Allowed values for select/multi_select widgets */
  options?: string[];
  /** Target schema name for relation widgets */
  schema?: string;
  /** Item descriptor for list widgets */
  items?: PropertyDescriptor;
}

export type PropertyWidget = 'text' | 'number' | 'checkbox' | 'date' | 'select' | 'multiSelect' | 'relation' | 'list';

export interface SchemaDefinition {
  [property: string]: PropertyDescriptor;
}

export type SchemaGroups = Record<string, ContentSummary[]>;

export interface SubdirectoryEntry {
  name: string;
  url: string;
  count: number;
}
