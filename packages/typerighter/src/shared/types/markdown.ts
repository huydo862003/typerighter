/* Based on https://github.com/vuejs/vitepress/blob/main/types/shared.d.ts
 *
 * Commit: 2fa0ded
 * */
// Manually declaring all properties as rollup-plugin-dts
// is unable to merge augmented module declarations
export interface MarkdownEnv {
  /**
   * Populated by `@mdit-vue/plugin-headers`
   */
  headers?: MarkdownHeading[];
  /**
   * The title that extracted by `@mdit-vue/plugin-title`
   */
  title?: string;
  path: string;
  relativePath: string;
  cleanUrls: boolean;
  links?: string[];
  linkLines?: number[];
  includes?: string[];
  realPath?: string;
  localeIndex?: string;
}

export interface MarkdownHeading {
  /**
   * The level of the header
   *
   * `1` to `6` for `<h1>` to `<h6>`
   */
  level: number;
  /**
   * The title of the header
   */
  title: string;
  /**
   * The slug of the header
   *
   * Typically the `id` attr of the header anchor
   */
  slug: string;
  /**
   * Link of the header
   *
   * Typically using `#${slug}` as the anchor hash
   */
  link: string;
  /**
   * The children of the header
   */
  children: MarkdownHeading[];
}
