import {
  h, render,
} from 'vue';
import TdDiagram from '../../components/TdDiagram.vue';

// Bridge between the lazy-embed DOM pipeline and the Vue component
// Renders TdDiagram into the placeholder element, letting Vue own the DOM from that point

export function mountDiagram (
  placeholder: Element,
  options: {
    /** Rendered SVG markup */
    svg: string;
    /** Diagram source code */
    source: string;
    /** Optional caption */
    title?: string;
    /** Display label (Mermaid, D2) */
    label: string;
    /** Unique element id */
    diagramId: string;
  },
): void {
  const container = document.createElement('div');

  container.style.display = 'contents'; // wrapper is layout-transparent
  placeholder.replaceWith(container);
  render(h(TdDiagram, options), container);
}
