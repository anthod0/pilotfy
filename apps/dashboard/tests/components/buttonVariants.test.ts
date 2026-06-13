import { expect, test } from 'vitest';
import { buttonVariants } from '../../src/lib/components/ui/button/button.svelte';

test('ghost button variant is borderless by default', () => {
  const classes = buttonVariants({ variant: 'ghost' });

  expect(classes).not.toMatch(/(?:^|\s)border(?:\s|$)/);
  expect(classes).not.toContain('border-transparent');
});
