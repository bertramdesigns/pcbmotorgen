<script lang="ts">
  import { Tooltip } from "bits-ui";

  /**
   * Hoverable/focusable info superscript tag built on Bits UI's accessible
   * Tooltip primitive (floating-UI positioning with collision handling, so
   * overflow-hidden panel containers can never clip it). Shows `tip` text
   * and an optional `image` (imported asset URL, e.g. an inline SVG
   * illustration).
   */
  interface HelpTagProps {
    tip: string;
    image?: string;
    imageAlt?: string;
    label?: string;
  }

  let {
    tip,
    image,
    imageAlt = "",
    label = "More information",
  }: HelpTagProps = $props();
</script>

<Tooltip.Provider delayDuration={150}>
  <Tooltip.Root>
    <Tooltip.Trigger
      class="relative ml-0.5 cursor-help align-super text-slate-500 transition-colors hover:text-emerald-300 focus-visible:text-emerald-300 focus-visible:outline-none"
      aria-label={label}
    >
      <svg
        viewBox="0 0 12 12"
        class="inline-block h-2.5 w-2.5"
        fill="none"
        stroke="currentColor"
        stroke-width="1.3"
        aria-hidden="true"
      >
        <circle cx="6" cy="6" r="5.2" />
        <line x1="6" y1="5.4" x2="6" y2="8.8" stroke-linecap="round" />
        <circle cx="6" cy="3.3" r="0.55" fill="currentColor" stroke="none" />
      </svg>
    </Tooltip.Trigger>
    <Tooltip.Portal>
      <Tooltip.Content
        side="top"
        sideOffset={6}
        collisionPadding={8}
        class="z-50 w-56 rounded-md border border-slate-600 bg-slate-900 px-2.5 py-2 text-left text-[10px] font-normal leading-relaxed text-slate-200 shadow-lg shadow-black/40"
      >
        {tip}
        {#if image}
          <img
            src={image}
            alt={imageAlt}
            draggable="false"
            class="mt-1.5 w-full"
          />
        {/if}
      </Tooltip.Content>
    </Tooltip.Portal>
  </Tooltip.Root>
</Tooltip.Provider>
