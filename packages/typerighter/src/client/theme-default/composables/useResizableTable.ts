import {
  onMounted, onUnmounted,
} from 'vue';

const HANDLE_WIDTH = 6;
const MIN_COL_WIDTH = 40;

// Enable column resizing on all tables inside .td-content via event delegation
export function useResizableTable (): void {
  function handlePointerDown (event: PointerEvent): void {
    const th = (event.target as Element)?.closest('.td-content th') as HTMLTableCellElement | undefined;

    if (!th) return;

    // Only trigger when clicking near the right edge of the header cell
    const rect = th.getBoundingClientRect();

    if (rect.right - event.clientX > HANDLE_WIDTH) return;

    // Find the adjacent header cell
    const nextTh = th.nextElementSibling as HTMLTableCellElement | undefined;

    if (!nextTh) return;

    event.preventDefault();

    // Ensure the table uses fixed layout for explicit column widths
    const table = th.closest('table') as HTMLTableElement;

    table.style.tableLayout = 'fixed';

    // Lock initial widths from computed values
    const headers = table.querySelectorAll<HTMLTableCellElement>('th');

    for (const header of headers) {
      header.style.width = `${header.offsetWidth}px`;
    }

    const startX = event.clientX;
    const startWidth = th.offsetWidth;
    const nextStartWidth = nextTh.offsetWidth;

    document.body.style.cursor = 'col-resize';

    function onMove (moveEvent: PointerEvent) {
      const delta = moveEvent.clientX - startX;

      th!.style.width = `${Math.max(MIN_COL_WIDTH, startWidth + delta)}px`;
      nextTh!.style.width = `${Math.max(MIN_COL_WIDTH, nextStartWidth - delta)}px`;
    }

    function onUp () {
      document.body.style.cursor = '';
      document.removeEventListener('pointermove', onMove);
      document.removeEventListener('pointerup', onUp);
    }

    document.addEventListener('pointermove', onMove);
    document.addEventListener('pointerup', onUp);
  }

  onMounted(() => {
    document.addEventListener('pointerdown', handlePointerDown);
  });

  onUnmounted(() => {
    document.removeEventListener('pointerdown', handlePointerDown);
  });
}
