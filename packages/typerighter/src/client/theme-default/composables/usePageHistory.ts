import {
  computed,
  inject,
  provide,
  ref,
  watch,
  type ComputedRef,
  type InjectionKey,
} from 'vue';
import {
  useTdContent, useRoute, useRouter,
} from '@/client/app';

const STORAGE_KEY = 'td-page-history';
const MAX_HISTORY = 50;
const RECENT_COUNT = 10;

const historyKey: InjectionKey<PageHistoryState> = Symbol('page-history');

export interface HistoryEntry {
  path: string;
  title: string;
}

// Called once from TdLayout
export function providePageHistory (): PageHistoryState {
  const queue = new PageHistoryQueue(readHistory());
  // Incremented after any mutation so computed refs re-evaluate
  const version = ref(0);
  const route = useRoute();
  const router = useRouter();
  const {
    title,
  } = useTdContent();

  function persistToLocalStorage (): void {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(queue.backEntries()));
    version.value++;
  }

  // Watch path and title together so the old title is available as oldTitle in the callback
  watch(
    () => [
      route.path,
      title.value ?? '',
    ] as [string, string],
    ([newPath], [
      oldPath,
      oldTitle,
    ]) => {
      if (!oldPath || oldPath === newPath) return;

      if (queue.contains(oldPath)) return;

      queue.push(oldPath, oldTitle || oldPath);
      persistToLocalStorage();
    },
  );

  const canGoBack = computed(() => 0 <= version.value && queue.canGoBack);

  const canGoNext = computed(() => 0 <= version.value && queue.canGoNext);

  const recentPages = computed((): HistoryEntry[] =>
    0 <= version.value ? queue.recentEntries(RECENT_COUNT) : []);

  function goBack (): void {
    const entry = queue.stepBack(route.path, title.value ?? route.path);

    if (!entry) return;

    persistToLocalStorage();
    router.go(entry.path);
  }

  function goNext (): void {
    const entry = queue.stepNext(route.path, title.value ?? route.path);

    if (!entry) return;

    persistToLocalStorage();
    router.go(entry.path);
  }

  const state: PageHistoryState = {
    goBack,
    goNext,
    canGoBack,
    canGoNext,
    recentPages,
  };

  provide(historyKey, state);

  return state;
}

// Called from any descendant of TdLayout - injects the shared state, no extra watchers
export function usePageHistory (): PageHistoryState {
  const state = inject(historyKey);

  if (!state)
    throw new Error(
      'usePageHistory() called without a parent providePageHistory()',
    );

  return state;
}

interface PageHistoryState {
  goBack: () => void;
  goNext: () => void;
  canGoBack: ComputedRef<boolean>;
  canGoNext: ComputedRef<boolean>;
  recentPages: ComputedRef<HistoryEntry[]>;
}

// Manages page history as a navigable queue
class PageHistoryQueue {
  private back: HistoryEntry[];
  private forward: HistoryEntry[];

  constructor (savedBack: HistoryEntry[]) {
    this.back = savedBack.slice();
    this.forward = [];
  }

  get canGoBack (): boolean {
    return 0 < this.back.length;
  }

  get canGoNext (): boolean {
    return 0 < this.forward.length;
  }

  // Records a page that was just left via regular navigation and clears forward history
  push (path: string, title: string): void {
    this.forward = [];

    const last = this.back.at(-1);

    if (last?.path === path) {
      last.title ||= title;

      return;
    }

    this.back.push({
      path,
      title,
    });

    if (MAX_HISTORY < this.back.length) this.back.shift();
  }

  // Moves one step back
  // returns the entry to navigate to
  stepBack (
    currentPath: string,
    currentTitle: string,
  ): HistoryEntry | undefined {
    const entry = this.back.pop();

    if (!entry) return undefined;

    this.forward.unshift({
      path: currentPath,
      title: currentTitle,
    });

    return entry;
  }

  // Moves one step forward
  // returns the entry to navigate to
  stepNext (
    currentPath: string,
    currentTitle: string,
  ): HistoryEntry | undefined {
    const entry = this.forward.shift();

    if (!entry) return undefined;

    this.back.push({
      path: currentPath,
      title: currentTitle,
    });

    if (MAX_HISTORY < this.back.length) this.back.shift();

    return entry;
  }

  // Returns the most recent back entries for display, most recent first
  recentEntries (count: number): HistoryEntry[] {
    return this.back.slice(-count).reverse();
  }

  // Returns the back stack for persistence
  backEntries (): HistoryEntry[] {
    return this.back.slice();
  }

  // Returns true if path is currently in back or forward
  contains (path: string): boolean {
    return (
      this.back.some((entry) => entry.path === path)
      || this.forward.some((entry) => entry.path === path)
    );
  }
}

function readHistory (): HistoryEntry[] {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]');
  } catch {
    return [];
  }
}
