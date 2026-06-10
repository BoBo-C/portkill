<script setup>
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { locale, setLocale, t, LOCALES } from "./i18n";

// Dev-server ports pinned to the top, in this order
const COMMON_PORTS = [3000, 3001, 4200, 5000, 5173, 5174, 8000, 8080, 8888, 9000];

const entries = ref([]);
const loading = ref(false);
const error = ref("");
const killingPids = ref(new Set());
// { x, y, entry } | null
const ctxMenu = ref(null);

let unlisten = null;

function fmtMem(kb) {
  if (!kb) return "";
  if (kb >= 1024 * 1024) return `${(kb / 1024 / 1024).toFixed(1)} GB`;
  if (kb >= 1024) return `${Math.round(kb / 1024)} MB`;
  return `${kb} KB`;
}

function openCtxMenu(event, entry) {
  // Menu is ~160px wide, ~36px tall; keep it inside the panel
  const x = Math.min(event.clientX, window.innerWidth - 170);
  const y = Math.min(event.clientY, window.innerHeight - 44);
  ctxMenu.value = { x, y, entry };
}

function closeCtxMenu() {
  ctxMenu.value = null;
}

async function bringToFront() {
  const entry = ctxMenu.value?.entry;
  closeCtxMenu();
  if (!entry) return;
  error.value = "";
  try {
    await invoke("focus_process", { pid: entry.pid });
  } catch {
    error.value = t("focusFailed");
  }
}

function onKeydown(e) {
  if (e.key === "Escape") closeCtxMenu();
}

async function refresh() {
  loading.value = true;
  error.value = "";
  try {
    entries.value = await invoke("list_ports");
  } catch (e) {
    error.value = `${t("loadFailed")}: ${e}`;
  } finally {
    loading.value = false;
  }
}

async function kill(entry) {
  killingPids.value.add(entry.pid);
  killingPids.value = new Set(killingPids.value);
  error.value = "";
  try {
    // Port passed along so Rust can revalidate (pid-reuse race guard)
    await invoke("kill_process", { pid: entry.pid, port: entry.port });
    // Small delay so the OS releases the socket before we re-query
    await new Promise((r) => setTimeout(r, 150));
    await refresh();
  } catch (e) {
    error.value = `${t("killFailed")}: ${e}`;
  } finally {
    killingPids.value.delete(entry.pid);
    killingPids.value = new Set(killingPids.value);
  }
}

const sorted = computed(() => {
  const rank = (e) => {
    const i = COMMON_PORTS.indexOf(e.port);
    return i === -1 ? Infinity : i;
  };
  return [...entries.value].sort(
    (a, b) => rank(a) - rank(b) || a.port - b.port || a.pid - b.pid
  );
});

const commonEntries = computed(() =>
  sorted.value.filter((e) => COMMON_PORTS.includes(e.port))
);
const otherEntries = computed(() =>
  sorted.value.filter((e) => !COMMON_PORTS.includes(e.port))
);

onMounted(async () => {
  refresh();
  window.addEventListener("keydown", onKeydown);
  // Rust emits this every time the tray panel is opened
  unlisten = await listen("panel-shown", () => {
    closeCtxMenu();
    refresh();
  });
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
  if (unlisten) unlisten();
});
</script>

<template>
  <main class="panel">
    <header class="header">
      <h1 class="title">{{ t("title") }}</h1>
      <div class="header-actions">
        <button
          class="icon-btn"
          :title="t('refresh')"
          :disabled="loading"
          @click="refresh"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" :class="{ spinning: loading }">
            <path d="M21 12a9 9 0 1 1-2.64-6.36" />
            <polyline points="21 3 21 9 15 9" />
          </svg>
        </button>
        <div class="lang-switch">
          <button
            v-for="l in LOCALES"
            :key="l.value"
            class="lang-btn"
            :class="{ active: locale === l.value }"
            @click="setLocale(l.value)"
          >
            {{ l.label }}
          </button>
        </div>
      </div>
    </header>

    <p v-if="error" class="error">{{ error }}</p>

    <div class="list">
      <template v-if="commonEntries.length">
        <p class="section-label">{{ t("common") }}</p>
        <div
          v-for="e in commonEntries"
          :key="`${e.pid}-${e.port}`"
          class="row common"
          @contextmenu.prevent="openCtxMenu($event, e)"
        >
          <span class="port">:{{ e.port }}</span>
          <span class="proc" :title="`${e.process_name} (pid ${e.pid}) ${e.address}`">
            {{ e.process_name }}
            <span class="pid">{{ e.pid }}</span>
          </span>
          <span class="mem">{{ fmtMem(e.memory_kb) }}</span>
          <button
            class="kill-btn"
            :disabled="killingPids.has(e.pid)"
            @click="kill(e)"
          >
            {{ t("kill") }}
          </button>
        </div>
      </template>

      <template v-if="otherEntries.length">
        <p class="section-label">{{ t("others") }}</p>
        <div
          v-for="e in otherEntries"
          :key="`${e.pid}-${e.port}`"
          class="row"
          @contextmenu.prevent="openCtxMenu($event, e)"
        >
          <span class="port">:{{ e.port }}</span>
          <span class="proc" :title="`${e.process_name} (pid ${e.pid}) ${e.address}`">
            {{ e.process_name }}
            <span class="pid">{{ e.pid }}</span>
          </span>
          <span class="mem">{{ fmtMem(e.memory_kb) }}</span>
          <button
            class="kill-btn"
            :disabled="killingPids.has(e.pid)"
            @click="kill(e)"
          >
            {{ t("kill") }}
          </button>
        </div>
      </template>

      <p v-if="!loading && !sorted.length && !error" class="empty">
        {{ t("empty") }}
      </p>
    </div>

    <!-- Right-click context menu -->
    <template v-if="ctxMenu">
      <div class="ctx-overlay" @click="closeCtxMenu" @contextmenu.prevent="closeCtxMenu"></div>
      <div class="ctx-menu" :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }">
        <button class="ctx-item" @click="bringToFront">
          {{ t("bringToFront") }}
        </button>
      </div>
    </template>
  </main>
</template>
