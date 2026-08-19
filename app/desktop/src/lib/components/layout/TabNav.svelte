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

<nav aria-label="Motor workflow">
  <div class="flex gap-1" role="tablist" aria-label="Motor workflow tabs">
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
          ? "rounded-md bg-emerald-500/20 px-4 py-2 text-sm font-semibold text-emerald-200 ring-1 ring-emerald-500/60"
          : "rounded-md px-4 py-2 text-sm text-slate-400 transition hover:bg-slate-800 hover:text-slate-100"}
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