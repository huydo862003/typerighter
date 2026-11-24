import {
  type CardState, newCardState, nextState,
} from '../components/custom/flashcard/sm2';
import {
  useIndexedStore,
} from './useIndexedDb';

export type {
  CardState,
} from '../components/custom/flashcard/sm2';
export {
  isDue,
} from '../components/custom/flashcard/sm2';

const store = useIndexedStore<CardState>('flashcards');

export async function getCardState (cardId: string): Promise<CardState | undefined> {
  return store.get(cardId);
}

export async function getCardStates (cardIds: string[]): Promise<Map<string, CardState>> {
  const result = new Map<string, CardState>();

  for (const id of cardIds) {
    const state = await store.get(id);

    if (state) result.set(id, state);
  }

  return result;
}

export async function pruneStaleCards (validFilepaths: Set<string>): Promise<number> {
  return store.prune(validFilepaths);
}

export async function reviewCard (cardId: string, quality: number): Promise<CardState> {
  const current = (await store.get(cardId)) ?? newCardState();
  const next = nextState(current, quality);

  await store.set(cardId, next);

  return next;
}
