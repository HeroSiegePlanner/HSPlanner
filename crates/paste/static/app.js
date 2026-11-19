// Loaded in <head> without `defer`, so the class is set before the body paints.
document.documentElement.classList.add("js");

/**
 * @param {string} text
 * @returns {Promise<void>}
 */
async function copyText(text) {
  if (navigator.clipboard && window.isSecureContext) {
    await navigator.clipboard.writeText(text);
    return;
  }
  // Plain-http addresses (a Pi on the local network) have no clipboard API.
  const area = document.createElement("textarea");
  area.value = text;
  area.setAttribute("readonly", "");
  area.className = "offscreen";
  document.body.append(area);
  area.select();
  const copied = document.execCommand("copy");
  area.remove();
  if (!copied) throw new Error("copy was refused");
}

/**
 * Marks one element of a group as the active one.
 * @param {Iterable<Element>} group
 * @param {(element: Element) => boolean} isActive
 * @param {string} [stateAttribute]
 */
function activate(group, isActive, stateAttribute) {
  for (const element of group) {
    const active = isActive(element);
    element.classList.toggle("active", active);
    if (stateAttribute && element.hasAttribute(stateAttribute)) {
      element.setAttribute(stateAttribute, String(active));
    }
  }
}

/** @param {HTMLElement} button */
async function copyFrom(button) {
  const text =
    button.dataset.copy === "link"
      ? location.href
      : document.getElementById("build-code").value;
  const label = button.textContent;
  try {
    await copyText(text);
    button.textContent = "Copied";
  } catch {
    button.textContent = "Copy failed";
  }
  setTimeout(() => {
    button.textContent = label;
  }, 1600);
}

/** @param {Element} tab */
function openTab(tab) {
  const id = tab.getAttribute("data-tab");
  activate(document.querySelectorAll("[data-tab]"), (other) => other === tab, "aria-selected");
  activate(
    document.querySelectorAll("[data-panel]"),
    (panel) => panel.getAttribute("data-panel") === id,
    "data-panel-state",
  );
}

document.addEventListener("DOMContentLoaded", () => {
  const wanted = location.hash.slice(1);
  const tab = [...document.querySelectorAll("[data-tab]")].find(
    (candidate) => candidate.getAttribute("data-tab") === wanted,
  );
  if (tab) openTab(tab);
});

// Choosing a loadout reloads the page with it; without scripts the form has a button.
document.addEventListener("change", (event) => {
  const select = /** @type {Element} */ (event.target).closest("select[data-submit]");
  if (select) /** @type {HTMLSelectElement} */ (select).form.submit();
});

// Item cards on the equipment board float beside the pointer, like the planner's tooltips.
const CARD_OFFSET = 16;
const CARD_EDGE = 8;
const HOVER_SLOT = "[data-hover] [data-pick]";

/** @type {Element | null} The board whose item card is showing. */
let floating = null;

function hideCard() {
  if (!floating) return;
  floating.querySelector(".pick-panes").classList.remove("open");
  activate(floating.querySelectorAll("[data-pick]"), () => false);
  floating = null;
}

/**
 * @param {HTMLElement} panes
 * @param {number} x
 * @param {number} y
 */
function placeCard(panes, x, y) {
  const { width, height } = panes.getBoundingClientRect();
  const right = x + CARD_OFFSET;
  const left = right + width > innerWidth - CARD_EDGE ? x - CARD_OFFSET - width : right;
  const top = Math.min(y + CARD_OFFSET, innerHeight - height - CARD_EDGE);
  panes.style.left = `${Math.max(CARD_EDGE, left)}px`;
  panes.style.top = `${Math.max(CARD_EDGE, top)}px`;
}

/**
 * Shows a slot's card beside a point; without one (keyboard), beside the slot itself.
 * @param {Element} slot
 * @param {{ x: number, y: number }} [point]
 */
function showCard(slot, point) {
  const board = slot.closest("[data-hover]");
  const panes = /** @type {HTMLElement} */ (board.querySelector(".pick-panes"));
  const index = slot.getAttribute("data-pick");
  activate(board.querySelectorAll("[data-pick]"), (other) => other === slot);
  activate(panes.children, (pane) => pane.getAttribute("data-pane") === index);
  panes.classList.add("open");
  floating = board;
  const box = slot.getBoundingClientRect();
  placeCard(panes, point ? point.x : box.right - CARD_OFFSET, point ? point.y : box.top);
}

document.addEventListener("pointerover", (event) => {
  if (event.pointerType !== "mouse") return;
  const slot = /** @type {Element} */ (event.target).closest(HOVER_SLOT);
  if (slot) showCard(slot, { x: event.clientX, y: event.clientY });
  else hideCard();
});

document.addEventListener("pointermove", (event) => {
  if (event.pointerType !== "mouse" || !floating) return;
  placeCard(floating.querySelector(".pick-panes"), event.clientX, event.clientY);
});

document.addEventListener("focusin", (event) => {
  const slot = /** @type {Element} */ (event.target).closest(HOVER_SLOT);
  if (slot && slot.matches(":focus-visible")) showCard(slot);
});

document.addEventListener("focusout", hideCard);
document.addEventListener("scroll", hideCard, { passive: true });
document.documentElement.addEventListener("pointerleave", hideCard);

document.addEventListener("click", (event) => {
  const target = /** @type {Element} */ (event.target);
  const copy = target.closest("[data-copy]");
  if (copy) {
    copyFrom(/** @type {HTMLElement} */ (copy));
    return;
  }
  const tab = target.closest("[data-tab]");
  if (tab) {
    openTab(tab);
    // Keeps the tab across the reload a loadout choice makes, and in copied links.
    history.replaceState(null, "", `#${tab.getAttribute("data-tab")}`);
    return;
  }
  const row = target.closest("[data-pick]");
  const picker = row && row.closest(".picker");
  if (!row || !picker) {
    hideCard();
    return;
  }
  if (picker.hasAttribute("data-hover")) {
    // A tap has no hover; a keyboard press has no pointer position.
    showCard(row, event.detail ? { x: event.clientX, y: event.clientY } : undefined);
    return;
  }
  const index = row.getAttribute("data-pick");
  activate(picker.querySelectorAll("[data-pick]"), (other) => other === row, "aria-pressed");
  activate(
    picker.querySelectorAll("[data-pane]"),
    (pane) => pane.getAttribute("data-pane") === index,
    "data-pane-state",
  );
  if (window.matchMedia("(max-width: 900px)").matches) {
    const pane = picker.querySelector(".pick-pane.active");
    if (pane) pane.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }
});
