(() => {
  if (window.top !== window || location.hostname !== "music.youtube.com") return;

  const INTERACTIVE =
    "a,button,input,textarea,select,video,[contenteditable],[draggable=true]," +
    "[role=button],[role=slider],[role=menuitem],[role=option],[role=tab],[role=link]," +
    "[role=checkbox],[role=switch],[role=listbox],[role=menu],tp-yt-paper-slider";

  const NAV_BAR_HEIGHT = 84;

  const mount = () => {
    const style = document.createElement("style");
    style.textContent = `
      html { background: #030303; scrollbar-width: none; --ytmusic-nav-bar-height: ${NAV_BAR_HEIGHT}px !important; }
      ::-webkit-scrollbar { display: none !important; width: 0 !important; height: 0 !important; }
      *, *::before, *::after { -webkit-user-select: none !important; user-select: none !important; }
      input, textarea, [contenteditable], [contenteditable] * {
        -webkit-user-select: text !important; user-select: text !important;
      }
    `;
    document.documentElement.append(style);

    const inInteractive = (path) =>
      path.some((el) => {
        if (!(el instanceof Element)) return false;
        if (el.matches(INTERACTIVE)) return true;
        const cursor = getComputedStyle(el).cursor;
        return cursor !== "auto" && cursor !== "default";
      });

    document.addEventListener(
      "mousedown",
      (e) => {
        if (e.button !== 0) return;
        if (e.clientX >= document.documentElement.clientWidth) return;
        if (inInteractive(e.composedPath())) return;
        e.preventDefault();
        const invoke = window.__TAURI_INTERNALS__?.invoke;
        if (!invoke) return;
        const maximize = e.detail === 2 && e.clientY < NAV_BAR_HEIGHT;
        invoke(maximize ? "toggle_maximize" : "start_drag").catch(() => {});
      },
      true,
    );
  };
  if (document.documentElement) mount();
  else document.addEventListener("DOMContentLoaded", mount);

  const $ = (s) => document.querySelector(s);
  const video = () => $("video");
  const click = (...selectors) => {
    for (const s of selectors) {
      const el = $(s);
      if (el) return el.click();
    }
  };

  const queueState = () => {
    try {
      return $("ytmusic-player-queue").queue.store.store.getState().queue;
    } catch {
      return {};
    }
  };

  window.__ytm = {
    play: () => {
      if (video()?.paused) click("ytmusic-player-bar #play-pause-button");
    },
    playPause: () => click("ytmusic-player-bar #play-pause-button"),
    next: () => click("ytmusic-player-bar .next-button"),
    prev: () => click("ytmusic-player-bar .previous-button"),
    like: () =>
      click(
        "ytmusic-player-bar #like-button-renderer #button-shape-like button",
        "ytmusic-player-bar #like-button-renderer .like",
      ),
    dislike: () =>
      click(
        "ytmusic-player-bar #like-button-renderer #button-shape-dislike button",
        "ytmusic-player-bar #like-button-renderer .dislike",
      ),
    toggleShuffle: () => click("ytmusic-player-bar .shuffle"),
    toggleRepeat: () => click("ytmusic-player-bar .repeat"),
    toggleMute: () => {
      const v = video();
      if (v) v.muted = !v.muted;
    },
    seek: (fraction) => {
      const v = video();
      if (v && Number.isFinite(v.duration)) v.currentTime = fraction * v.duration;
    },
  };

  const snapshot = () => {
    const v = video();
    const title = $("ytmusic-player-bar .title")?.textContent?.trim() ?? "";
    const byline = $("ytmusic-player-bar .byline")?.textContent ?? "";
    const cover = $("ytmusic-player-bar img.image")?.src ?? "";
    return {
      title,
      artist: byline.split("•")[0].trim(),
      cover: cover.replace(/=w\d+-h\d+.*$/, "=w544-h544"),
      playing: !!v && !v.paused && !v.ended,
      position: v?.currentTime ?? 0,
      duration: Number.isFinite(v?.duration) ? v.duration : 0,
      muted: !!v?.muted,
      shuffle: !!queueState().shuffleEnabled,
      repeat: queueState().repeatMode ?? "NONE",
      likeStatus:
        $("ytmusic-player-bar ytmusic-like-button-renderer")?.getAttribute("like-status") ??
        "INDIFFERENT",
    };
  };

  let last = "";
  setInterval(() => {
    const invoke = window.__TAURI_INTERNALS__?.invoke;
    if (!invoke) return;
    const state = snapshot();
    const json = JSON.stringify(state);
    if (json === last) return;
    last = json;
    invoke("report_state", { state }).catch(() => {});
  }, 500);
})();
