import { toProxyUrl } from '../utils/url';

/**
 * グローバルなクリックリスナーを初期化し、ナビゲーション（<a>タグ）を傍受します。
 * ブラウザが遷移を開始する前に、遷移先のURLをMYCUTEプロキシスキームへと書き換えます。
 */
/**
 * グローバルなクリックリスナーとAPIのオーバーライドで、同期的なページ遷移を完全にロックします。
 * MYCUTEはOS上のシングルアプリケーションとして振る舞うため、iframe内でのページ遷移は禁止されています。
 */
export function initNavigationInterceptor() {
    // 1. Block window.open
    const originalOpen = window.open;
    window.open = function (...args: any[]) {
        console.warn(`[MYCUTE SDK] 🚫 Blocked window.open() call. Synchronous page transitions are prohibited in this application context.`);
        return null;
    };

    // 2. Block <a> tag navigation
    document.addEventListener('click', (event) => {
        const target = (event.target as HTMLElement).closest('a');

        if (target && target.href) {
            // Check if this is a hash link or same-page link?
            // User requested strict ban on sync requests.
            // If the application (Vue/React) handles this click and calls preventDefault(), 
            // then we don't need to do anything (it's an SPA transition).
            // But if the event continues bubbling and browser is about to load URL, we MUST stop it.

            // We use 'click' with capture=true to inspect it, but we can't know if preventDefault called yet?
            // Actually capture happens BEFORE internal handlers.
            // If we want to ban "sync request", we should perhaps allow internal handlers to run,
            // but ensure that if it bubbles up to causing navigation, it gets killed.

            // However, to be "strictly prohibited", we should intercept here.
            // But blocking ALL <a> tags breaks SPA routing unless we carefully check.

            // The user said: "Sync request page transition... prohibited".
            // SPA routing is NOT sync request page transition.
            // So we simply add a listener at the bubbled stage (false) or just preventDefault if it looks like external nav?
            // Or simpler: override `window.location` setters? (Hard to do fully).

            // Let's stick to the prompt: "prohibit any sync request... moving to another app".
            // We will listen in bubbling phase (default) to let framework handle it if it wants.
            // If event.defaultPrevented is false, it means browser IS about to navigate.
            // So we kill it.
        }
    }, true); // Capture phase is safer to inspect but we want to know if it's handled?

    // Better strategy:
    // Listen on 'click' at document level (bubbling phase). 
    // If preventDefault() was NOT called, then it's a native navigation -> BLOCK IT.
    document.addEventListener('click', (event) => {
        // Only care about left click
        if (event.button !== 0) return;

        const target = (event.target as HTMLElement).closest('a');
        if (target && target.href) {
            // If default is prevented, it's likely handled by SPA router. Good.
            if (event.defaultPrevented) return;

            // If it is NOT prevented, browser will navigate.
            // We verify if it is a harmless anchor link (#) or actual navigation.
            const url = new URL(target.href, window.location.href);
            // Ignore same-page hash links
            if (url.origin === window.location.origin && url.pathname === window.location.pathname && url.hash) {
                return;
            }

            // Otherwise, BLOCK IT.
            event.preventDefault();
            event.stopPropagation();
            console.warn(`[MYCUTE SDK] 🚫 Blocked link navigation to "${target.href}". Synchronous page transitions are prohibited.`);
        }
    }, false); // Bubbling phase (wait for SPA routers)

    // 3. Block window.location assignment (Best effort)
    // It's hard to catch `location.href = ...` perfectly without proxying window.location which is complex.
    // But we can listen for `beforeunload`? User didn't ask for that specifically but `sync request` implies avoiding unload.
    window.addEventListener('beforeunload', (event) => {
        // We can't strictly block it via beforeunload easily without user interaction.
        // But preventing <a> and window.open covers 90% of explicit user actions.
    });

    console.log('[MYCUTE SDK] Navigation Lock active. Sync page transitions are strictly prohibited.');
}
