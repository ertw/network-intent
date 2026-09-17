/** Fixture folder names become `/#/<name>` routes. Not a product registry. */
const FIXTURE_NAME = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;

export type FixtureResolution =
  | { status: 'idle'; name: null }
  | { status: 'found'; name: string }
  | { status: 'unknown'; name: string };

export function fixtureNameFromModulePath(modulePath: string): string | null {
  const normalized = modulePath.replaceAll('\\', '/');
  const match = normalized.match(/(?:^|\/)features\/([^/]+)\/Demo\.svelte$/);
  if (!match) {
    return null;
  }
  const name = match[1];
  if (!FIXTURE_NAME.test(name)) {
    return null;
  }
  return name;
}

export function discoverFixtureNames(modulePaths: readonly string[]): string[] {
  const names = new Set<string>();
  for (const modulePath of modulePaths) {
    const name = fixtureNameFromModulePath(modulePath);
    if (name) {
      names.add(name);
    }
  }
  return [...names].sort((a, b) => a.localeCompare(b));
}

export function parseFixtureRoute(hash: string): string | null {
  const withoutHash = hash.startsWith('#') ? hash.slice(1) : hash;
  const pathOnly = withoutHash.split(/[?#]/, 1)[0] ?? '';
  const trimmed = pathOnly.replace(/^\/+|\/+$/g, '');
  if (!trimmed) {
    return null;
  }
  let decoded: string;
  try {
    decoded = decodeURIComponent(trimmed);
  } catch {
    return trimmed;
  }
  if (decoded.includes('/') || decoded.includes('\\') || decoded.includes('..')) {
    return decoded;
  }
  return decoded;
}

type FixtureNameSource =
  | ReadonlySet<string>
  | ReadonlyMap<string, unknown>
  | readonly string[]
  | Readonly<Record<string, unknown>>;

function fixtureNameSet(fixtureNames: FixtureNameSource): ReadonlySet<string> {
  if (fixtureNames instanceof Set) {
    return fixtureNames;
  }
  if (fixtureNames instanceof Map) {
    return new Set(fixtureNames.keys());
  }
  if (Array.isArray(fixtureNames)) {
    return new Set(fixtureNames);
  }
  return new Set(Object.keys(fixtureNames));
}

export function resolveFixtureRoute(
  hash: string,
  fixtureNames: FixtureNameSource,
): FixtureResolution {
  const name = parseFixtureRoute(hash);
  if (name === null) {
    return { status: 'idle', name: null };
  }
  const names = fixtureNameSet(fixtureNames);
  if (names.has(name)) {
    return { status: 'found', name };
  }
  return { status: 'unknown', name };
}
