<script lang="ts">
  import type { KicadConnection, BoardDiagnostics } from "../../types";

  let {
    connection,
    connected,
    boardName,
    copperLayers,
    diagnostics,
    connecting,
    refreshingDiag,
    onConnect,
    onRefresh,
  }: {
    connection: KicadConnection | null;
    connected: boolean;
    boardName: string;
    copperLayers: number;
    diagnostics: BoardDiagnostics | null;
    connecting: boolean;
    refreshingDiag: boolean;
    onConnect: () => void;
    onRefresh: () => void;
  } = $props();
</script>

<div class="space-y-3">
  <!-- Board info when connected -->
  {#if connected}
    <div class="text-xs text-slate-400 font-mono">
      Board: <span class="text-slate-200">{boardName}</span>
      · {copperLayers} copper layer{copperLayers === 1 ? "" : "s"}
      {#if diagnostics && diagnostics.board_x_max_mm > 0}
        · {Math.round(diagnostics.board_x_max_mm - diagnostics.board_x_min_mm)} mm
          × {Math.round(diagnostics.board_y_max_mm - diagnostics.board_y_min_mm)} mm
      {/if}
    </div>
  {:else if connection && !connection.connected}
    <div class="text-xs text-slate-500">
      No KiCad board open. Connect to a running KiCad 10 instance to write coils.
    </div>
  {/if}

  <!-- Diagnostics card (from get_board_diagnostics) -->
  {#if diagnostics}
    <div
      class="rounded-md border border-slate-700 bg-slate-900/40 px-3 py-2 text-xs text-slate-300"
    >
      <div class="font-semibold text-slate-200 mb-1">Board diagnostics</div>
      <div class="font-mono text-[11px] leading-snug text-slate-400">
        board_name: <span class="text-slate-200">{diagnostics.board_name}</span><br />
        copper_layer_count: <span class="text-slate-200">{diagnostics.copper_layer_count}</span><br />
        {#if diagnostics.board_x_max_mm > 0}
          edge cuts: <span class="text-slate-200">
            {diagnostics.board_x_min_mm.toFixed(1)}…{diagnostics.board_x_max_mm.toFixed(1)} mm
            × {diagnostics.board_y_min_mm.toFixed(1)}…{diagnostics.board_y_max_mm.toFixed(1)} mm
          </span><br />
        {:else}
          edge cuts: <span class="text-slate-500">(not yet queryable in KiCad 10 IPC)</span><br />
        {/if}
        net_classes: <span class="text-slate-200">{diagnostics.available_net_classes.length}</span>
      </div>
    </div>
  {/if}

  <!-- Connection action buttons -->
  <div class="flex flex-wrap items-center gap-2">
    <button
      type="button"
      onclick={onConnect}
      disabled={connecting}
      class="rounded-md border border-slate-600 bg-slate-700/60 px-3 py-1.5 text-xs font-medium text-slate-100 transition hover:bg-slate-600/60 disabled:cursor-not-allowed disabled:opacity-50"
    >
      {connecting ? "Connecting…" : connected ? "Reconnect" : "Connect to KiCad"}
    </button>

    <button
      type="button"
      onclick={onRefresh}
      disabled={refreshingDiag}
      class="rounded-md border border-slate-600 bg-slate-700/60 px-3 py-1.5 text-xs font-medium text-slate-100 transition hover:bg-slate-600/60 disabled:cursor-not-allowed disabled:opacity-50"
    >
      {refreshingDiag ? "Refreshing…" : "Refresh Diagnostics"}
    </button>
  </div>
</div>