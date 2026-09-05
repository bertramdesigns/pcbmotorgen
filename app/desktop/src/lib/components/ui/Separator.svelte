<script lang="ts">
  import { Separator } from "bits-ui";

  /**
   * Thin visual divider built on Bits UI's Separator primitive. Decorative
   * by default (renders `role="none"` + `aria-hidden`) so it replaces the
   * app's previous `border-t` dividers 1:1 without adding anything to the
   * accessibility tree.
   */
  interface SeparatorProps {
    orientation?: "horizontal" | "vertical"; // default "horizontal"
    /** Visual-only dividers in this app; hidden from the a11y tree by default. */
    decorative?: boolean; // default true
    /** Color/spacing utilities; the sizing defaults below stay. */
    class?: string; // default "bg-slate-700"
  }

  let {
    orientation = "horizontal",
    decorative = true,
    class: cls = "bg-slate-700",
  }: SeparatorProps = $props();
</script>

<Separator.Root
  {orientation}
  {decorative}
  class="{orientation === 'horizontal' ? 'h-px w-full' : 'w-px h-full'} {cls}"
/>
