import {
  useIndexedStore,
} from '../useIndexedDb';
import {
  type CardState, isCardDue, newCardState, nextState,
} from './sm2';

export type {
  CardState,
};

export function useFlashcardStore () {
  const store = useIndexedStore<CardState>('flashcards');

  async function getCardStates (
    cardIds: string[],
  ): Promise<Map<string, CardState>> {
    const result = new Map<string, CardState>();

    for (const id of cardIds) {
      const state = await store.get(id);

      if (state) result.set(id, state);
    }

    return result;
  }

  async function pruneStaleCards (validFilepaths: Set<string>): Promise<number> {
    return store.prune(validFilepaths);
  }

  async function reviewCard (
    cardId: string,
    quality: number,
  ): Promise<CardState> {
    const current = (await store.get(cardId)) ?? newCardState();
    const next = nextState(current, quality);

    await store.set(cardId, next);

    return next;
  }

  return {
    getCardStates,
    isCardDue,
    pruneStaleCards,
    reviewCard,
  };
}
