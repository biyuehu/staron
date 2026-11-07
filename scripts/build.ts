import { execSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import esbuild from 'esbuild'

execSync('bunx tsc --emitDeclarationOnly', { stdio: 'inherit' })

await esbuild.build({
  entryPoints: ['src/index.ts'],
  outfile: 'dist/index.js',
  bundle: true,
  format: 'esm',
  platform: 'node',
  target: 'node18',
  mainFields: ['module', 'main']
})

await esbuild.build({
  entryPoints: ['src/index.ts'],
  outfile: 'dist/index.cjs',
  bundle: true,
  format: 'cjs',
  platform: 'node',
  target: 'node18',
  mainFields: ['module', 'main']
})

writeFileSync('dist/index.d.cts', "export * from './index.js';\n")
