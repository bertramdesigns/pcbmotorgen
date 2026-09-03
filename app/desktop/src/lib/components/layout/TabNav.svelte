<script lang="ts">
  import type { TabId } from "../../ui";

  let {
    tabs,
    activeTab,
    statusFor,
    onSelect,
  }: {
    tabs: { id: TabId; label: string }[];
    activeTab: TabId;
    statusFor: (tab: TabId) => { label: string; className: string };
    onSelect: (tab: TabId) => void;
  } = $props();

  function handleTabKeydown(event: KeyboardEvent, currentTab: TabId): void {
    const currentIndex = tabs.findIndex((tab) => tab.id === currentTab);
    let nextIndex = currentIndex;
    if (event.key === "ArrowRight" || event.key === "ArrowDown") {
      nextIndex = (currentIndex + 1) % tabs.length;
    } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
      nextIndex = (currentIndex - 1 + tabs.length) % tabs.length;
    } else if (event.key === "Home") {
      nextIndex = 0;
    } else if (event.key === "End") {
      nextIndex = tabs.length - 1;
    } else {
      return;
    }

    event.preventDefault();
    onSelect(tabs[nextIndex].id);
    document.getElementById(`tab-${tabs[nextIndex].id}`)?.focus();
  }
</script>

<!--
  Zone 2 of the slim top bar: browser-style file tabs attached to the
  header's bottom divider. The active tab shares the page background and
  overlaps the divider by 1px (-mb-px), so it reads as "open" onto the
  content below.
-->
<nav aria-label="Motor workflow" class="px-3">
  <div
    class="flex items-end gap-1 border-b border-slate-800"
    role="tablist"
    aria-label="Motor workflow tabs"
  >
    {#each tabs as tab (tab.id)}
      {@const status = statusFor(tab.id)}
      <button
        id={`tab-${tab.id}`}
        type="button"
        role="tab"
        aria-controls={`panel-${tab.id}`}
        aria-selected={activeTab === tab.id}
        tabindex={activeTab === tab.id ? 0 : -1}
        class={activeTab === tab.id
          ? "-mb-px rounded-t-md border border-b-0 border-slate-800 bg-slate-900 px-3.5 py-1.5 text-sm font-semibold text-slate-100"
          : "-mb-px rounded-t-md border border-b-0 border-transparent px-3.5 py-1.5 text-sm text-slate-400 transition hover:bg-slate-800/60 hover:text-slate-200"}
        onclick={() => onSelect(tab.id)}
        onkeydown={(event) => handleTabKeydown(event, tab.id)}
      >
        {tab.label}
        <span class={`ml-2 text-[10px] font-normal uppercase tracking-wider ${status.className}`}>
          {status.label}
        </span>
      </button>
    {/each}
  </div>
</nav>