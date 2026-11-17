import { $, h, fill, toast } from "./dom.js";
import { t_mark } from "./tracker_api.js";
import { ts } from "./tracker_store.js";

const LIFE = 15000;
const MAX = 6;
const shown = [];

function drop(id) {
  const i = shown.findIndex((x) => x.l.id === id);
  if (i < 0) return;
  clearTimeout(shown[i].timer);
  shown.splice(i, 1);
  paint();
}

async function undo(id) {
  drop(id);
  try {
    await t_mark(id, false);
    toast("Undone. The check is open again.");
  } catch (e) {
    toast(e.message);
  }
}

function from() {
  return ts.state && ts.state.source && ts.state.source.live ? "the game" : "your save";
}

function card(l) {
  const where = [l.label, l.area ? `(${l.area})` : null].filter(Boolean).join(" ");
  return h(
    "div",
    { class: "nb" },
    h("h3", null, l.item || `New from ${from()}`),
    h("p", { class: "where muted" }, where),
    h(
      "div",
      { class: "row" },
      h("button", { type: "button", class: "btn small", onclick: () => undo(l.id) }, "Undo"),
      h("button", { type: "button", class: "btn small", onclick: () => drop(l.id) }, "Close"),
    ),
  );
}

function paint() {
  const box = $("#news");
  if (!box) return;
  fill(box, shown.slice(-MAX).map((x) => card(x.l)));
}

export function announce(locs) {
  for (const l of locs) {
    if (shown.some((x) => x.l.id === l.id)) continue;
    shown.push({ l, timer: setTimeout(() => drop(l.id), LIFE) });
  }
  paint();
}
