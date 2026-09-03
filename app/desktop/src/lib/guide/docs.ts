/**
 * docs.ts — build-time bundling of the routing-plugin authoring guide
 * (kata bprp).
 * ============================================================================
 *
 * SINGLE SOURCE OF TRUTH: every byte of guide content below is imported
 * VERBATIM from the routing/export crates at build time via Vite `?raw`
 * imports. Nothing here forks, paraphrases, or duplicates contract content —
 * when the crate docs change, the in-app guide changes with them on the
 * next build.
 *
 * BUILD-TIME MISSING-DOC GUARANTEE: Vite raw imports are resolved by the
 * bundler; if any imported doc file is missing (renamed, moved, deleted),
 * `vite build` and `vite dev` FAIL with an unresolved-import error before
 * any bundle exists. There is no runtime fallback and none is wanted — a
 * guide silently missing its contract sections would be worse than a
 * failed build.
 *
 * Dev-server note: these files live outside `app/desktop` (the Vite root),
 * so `vite.config.ts` extends `server.fs.allow` to the repo root — the
 * modules are then served as `/@fs/...` raw imports in `vite dev` (the
 * Playwright webServer). `vite build` is unaffected (rollup resolves
 * relative imports regardless of fs.allow).
 */

import authoringGuideMd from "../../../../../crates/pcbmotorgen-routing/docs/routing-pattern-authoring.md?raw";
import apiReferenceMd from "../../../../../crates/pcbmotorgen-routing/docs/API.md?raw";
import exampleRunnerPy from "../../../../../crates/pcbmotorgen-export/scripts/pattern_runners/example_runner.py?raw";

import { markdownToHtml } from "./markdown";

export interface GuideTab {
  /** Stable tab id (also used as the DOM key). */
  id: string;
  /** Short tab label. */
  label: string;
  /** Accessible name of the tab panel content (first heading). */
  title: string;
  /** Pre-rendered, escaped HTML for the tab body (safe for {@html}). */
  html: string;
}

/**
 * The reference Python runner, bundled VERBATIM as the guide's worked
 * example. It is the same file the authoring guide points at
 * (`scripts/pattern_runners/example_runner.py`), displayed inside a fenced
 * code block — the only glue here is the framing sentence and the path
 * label, not any contract content.
 */
function workedExampleHtml(): string {
  const md = [
    "# Worked example — reference Python runner",
    "",
    "The reference runner below is bundled verbatim from",
    "`crates/pcbmotorgen-export/scripts/pattern_runners/example_runner.py`.",
    "It is a fully valid, minimal plugin: copy it and grow your generator",
    "from there. Note the two modes — the strict stdin → stdout",
    "`RoutingResult` contract, and the optional `--metadata` block.",
    "",
    "```python",
    exampleRunnerPy.replace(/\s+$/, ""),
    "```",
  ].join("\n");
  return markdownToHtml(md);
}

function buildTabs(): GuideTab[] {
  return [
    {
      id: "authoring",
      label: "Authoring guide",
      title: "Routing-Pattern Plugin Authoring Guide",
      html: markdownToHtml(authoringGuideMd),
    },
    {
      id: "api",
      label: "API reference",
      title: "pcbmotorgen-routing API",
      html: markdownToHtml(apiReferenceMd),
    },
    {
      id: "example",
      label: "Worked example",
      title: "Worked example — reference Python runner",
      html: workedExampleHtml(),
    },
  ];
}

/** The bundled guide tabs, rendered once at module load. */
export const GUIDE_TABS: GuideTab[] = buildTabs();

/**
 * The verbatim source documents (raw markdown / python), exposed for tests
 * and debugging — the UI renders only `GUIDE_TABS`.
 */
export const GUIDE_SOURCES = {
  authoringGuideMd,
  apiReferenceMd,
  exampleRunnerPy,
} as const;
