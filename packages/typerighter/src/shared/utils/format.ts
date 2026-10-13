// Extract numeric prefix and the rest of the string
// e.g. "01-getting-started" -> { order: 1, rest: "getting-started" }
// e.g. "advanced" -> { order: Infinity, rest: "advanced" }
export function parseNumericPrefix (name: string): {
  order: number;
  rest: string;
} {
  const match = name.match(/^(\d+)[-_](.+)$/);

  if (match) return {
    order: Number(match[1]),
    rest: match[2],
  };

  return {
    order: Infinity,
    rest: name,
  };
}

// Convert a kebab-case or snake_case string to PascalCase
// e.g. "video-embed" -> "VideoEmbed", "my_chart" -> "MyChart"
export function toPascalCase (string_: string): string {
  return string_.replace(/(^|[-_])([a-z])/g, (_, __, letter: string) => letter.toUpperCase());
}

// Replace dashes and underscores with spaces, capitalize the first word
// Strips leading numeric prefixes (e.g. "01-design-mockups" -> "Design mockups")
export function unslugify (slug: string): string {
  const {
    rest,
  } = parseNumericPrefix(slug);
  const words = rest.replace(/[-_]/g, ' ').split(' ');

  if (0 < words.length) {
    words[0] = words[0].charAt(0).toUpperCase() + words[0].slice(1);
  }

  return words.join(' ');
}
