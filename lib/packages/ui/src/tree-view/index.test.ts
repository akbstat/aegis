import { describe, expect, it } from 'vitest';

import { SimpleTreeView, TreeItem } from './index';

describe('@aegis/ui/tree-view', () => {
  it('re-exports the tree primitives', () => {
    expect(SimpleTreeView).toBeDefined();
    expect(TreeItem).toBeDefined();
  });
});
