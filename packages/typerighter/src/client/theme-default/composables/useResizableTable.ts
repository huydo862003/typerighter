import {
  onMounted, onUnmounted,
} from 'vue';

const HANDLE_WIDTH = 6;
const MIN_COL_WIDTH = 40;

// Enable column resizing on all tables inside .td-content via event delegation
export function useResizableTable (): void {
  let lastHoveredTh: HTMLTableCellElement | undefined;
  let dragging = false;

  function isNearRightEdge (th: HTMLTableCellElement, clientX: number): boolean {
    const rect = th.getBoundingClientRect();

    return rect.right - clientX < HANDLE_WIDTH;
  }

  function handlePointerMove (event: PointerEvent): void {
    if (dragging) return;

    const target = event.target as Element | undefined;

    if (!target?.closest('.td-content')) {
      if (lastHoveredTh) {
        lastHoveredTh.classList.remove('is-near-border');
        lastHoveredTh = undefined;
      }

      return;
    }

    const th = target.closest('th') as HTMLTableCellElement | undefined;

    if (lastHoveredTh && lastHoveredTh !== th) {
      lastHoveredTh.classList.remove('is-near-border');
      lastHoveredTh = undefined;
    }

    if (!th || !th.nextElementSibling) return;

    if (isNearRightEdge(th, event.clientX)) {
      th.classList.add('is-near-border');
      lastHoveredTh = th;
    } else {
      th.classList.remove('is-near-border');
      lastHoveredTh = undefined;
    }
  }

  function handlePointerDown (event: PointerEvent): void {
    const th = (event.target as Element)?.closest('.td-content th') as HTMLTableCellElement | undefined;

    if (!th || !th.nextElementSibling) return;
    if (!isNearRightEdge(th, event.clientX)) return;

    event.preventDefault();
    dragging = true;

    const startX = event.clientX;
    const startWidth = th.offsetWidth;

    th.classList.add('is-resizing');
    document.body.style.cursor = 'col-resize';

    function onMove (moveEvent: PointerEvent) {
      const delta = moveEvent.clientX - startX;

      if (!th) return;
      th.style.width = `${Math.max(MIN_COL_WIDTH, startWidth + delta)}px`;
    }

    function onUp () {
      dragging = false;
      th?.classList.remove('is-resizing');
      document.body.style.cursor = '';
      document.removeEventListener('pointermove', onMove);
      document.removeEventListener('pointerup', onUp);
    }

    document.addEventListener('pointermove', onMove);
    document.addEventListener('pointerup', onUp);
  }

  onMounted(() => {
    document.addEventListener('pointermove', handlePointerMove);
    document.addEventListener('pointerdown', handlePointerDown);
  });

  onUnmounted(() => {
    document.removeEventListener('pointermove', handlePointerMove);
    document.removeEventListener('pointerdown', handlePointerDown);
  });
}
