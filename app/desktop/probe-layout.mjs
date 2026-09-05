import { chromium } from "@playwright/test";

const browser = await chromium.launch({ channel: "chrome" });
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
page.on("pageerror", (err) => console.log("[pageerror]", err.message));
await page.goto("http://localhost:1420/");
await page.waitForSelector("footer");
await page.waitForTimeout(800);

const data = await page.evaluate(() => {
  const probe = (el) => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    return {
      tag: el.tagName.toLowerCase(),
      id: el.id || null,
      cls: (el.className || "").toString().slice(0, 60),
      y: Math.round(r.y),
      h: Math.round(r.height),
      display: cs.display,
      overflow: cs.overflowY,
      flex: cs.flex,
      minH: cs.minHeight,
    };
  };
  const main = document.querySelector("main");
  const aside = document.querySelector("aside[aria-label='Persistent design reflection']");
  const saRoot = aside?.querySelector("[data-scroll-area-root]");
  const saViewport = aside?.querySelector("[data-scroll-area-viewport]");
  const saContent = aside?.querySelector("[data-scroll-area-content]");
  const grid = main?.querySelector(":scope > div.grid");
  const panelsWrap = grid?.querySelector(":scope > div.min-w-0");
  const panelDesign = document.querySelector("#panel-design");
  const designScroll = document.querySelector("#design-settings-scroll");
  const footer = document.querySelector("footer");
  const firstCard = document.querySelector(
    "aside [data-scroll-area-content] > :first-child",
  );
  return {
    main: probe(main),
    header: probe(main?.querySelector("header")),
    grid: probe(grid),
    aside: probe(aside),
    saRoot: probe(saRoot),
    saViewport: probe(saViewport),
    saContent: probe(saContent),
    firstCard: probe(firstCard),
    panelsWrap: probe(panelsWrap),
    panelDesign: probe(panelDesign),
    designScroll: probe(designScroll),
    footer: probe(footer),
    scrollHeights: {
      mainScrollH: main?.scrollHeight,
      bodyScrollH: document.body.scrollHeight,
    },
  };
});
for (const [k, v] of Object.entries(data)) {
  console.log(k, JSON.stringify(v));
}
await browser.close();
