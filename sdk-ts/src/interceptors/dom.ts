import { toProxyUrl, isProxyUrl } from '../utils/url';
import { MYCUTE_ORIGIN } from '../generated_constants';

// Helper to check if string looks like an external URL
function isExternalUrl(value: string): boolean {
    if (!value || typeof value !== 'string') return false;
    const v = value.trim();
    return (v.startsWith('http://') || v.startsWith('https://')) && !isProxyUrl(v);
}

// Helper to resolve and proxy a URL
function resolveAndProxy(value: string): string {
    try {
        const resolved = new URL(value, window.location.href).href;
        return toProxyUrl(resolved);
    } catch (e) {
        return value;
    }
}

export function initDomInterceptors() {
    if ((window as any).__MYCUTE_DOM_INTERCEPTORS_ACTIVE__) return;
    (window as any).__MYCUTE_DOM_INTERCEPTORS_ACTIVE__ = true;

    console.log('[MYCUTE SDK] Activating Advanced DOM Interceptors (MutationObserver + setAttribute)');

    // 1. Hook Element.prototype.setAttribute
    // This catches *any* attribute assignment via JS that looks like a URL
    const originalSetAttribute = Element.prototype.setAttribute;
    Element.prototype.setAttribute = function (name: string, value: string) {
        // We don't check attribute name (src/href), we check the VALUE.
        // If it looks like an absolute URL (http/s), we proxy it.
        if (isExternalUrl(value)) {
            // console.debug(`[MYCUTE SDK] Intercepted setAttribute(${name}):`, value);
            value = resolveAndProxy(value);
        }
        return originalSetAttribute.call(this, name, value);
    };

    // 2. Hook Image Accessor (new Image().src = ...)
    // Still useful for direct property access which bypasses setAttribute
    const imageProto = HTMLImageElement.prototype;
    const originalImageSrcDescriptor = Object.getOwnPropertyDescriptor(imageProto, 'src');

    if (originalImageSrcDescriptor) {
        Object.defineProperty(imageProto, 'src', {
            get() {
                return originalImageSrcDescriptor.get?.call(this);
            },
            set(value: string) {
                if (isExternalUrl(value)) {
                    // console.debug('[MYCUTE SDK] Intercepted Image.src:', value);
                    value = resolveAndProxy(value);
                }
                originalImageSrcDescriptor.set?.call(this, value);
            },
            configurable: true,
            enumerable: true
        });
    }

    // 3. Hook InnerHTML (Naive approach)
    // We can't easily parse partial HTML strings efficiently, but MutationObserver covers the result.
    // So we rely on MutationObserver for innerHTML injections.

    // 4. MutationObserver for DOM additions
    // This catches <img src="..."> inserted via innerHTML or appendChild
    const observer = new MutationObserver((mutations) => {
        mutations.forEach((mutation) => {
            // Check added nodes
            mutation.addedNodes.forEach((node) => {
                if (node.nodeType !== Node.ELEMENT_NODE) return;
                const el = node as Element;

                // Process the element itself
                processElement(el);

                // Process all descendants
                const descendants = el.querySelectorAll('*');
                descendants.forEach(processElement);
            });

            // Check attribute changes (if missed by setAttribute hook, e.g. via parser)
            if (mutation.type === 'attributes' && mutation.attributeName) {
                const el = mutation.target as Element;
                const attrName = mutation.attributeName;
                const val = el.getAttribute(attrName);
                if (val && isExternalUrl(val)) {
                    // console.debug(`[MYCUTE SDK] MutationObserver caught attribute ${attrName}:`, val);
                    const newVal = resolveAndProxy(val);
                    // Avoid infinite loop by checking if value actually changed
                    if (newVal !== val) {
                        // We must use the original setAttribute to avoid recursive hook logging? 
                        // Actually our hook is safe because resolveAndProxy returns verified proxied URL,
                        // so isExternalUrl(newVal) will be false (if implemented correctly with isProxyUrl check).
                        el.setAttribute(attrName, newVal);
                    }
                }
            }
        });
    });

    observer.observe(document.documentElement, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: ['src', 'href', 'srcset', 'data-src', 'poster', 'action'] // Common URL attributes for perm
    });
    // 5. Hook attachShadow to monitor Shadow DOM
    const originalAttachShadow = Element.prototype.attachShadow;
    Element.prototype.attachShadow = function (init: ShadowRootInit): ShadowRoot {
        const shadowRoot = originalAttachShadow.call(this, init);

        // Observe the new Shadow Root
        observer.observe(shadowRoot, {
            childList: true,
            subtree: true,
            attributes: true,
            attributeFilter: ['src', 'href', 'srcset', 'data-src', 'poster', 'action']
        });

        return shadowRoot;
    };

    console.log('[MYCUTE SDK] DOM interceptors active (including Shadow DOM).');
}

function processElement(el: Element) {
    // Check common attributes
    // We iterate specific attributes to avoid checking every single attribute on every element
    const urlAttributes = ['src', 'href', 'srcset', 'poster', 'action', 'data-src'];

    urlAttributes.forEach(attr => {
        if (el.hasAttribute(attr)) {
            const val = el.getAttribute(attr);
            if (val && isExternalUrl(val)) {
                // console.debug(`[MYCUTE SDK] MutationObserver caught new element ${el.tagName} with ${attr}:`, val);
                el.setAttribute(attr, resolveAndProxy(val));
            }
        }
    });

    // Case for style attribute (background-image) is hard to parse reliably in JS without huge overhead.
    // We leave CSS handling to the server-side rewriter.
}
