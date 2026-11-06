import {
  computed, inject, provide,
} from 'vue';
import type {
  InjectionKey,
} from 'vue';
import type {
  Route,
} from '../router';
import {
  useTheme,
} from '../../theme-default/composables/client-preferences/useTheme';
import type {
  TypedownData,
} from '@/shared';

const tdContentKey: InjectionKey<TypedownData> = Symbol('td-content');

// Provide reactive content bindings from the current route
export function provideTdContent (route: Route): TypedownData {
  const theme = useTheme();
  const state: TypedownData = {
    page: computed(() => route.data),
    frontmatter: computed(() => route.data.frontmatter),
    title: computed(() => route.data.title),
    isDark: computed(() => theme.isDark.value),
  };

  provide(tdContentKey, state);

  return state;
}

// Inject the reactive content bindings provided by a parent component
export function useTdContent (): TypedownData {
  const state = inject(tdContentKey);

  if (state === undefined) {
    throw new Error('useTdContent() called without a parent provideTdContent()');
  }

  return state;
}
