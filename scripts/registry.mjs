import { readFile, appendFile } from 'node:fs/promises';
const local = JSON.parse(await readFile('target/npm/manifest.json', 'utf8'));
const response = await fetch(`https://registry.npmjs.org/${local.name}/${local.version}`);
if (response.status === 404 && process.argv[2] === 'check') {
  await appendFile(process.env.GITHUB_OUTPUT, 'exists=false\n');
} else {
  if (!response.ok) throw new Error(`Registry returned ${response.status}`);
  const remote = await response.json();
  if (remote.dist.integrity !== local.integrity) throw new Error('Published package differs from the verified artifact');
  if (process.argv[2] === 'check') await appendFile(process.env.GITHUB_OUTPUT, 'exists=true\n');
  console.log(`Registry integrity matches ${local.name}@${local.version}`);
}
