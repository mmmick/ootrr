import { init_token, get, post, q } from "./api.js";
import { h, fill, $ } from "./dom.js";

const view = { locked: false, done: false };
let last = null;
let seen = new Set();
let seq = 0;

function load_view() {
  try {
    const v = JSON.parse(localStorage.getItem("ootrr.now.view") || "{}");
    view.locked = !!v.locked;
    view.done = !!v.done;
  } catch {
    view.locked = false;
  }
  $("#locked").checked = view.locked;
  $("#done").checked = view.done;
}

function save_view() {
  try {
    localStorage.setItem("ootrr.now.view", JSON.stringify(view));
  } catch {
    view.done = !!view.done;
  }
}

function live_chip(l) {
  const el = $("#live");
  el.classList.toggle("on", !!l.on);
  el.classList.toggle("off", !l.on);
  if (l.on) el.textContent = l.child === true ? "Live, child" : l.child === false ? "Live, adult" : "Live";
  else el.textContent = l.msg ? `Not live: ${l.msg}` : "Not live";
}

let last_key = "";

function last_drop(l) {
  const el = $("#last");
  if (!l) {
    fill(el);
    last_key = "";
    return;
  }
  const key = `${l.area}|${l.label}|${l.item}`;
  fill(el, "Last: ", h("b", null, l.item || "nothing"), ` from ${l.area}: ${l.label}`);
  el.title = el.textContent;
  if (last_key && key !== last_key) {
    el.classList.remove("fresh");
    void el.offsetWidth;
    el.classList.add("fresh");
  }
  last_key = key;
}

let bar_key = "";

function spoiler_bar(s) {
  const key = JSON.stringify(s);
  if (key === bar_key) return;
  bar_key = key;
  const el = $("#spoiler");
  el.classList.toggle("err", !!s.error);
  const input = h("input", { type: "text", placeholder: "Spoiler.txt path or mod folder, empty to find it next to the running game", value: s.pin || "" });
  const set = async () => {
    try {
      await post("/api/now/spoiler", { path: input.value });
    } catch (e) {
      el.classList.add("err");
      el.prepend(h("span", null, e.message));
    }
  };
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") set();
  });
  const label = s.error ? s.error : s.path ? `Seed ${s.seed} from ${s.path}` : "Looking for the spoiler next to the running game";
  fill(el, h("span", null, label), input, h("button", { type: "button", onclick: set }, "Use"));
}

async function toggle(spot) {
  if (spot.checked && !spot.manual) return;
  try {
    await post("/api/now/mark", { name: spot.name, on: !spot.checked });
  } catch (e) {
    $("#count").textContent = e.message;
  }
}

function row(spot) {
  const cls = spot.checked ? "done" : spot.open ? "open" : "locked";
  const fresh = spot.open && !spot.checked && last !== null && !seen.has(spot.name);
  const title = spot.checked ? (spot.manual ? "Checked by hand, click to undo" : "Checked in the game") : spot.sensed ? "Click to mark checked by hand" : "The game cannot tell this one, click when done";
  return h(
    "li",
    { class: fresh ? `${cls} fresh` : cls, title, onclick: () => toggle(spot) },
    h("span", null, spot.label),
    spot.age ? h("span", { class: `age ${spot.age}` }, `(${spot.age})`) : null,
    spot.sensed ? null : h("span", { class: "tag" }, "manual"),
    spot.maybe ? h("span", { class: "tag", title: "A shared flag says you may have it, click if done" }, "maybe") : null,
    h("span", { class: "k" }, spot.kind),
  );
}

function render(st) {
  live_chip(st.live || {});
  spoiler_bar(st.spoiler || {});
  const count = $("#count");
  if (st.total === undefined) {
    fill(count);
    last_drop(null);
    fill($("#areas"), h("div", { class: "empty" }, "No spoiler loaded yet. Start the game with the randomizer mod enabled, or give the path above."));
    return;
  }
  fill(count, h("b", null, String(st.open)), " open, ", String(st.checked), " of ", String(st.total), " checked");
  last_drop(st.last);
  const cards = [];
  for (const a of st.areas) {
    const spots = a.spots.filter((s) => (s.checked ? view.done : s.open || view.locked));
    if (!spots.length) continue;
    cards.push(
      h(
        "section",
        { class: a.area === st.active ? "area active" : "area", title: a.area === st.active ? "Your last check was here" : null },
        h("h2", null, h("span", null, a.area), h("span", { class: "n" }, String(a.open)), h("span", { class: "m" }, `${a.locked} locked, ${a.done} done`)),
        h("ul", null, spots.map(row)),
      ),
    );
  }
  fill($("#areas"), cards.length ? cards : h("div", { class: "empty" }, "Nothing is open right now."));
  const next = new Set();
  for (const a of st.areas) for (const s of a.spots) if (s.open && !s.checked) next.add(s.name);
  seen = next;
  last = st;
}

async function refresh() {
  const st = await get("/api/now/state");
  seq = Math.max(seq, st.seq || 0);
  render(st);
}

async function pump() {
  for (;;) {
    try {
      const b = await get(`/api/tracker/events?after=${q(seq)}`);
      const moved = (b.seq || 0) !== seq || b.reset;
      seq = b.seq || seq;
      if (moved) await refresh();
    } catch (e) {
      $("#live").textContent = e.message;
      $("#live").classList.add("off");
      await new Promise((r) => setTimeout(r, 2000));
    }
  }
}

async function main() {
  init_token();
  load_view();
  for (const k of ["locked", "done"]) {
    $(`#${k}`).addEventListener("change", (e) => {
      view[k] = e.target.checked;
      save_view();
      if (last) render(last);
    });
  }
  try {
    await refresh();
  } catch (e) {
    fill($("#areas"), h("div", { class: "empty" }, e.message));
  }
  pump();
}

main();
