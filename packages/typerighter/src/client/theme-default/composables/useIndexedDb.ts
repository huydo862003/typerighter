// Thin IndexedDB abstraction for client-side persistence
// Single object store with namespaced keys (namespace:key)

const DB_NAME = 'typerighter';
const DB_VERSION = 1;
const STORE_NAME = 'kv';

let dbPromise: Promise<IDBDatabase> | undefined;

export interface IndexedStore<T> {
  get: (key: string) => Promise<T | undefined>;
  set: (key: string, value: T) => Promise<void>;
  delete: (key: string) => Promise<void>;
  keys: () => Promise<string[]>;
  getAll: () => Promise<Map<string, T>>;
  prune: (validKeys: Set<string>) => Promise<number>;
}

export function useIndexedStore<T> (namespace: string): IndexedStore<T> {
  const prefix = namespace + ':';

  function prefixed (key: string): string {
    return prefix + key;
  }

  function unprefixed (raw: string): string {
    return raw.slice(prefix.length);
  }

  return {
    async get (key) {
      const db = await open();

      return new Promise((resolve, reject) => {
        const request = db
          .transaction(STORE_NAME, 'readonly')
          .objectStore(STORE_NAME)
          .get(prefixed(key));

        request.onsuccess = () => resolve(request.result as T | undefined);
        request.onerror = () => reject(request.error);
      });
    },

    async set (key, value) {
      const db = await open();

      return new Promise((resolve, reject) => {
        const transaction = db.transaction(STORE_NAME, 'readwrite');

        transaction.objectStore(STORE_NAME).put(value, prefixed(key));
        transaction.oncomplete = () => resolve();
        transaction.onerror = () => reject(transaction.error);
      });
    },

    async delete (key) {
      const db = await open();

      return new Promise((resolve, reject) => {
        const transaction = db.transaction(STORE_NAME, 'readwrite');

        transaction.objectStore(STORE_NAME).delete(prefixed(key));
        transaction.oncomplete = () => resolve();
        transaction.onerror = () => reject(transaction.error);
      });
    },

    async keys () {
      const db = await open();

      return new Promise((resolve, reject) => {
        const result: string[] = [];
        const range = IDBKeyRange.bound(prefix, prefix + '\uffff');
        const request = db
          .transaction(STORE_NAME, 'readonly')
          .objectStore(STORE_NAME)
          .openKeyCursor(range);

        request.onsuccess = () => {
          const cursor = request.result;

          if (cursor) {
            result.push(unprefixed(cursor.key as string));
            cursor.continue();
          } else {
            resolve(result);
          }
        };
        request.onerror = () => reject(request.error);
      });
    },

    async getAll () {
      const db = await open();

      return new Promise((resolve, reject) => {
        const result = new Map<string, T>();
        const range = IDBKeyRange.bound(prefix, prefix + '\uffff');
        const cursor = db
          .transaction(STORE_NAME, 'readonly')
          .objectStore(STORE_NAME)
          .openCursor(range);

        cursor.onsuccess = () => {
          const entry = cursor.result;

          if (entry) {
            result.set(unprefixed(entry.key as string), entry.value as T);
            entry.continue();
          } else {
            resolve(result);
          }
        };
        cursor.onerror = () => reject(cursor.error);
      });
    },

    async prune (validKeys) {
      const db = await open();

      return new Promise((resolve, reject) => {
        const transaction = db.transaction(STORE_NAME, 'readwrite');
        let pruned = 0;
        const range = IDBKeyRange.bound(prefix, prefix + '\uffff');
        const cursor = transaction.objectStore(STORE_NAME).openCursor(range);

        cursor.onsuccess = () => {
          const entry = cursor.result;

          if (entry) {
            if (!validKeys.has(unprefixed(entry.key as string))) {
              entry.delete();
              pruned++;
            }
            entry.continue();
          }
        };
        transaction.oncomplete = () => resolve(pruned);
        transaction.onerror = () => reject(transaction.error);
      });
    },
  };
}

function open (): Promise<IDBDatabase> {
  if (dbPromise) return dbPromise;

  dbPromise = new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);

    request.onupgradeneeded = () => {
      const db = request.result;

      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME);
      }
    };

    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });

  return dbPromise;
}
