import { closeSync, mkdirSync, openSync, readFileSync, readSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { Host, MODS } from "../host.js";
import { remote_caller, remote_root, serve_entry } from "../remote.js";

const args = process.argv.slice(2);
const flag = (k, d) => {
  const i = args.indexOf(k);
  if (i >= 0) return args[i + 1];
  if (d === undefined) throw new Error("usage: node --experimental-wasm-jspi drop_build.mjs --wasm <ootrr.wasm> --game <romfs> --out <dir> [--host hookshot] [--seed 12345]");
  return d;
};
const game = flag("--game");
const out = flag("--out");
const seed = flag("--seed", "12345");
const host_kind = flag("--host", "hookshot");
const name = `randomizer-${seed}`;

const later = (f) => setImmediate(f);
const counts = { lists: 0, files: 0 };

function blob(path, start, end) {
  return {
    size: end - start,
    slice: (a, b) => blob(path, start + Math.max(0, a), start + Math.min(end - start, b)),
    arrayBuffer: async () => {
      const b = Buffer.alloc(end - start);
      const fd = openSync(path, "r");
      try {
        readSync(fd, b, 0, b.length, start);
      } finally {
        closeSync(fd);
      }
      return b.buffer.slice(b.byteOffset, b.byteOffset + b.length);
    },
  };
}

function file_entry(path) {
  return { name: basename(path), isDirectory: false, isFile: true, file: (ok) => { counts.files++; later(() => ok(blob(path, 0, statSync(path).size))); } };
}

function dir_entry(path) {
  return {
    name: basename(path),
    isDirectory: true,
    isFile: false,
    createReader() {
      const all = readdirSync(path, { withFileTypes: true }).map((e) => (e.isDirectory() ? dir_entry(join(path, e.name)) : file_entry(join(path, e.name))));
      let at = 0;
      counts.lists++;
      return { readEntries: (ok) => later(() => { ok(all.slice(at, at + 100)); at += 100; }) };
    },
    getDirectory(p, _o, ok, bad) {
      const full = join(path, ...p.split("/"));
      later(() => (statSync(full, { throwIfNoEntry: false })?.isDirectory() ? ok(dir_entry(full)) : bad(Object.assign(new Error(`${p} not found`), { name: "NotFoundError" }))));
    },
    getFile(p, _o, ok, bad) {
      const full = join(path, ...p.split("/"));
      later(() => (statSync(full, { throwIfNoEntry: false })?.isFile() ? ok(file_entry(full)) : bad(Object.assign(new Error(`${p} not found`), { name: "NotFoundError" }))));
    },
  };
}

if (typeof WebAssembly.Suspending !== "function") throw new Error("run with node --experimental-wasm-jspi");
const h = new Host(new WebAssembly.Module(readFileSync(flag("--wasm"))), { async: true, log: (t) => process.stderr.write(t) });
const roots = new Map([[1, dir_entry(game)]]);
let remote = null;
remote = remote_caller((m) => serve_entry(roots.get(m.root), m).then(
  (data) => later(() => remote.answer({ fs_reply: m.fs_ask, ok: true, data })),
  (e) => later(() => remote.answer({ fs_reply: m.fs_ask, ok: false, message: String(e.message || e) })),
));

async function call(method, path, body) {
  const t = performance.now();
  const r = await h.request(method, path, body === undefined ? null : JSON.stringify(body));
  const text = new TextDecoder().decode(r.body);
  console.error(`${method} ${path} -> ${r.status} in ${Math.round(performance.now() - t)} ms`);
  if (r.status < 200 || r.status > 299) throw new Error(`${path}: ${text}`);
  return { data: JSON.parse(text), built: r.built };
}

const at = `/game/${basename(game)}`;
await h.mount_handle(at, remote_root(remote.call, 1, basename(game)));
const v = (await call("GET", `/api/validate?path=${encodeURIComponent(at)}`)).data;
if (!v.ok) throw new Error(`validate: ${v.problems.join(" ")}`);
console.error(`after validate: ${counts.lists} folders listed, ${counts.files} files opened`);
const g = (await call("POST", "/api/generate", { seed, settings: {}, spoiler: true })).data;
mkdirSync(out, { recursive: true });
writeFileSync(join(out, "seed.ootrr"), g.seed_text);
let files = null;
for (let round = 1; round <= 2; round++) {
  const b = (await call("POST", "/api/build", { seed_text: g.seed_text, game_dir: at, mod_dir: MODS, mod_name: name, host: host_kind, music: "off", cosmetic_seed: "" })).data;
  const job = await call("GET", `/api/jobs/${b.job_id}`);
  if (job.data.state !== "done") throw new Error(job.data.message);
  files = job.built;
  console.error(`round ${round}: done, ${files.filter(([, x]) => x).length} files, ${counts.lists} folders listed and ${counts.files} files opened so far`);
}
for (const [p, b] of files) {
  const to = join(out, "mods", p);
  if (b === null) mkdirSync(to, { recursive: true });
  else {
    mkdirSync(dirname(to), { recursive: true });
    writeFileSync(to, b);
  }
}
console.error("ok");
process.exit(0);
