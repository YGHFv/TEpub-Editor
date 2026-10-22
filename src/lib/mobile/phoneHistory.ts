export type PhoneNavigationType =
    | "enter"
    | "form"
    | "leave"
    | "link"
    | "goto"
    | "popstate";

export const PHONE_HISTORY_LIMIT = 24;

function trimHistory(history: string[]): string[] {
    if (history.length <= PHONE_HISTORY_LIMIT) return history;
    return history.slice(history.length - PHONE_HISTORY_LIMIT);
}

export function routeHref(url: {
    pathname: string;
    search: string;
    hash: string;
}): string {
    return `${url.pathname}${url.search}${url.hash}`;
}

/**
 * Track app-internal navigation instead of trusting window.history.length.
 * The WebView can already have external or browser-generated history entries
 * on first launch, so history.back() is not guaranteed to stay inside the app.
 */
export function recordPhoneNavigation(
    history: string[],
    href: string,
    type: PhoneNavigationType,
    primaryRoute = false,
): string[] {
    if (!href || type === "leave") return history;

    const current = trimHistory(history);
    if (type === "enter") return [href];
    // Primary tabs are the start of a fresh mobile stack.
    if (primaryRoute) return [href];
    if (current.at(-1) === href) return current;

    if (type === "popstate") {
        const previous = current.lastIndexOf(href, current.length - 2);
        if (previous >= 0) return current.slice(0, previous + 1);
    }

    return trimHistory([...current, href]);
}

export function phoneBackTarget(
    history: string[],
    currentHref: string,
): string | null {
    const current = history.lastIndexOf(currentHref);
    const index = current >= 0 ? current : history.length - 1;
    return index > 0 ? history[index - 1] : null;
}
