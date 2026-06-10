import { ref } from "vue";

const messages = {
  en: {
    title: "PortKill",
    common: "Common ports",
    others: "Other ports",
    empty: "No listening ports",
    kill: "Kill",
    refresh: "Refresh",
    killFailed: "Failed to kill process",
    loadFailed: "Failed to list ports",
    bringToFront: "Bring app to front",
    focusFailed: "No app with windows found for this process",
  },
  zh: {
    title: "PortKill",
    common: "常用端口",
    others: "其他端口",
    empty: "没有正在监听的端口",
    kill: "结束",
    refresh: "刷新",
    killFailed: "结束进程失败",
    loadFailed: "获取端口列表失败",
    bringToFront: "前置应用",
    focusFailed: "该进程没有可前置的窗口程序",
  },
  ja: {
    title: "PortKill",
    common: "よく使うポート",
    others: "その他のポート",
    empty: "リッスン中のポートはありません",
    kill: "終了",
    refresh: "更新",
    killFailed: "プロセスの終了に失敗しました",
    loadFailed: "ポート一覧の取得に失敗しました",
    bringToFront: "アプリを前面に表示",
    focusFailed: "前面に表示できるアプリがありません",
  },
};

export const LOCALES = [
  { value: "en", label: "EN" },
  { value: "zh", label: "中" },
  { value: "ja", label: "日" },
];

const saved = localStorage.getItem("portkill-locale");
export const locale = ref(saved && messages[saved] ? saved : "en");

export function setLocale(value) {
  if (messages[value]) {
    locale.value = value;
    localStorage.setItem("portkill-locale", value);
  }
}

export function t(key) {
  return messages[locale.value][key] ?? messages.en[key] ?? key;
}
