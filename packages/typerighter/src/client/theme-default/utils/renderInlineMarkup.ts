import {
  ref, watchEffect, type Ref,
} from 'vue';
import {
  escapeHtml,
} from '@/shared';

// Matches inline code (`...`) and inline math ($...$)
const INLINE_RE = /`([^`]+)`|\$([^$]+)\$/g;

// temml is loaded lazily as it's very heavy
export function useLazyInlineMarkup (getText: () => string): Ref<string> {
  // Starts with code-only rendering, then upgrades to math if the text contains '$'
  const rendered = ref(renderCodeOnly(getText()));

  watchEffect((onCleanup) => {
    const text = getText();

    if (!text.includes('$')) {
      rendered.value = renderCodeOnly(text);

      return;
    }

    let active = true;

    onCleanup(() => {
      active = false;
    });

    import('./renderMath').then(({
      renderWithMath,
    }) => {
      if (active) rendered.value = renderWithMath(text);
    });
  });

  return rendered;
}

function renderCodeOnly (text: string): string {
  let lastIndex = 0;
  let result = '';

  for (const match of text.matchAll(INLINE_RE)) {
    result += escapeHtml(text.slice(lastIndex, match.index));

    if (match[1] !== undefined) {
      result += `<code>${escapeHtml(match[1])}</code>`;
    } else {
      result += escapeHtml(match[0]);
    }

    lastIndex = match.index + match[0].length;
  }

  result += escapeHtml(text.slice(lastIndex));

  return result;
}
