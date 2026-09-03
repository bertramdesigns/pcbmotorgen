<script lang="ts">
  import type { ProjectStore } from "../../stores/project.svelte";
  import ProjectControls from "./ProjectControls.svelte";
  import StatusIndicator from "./StatusIndicator.svelte";

  let {
    projects,
    loading,
  }: {
    projects: ProjectStore;
    loading: boolean;
  } = $props();
</script>

<!--
  Zone 1 of the slim top bar: the native macOS traffic lights (titleBarStyle:
  Overlay) float over its left edge, with 16px clearance before the app name.
  The row is 36px tall; trafficLightPosition y=18 centers the lights in it
  (tao semantics: button top = y − natural origin.y ≈ 7pt on macOS 26, button
  frame 14pt → center = 18 − 7 + 7 = 18). The whole row is a window drag
  region (drag + double-click maximize); the tab strip below is not.
-->
<div
  class="grid h-9 grid-cols-[1fr_auto_1fr] items-center"
  data-tauri-drag-region="deep"
>
  <div class="pl-[92px]">
    <span
      class="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400"
    >
      pcbmotorgen
    </span>
  </div>
  <div class="min-w-0 justify-self-center">
    <ProjectControls {projects} />
  </div>
  <div class="justify-self-end pr-4">
    <StatusIndicator {loading} />
  </div>
</div>
