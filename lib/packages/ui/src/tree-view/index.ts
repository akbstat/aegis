// Thin re-export of `@mui/x-tree-view@9.16.0`. Consumers compose the tree
// primitives directly; the UI package ships no tree wrapper. The dependency
// is a regular (not peer) dependency so the tree resolves the same
// `@mui/material` instance the rest of the app uses — a second instance
// would put the tree on a different theme context.
export * from '@mui/x-tree-view';
