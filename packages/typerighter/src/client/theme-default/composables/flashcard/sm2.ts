// SM-2 spaced repetition algorithm
// Pure function: takes current state + quality rating, returns next state

const INITIAL_EASE = 2.5;
const MIN_EASE = 1.3;

export interface CardState {
  version: number;
  easeFactor: number;
  interval: number;
  repetitions: number;
  nextReview: number;
  lastGrade?: Grade;
}

export type Grade = 'again' | 'hard' | 'good' | 'easy';

export const SCHEMA_VERSION = 1;

export function isCardDue (state: CardState | undefined): boolean {
  return !state || state.nextReview <= Date.now();
}

export function newCardState (): CardState {
  return {
    version: SCHEMA_VERSION,
    easeFactor: INITIAL_EASE,
    interval: 0,
    repetitions: 0,
    nextReview: 0,
  };
}

const GRADE_NAMES: Grade[] = [
  'again',
  'hard',
  'good',
  'easy',
];

// SM-2 quality scores: again=0 hard=3 good=4 easy=5
// Scores below 3 reset the card (lapse), scores >= 3 advance it
const QUALITY_SCORES = [
  0,
  3,
  4,
  5,
];

// quality: 0=again 1=hard 2=good 3=easy
export function nextState (current: CardState, quality: number): CardState {
  const gradeName = GRADE_NAMES[quality] ?? 'again';
  const score = QUALITY_SCORES[quality] ?? 0;
  const nextEase = Math.max(
    MIN_EASE,
    current.easeFactor + (0.1 - (5 - score) * (0.08 + (5 - score) * 0.02)),
  );

  // Lapse: reset repetitions, short retry interval
  if (score < 3) {
    return {
      version: SCHEMA_VERSION,
      easeFactor: nextEase,
      interval: 1,
      repetitions: 0,
      nextReview: Date.now() + 60_000,
      lastGrade: gradeName,
    };
  }

  // Pass: advance interval based on repetition count
  let nextInterval: number;

  if (current.repetitions === 0) {
    nextInterval = 1;
  } else if (current.repetitions === 1) {
    nextInterval = 6;
  } else {
    nextInterval = Math.round(current.interval * nextEase);
  }

  // "hard" gets a shorter interval (60% of normal)
  if (quality === 1) {
    nextInterval = Math.max(1, Math.round(nextInterval * 0.6));
  }

  return {
    version: SCHEMA_VERSION,
    easeFactor: nextEase,
    interval: nextInterval,
    repetitions: current.repetitions + 1,
    nextReview: Date.now() + nextInterval * 86_400_000,
    lastGrade: gradeName,
  };
}

// Returns predicted next intervals (in days) for each grade given a card state
// 0 means sub-day (e.g. 60s retry for "again")
export function previewIntervals (state: CardState | undefined): {
  again: number;
  hard: number;
  good: number;
  easy: number;
} {
  if (!state || state.repetitions === 0) {
    return {
      again: 0,
      hard: 1,
      good: 1,
      easy: 4,
    };
  }

  let baseInterval: number;

  if (state.repetitions === 1) {
    baseInterval = 6;
  } else {
    baseInterval = Math.round(state.interval * state.easeFactor);
  }

  return {
    again: 0,
    hard: Math.max(1, Math.round(baseInterval * 0.6)),
    good: baseInterval,
    easy: Math.round(baseInterval * 1.3),
  };
}
