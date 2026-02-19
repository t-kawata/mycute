/**
 * Registers the MYCUTE proxy Service Worker.
 * 
 * @param swPath The path to the compiled Service Worker file (default: '/sw.js').
 *               Ensure that the SW file is served from the root scope or appropriate scope.
 * @returns A promise that resolves to the ServiceWorkerRegistration.
 */
export async function registerServiceWorker(swPath: string = '/sw.js'): Promise<ServiceWorkerRegistration | undefined> {
    if ('serviceWorker' in navigator) {
        try {
            const registration = await navigator.serviceWorker.register(swPath, {
                scope: '/'
            });
            console.log('[MYCUTE SDK] Service Worker registered with scope:', registration.scope);
            return registration;
        } catch (error) {
            console.error('[MYCUTE SDK] Service Worker registration failed:', error);
            throw error;
        }
    } else {
        console.warn('[MYCUTE SDK] Service Worker is not supported in this browser.');
        return undefined;
    }
}
