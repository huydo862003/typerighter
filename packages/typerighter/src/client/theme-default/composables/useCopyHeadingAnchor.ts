import {
  onMounted, onUnmounted,
} from 'vue';

// Copy the full URL of a heading anchor to the clipboard when clicked
export function useCopyHeadingAnchor (): void {
  function handleClick (event: MouseEvent): void {
    const anchor = (event.target as Element)?.closest('.td-header-anchor');

    if (!anchor) return;

    event.preventDefault();

    const hash = anchor.getAttribute('href') ?? '';
    const url = location.href.split('#')[0] + hash;

    navigator.clipboard.writeText(url).catch(() => {});

    anchor.classList.add('td-header-anchor--copied');

    setTimeout(() => {
      anchor.classList.remove('td-header-anchor--copied');
    }, 2000);
  }

  onMounted(() => {
    document.addEventListener('click', handleClick);
  });

  onUnmounted(() => {
    document.removeEventListener('click', handleClick);
  });
}
