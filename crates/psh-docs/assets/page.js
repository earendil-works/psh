// The page's own behavior: the light/dark toggle, the chapter menu on narrow
// screens, and the table of contents' current section. The chosen color scheme is set
// on <html> before the first paint by a line in the template's <head>.
const root = document.documentElement;
const KEY = "psh-color-scheme";

document.querySelector(".scheme")?.addEventListener("click", () => {
  const dark = root.dataset.colorScheme
    ? root.dataset.colorScheme === "dark"
    : matchMedia("(prefers-color-scheme: dark)").matches;
  const next = dark ? "light" : "dark";
  // Back to following the system when the choice matches it.
  if (matchMedia(`(prefers-color-scheme: ${next})`).matches) {
    delete root.dataset.colorScheme;
    try { localStorage.removeItem(KEY); } catch {}
  } else {
    root.dataset.colorScheme = next;
    try { localStorage.setItem(KEY, next); } catch {}
  }
});

const menu = document.querySelector(".menu-button");
menu?.addEventListener("click", () => {
  const open = document.body.classList.toggle("nav-open");
  menu.setAttribute("aria-expanded", String(open));
});

// The current section: the last heading above the top quarter of the window.
const links = [...document.querySelectorAll("aside.toc a")];
const heads = links.map((a) => document.getElementById(decodeURIComponent(a.hash.slice(1))));
let current = null;
function spy() {
  const line = innerHeight / 4;
  let i = heads.findIndex((h) => h && h.getBoundingClientRect().top > line);
  i = i === -1 ? heads.length - 1 : i - 1;
  const a = links[Math.max(0, i)];
  if (a === current) return;
  current?.removeAttribute("aria-current");
  a?.setAttribute("aria-current", "true");
  current = a;
}
if (links.length) {
  addEventListener("scroll", spy, { passive: true });
  spy();
}
