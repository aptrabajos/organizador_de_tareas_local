import { afterEach } from 'vitest';
import { cleanup } from '@solidjs/testing-library';

// Cleanup después de cada test
afterEach(() => {
  cleanup();
});
