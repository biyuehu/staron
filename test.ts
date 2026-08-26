import { Block, gate } from './src/index'

await gate({
  clientId: 'Ov23liCJaZwQGcVvHs9X',
  owner: 'biyuehu',
  repo: 'staron',
  onFail: Block('Please star biyuehu/staron on GitHub to use this tool.'),
  onSuccess: (ctx) => console.log('success:', ctx),
  skipStarForOwner: true
})
