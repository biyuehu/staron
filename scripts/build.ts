import { execSync } from 'node:child_process'
import { readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import esbuild from 'esbuild'

rmSync('dist', { recursive: true, force: true })

execSync('bunx tsc --emitDeclarationOnly', { stdio: 'inherit' })

for (const file of readdirSync('dist')) {
  if (file.endsWith('.d.ts')) {
    const filePath = path.join('dist', file)
    const content = readFileSync(filePath, 'utf-8').replace(/from '(\.[^']+?)(\.js)?'/g, "from '$1.js'")
    writeFileSync(filePath, content)
  }
}

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
