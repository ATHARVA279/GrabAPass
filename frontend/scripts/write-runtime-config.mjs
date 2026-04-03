import fs from 'node:fs';
import path from 'node:path';

const projectRoot = path.resolve(new URL('..', import.meta.url).pathname);
const outputFile = path.join(projectRoot, 'public', 'runtime-config.js');

const envFiles = [
  path.join(projectRoot, '.env.local'),
  path.join(projectRoot, '.env'),
  path.join(projectRoot, '..', '.env.local'),
  path.join(projectRoot, '..', '.env'),
];

for (const envFile of envFiles) {
  if (!fs.existsSync(envFile)) continue;

  for (const rawLine of fs.readFileSync(envFile, 'utf8').split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;

    const separatorIndex = line.indexOf('=');
    if (separatorIndex === -1) continue;

    const key = line.slice(0, separatorIndex).trim();
    if (!key || process.env[key] !== undefined) continue;

    let value = line.slice(separatorIndex + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }

    process.env[key] = value;
  }
}

const runtimeConfig = {
  GOOGLE_MAPS_API_KEY: process.env.GOOGLE_MAPS_API_KEY ?? '',
};

fs.writeFileSync(
  outputFile,
  `window.__runtimeConfig = ${JSON.stringify(runtimeConfig, null, 2)};\n`,
  'utf8'
);

console.log(`Wrote runtime config to ${path.relative(projectRoot, outputFile)}`);
