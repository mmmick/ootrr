import { defaults, sanitize } from "./rules.js";

const KEY = "ootrr.v1";
const RKEY = "ootrr.result";

export const state = {
  schema: null,
  info: null,
  vals: {},
  seed: "",
  plando: [],
  paths: { game: "", update: "", mod: "", name: "My Seed", host: "eden" },
  include_update: false,
  cosmetic: { music: "off", sfx: "off", colors: "off", seed: "", title: true, parts: {}, groups: {} },
  game_check: null,
  mod_check: null,
  result: null,
  hide_spoiler: false,
  spoiler_log: true,
};

const subs = new Set();

export function on_change(fn) {
  subs.add(fn);
  return () => subs.delete(fn);
}

function put(store, key, v) {
  try {
    store.setItem(key, JSON.stringify(v));
  } catch {
    return;
  }
}

function take(store, key) {
  try {
    return JSON.parse(store.getItem(key) || "null");
  } catch {
    return null;
  }
}

export function save() {
  put(localStorage, KEY, { vals: state.vals, seed: state.seed, plando: state.plando, paths: state.paths, cosmetic: state.cosmetic, hide_spoiler: state.hide_spoiler, spoiler_log: state.spoiler_log, include_update: state.include_update });
  put(sessionStorage, RKEY, state.result);
}

export function touch() {
  for (const f of subs) f();
  save();
}

export function restore(schema) {
  state.schema = schema;
  const s = take(localStorage, KEY);
  state.vals = sanitize(schema, s && s.vals);
  if (s && typeof s.seed === "string") state.seed = s.seed.slice(0, 100);
  if (s && typeof s.hide_spoiler === "boolean") state.hide_spoiler = s.hide_spoiler;
  if (s && typeof s.spoiler_log === "boolean") state.spoiler_log = s.spoiler_log;
  if (s && typeof s.include_update === "boolean") state.include_update = s.include_update;
  if (s && Array.isArray(s.plando)) {
    state.plando = s.plando
      .filter((p) => p && typeof p.location === "string" && typeof p.item === "string")
      .slice(0, 200)
      .map((p) => ({ location: p.location.slice(0, 120), item: p.item.slice(0, 120) }));
  }
  if (s && s.paths && typeof s.paths === "object") {
    for (const k of ["game", "update", "mod", "name", "host"]) {
      if (typeof s.paths[k] === "string") state.paths[k] = s.paths[k].slice(0, 1000);
    }
    if (!["eden", "hookshot", "longshot", "nx2navi"].includes(state.paths.host)) state.paths.host = "eden";
  }
  if (s && s.cosmetic && typeof s.cosmetic === "object") {
    if (["off", "grouped", "anywhere"].includes(s.cosmetic.music)) state.cosmetic.music = s.cosmetic.music;
    if (["off", "grouped", "anywhere"].includes(s.cosmetic.sfx)) state.cosmetic.sfx = s.cosmetic.sfx;
    if (["off", "custom", "random", "shared"].includes(s.cosmetic.colors)) state.cosmetic.colors = s.cosmetic.colors;
    if (typeof s.cosmetic.seed === "string") state.cosmetic.seed = s.cosmetic.seed.slice(0, 64);
    if (typeof s.cosmetic.title === "boolean") state.cosmetic.title = s.cosmetic.title;
    if (s.cosmetic.parts && typeof s.cosmetic.parts === "object") {
      const known = new Set(((state.info && state.info.color_parts) || []).map((p) => p.id));
      for (const [k, v] of Object.entries(s.cosmetic.parts).slice(0, 64)) {
        if (known.has(k) && typeof v === "string" && /^(original|random|#[0-9a-f]{6})$/.test(v)) state.cosmetic.parts[k] = v;
      }
    }
    if (s.cosmetic.groups && typeof s.cosmetic.groups === "object") {
      const known = new Set(((state.info && state.info.color_parts) || []).map((p) => p.group));
      for (const [k, v] of Object.entries(s.cosmetic.groups).slice(0, 16)) {
        if (known.has(k) && ["original", "random", "custom"].includes(v)) state.cosmetic.groups[k] = v;
      }
    }
  }
  const r = take(sessionStorage, RKEY);
  if (r && typeof r.seed_text === "string") state.result = r;
  if (!state.vals) state.vals = defaults(schema);
}

// Colour parts come from the server, grouped by tunic or kind.
export const SOLO = "Other";
export function color_groups() {
  const groups = new Map();
  for (const p of (state.info && state.info.color_parts) || []) {
    if (!groups.has(p.group)) groups.set(p.group, []);
    groups.get(p.group).push(p);
  }
  return groups;
}

// The part colours a build is sent, used only in the custom colour mode: whole groups by
// their choice, picked groups and single parts by their own.
export function color_picks(cos) {
  const out = {};
  if (cos.colors !== "custom") return out;
  const gs = cos.groups || {};
  const own = cos.parts || {};
  for (const [g, ps] of color_groups()) {
    const mode = g === SOLO ? "custom" : gs[g] || "original";
    for (const p of ps) {
      if (mode === "random") out[p.id] = "random";
      else if (mode === "custom" && own[p.id]) out[p.id] = own[p.id];
    }
  }
  return out;
}
