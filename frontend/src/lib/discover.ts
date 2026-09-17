import type { Component } from 'svelte';

/**
 * Vite static glob. Future workers add `src/features/<name>/Demo.svelte`
 * without editing this file, App.svelte, or a name registry.
 */
export const fixtureModules = import.meta.glob<{ default: Component }>(
  '../features/*/Demo.svelte',
);
