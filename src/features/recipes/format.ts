/** "1 day ago", "3 days ago", "2 weeks ago". */
export function formatDaysAgo(days: number) {
	if (days === 1) return "1 day ago";
	if (days < 7) return `${days} days ago`;

	const weeks = Math.floor(days / 7);
	return weeks === 1 ? "1 week ago" : `${weeks} weeks ago`;
}

/** Host name of a link without `www.`, or an empty string when the link is not a valid URL. */
export function domainOf(url: string) {
	try {
		return new URL(url).hostname.replace(/^www\./, "");
	} catch {
		return "";
	}
}

/**
 * Turns a pasted link into a full web address. Adds `https://` when the scheme is missing.
 * Returns null when the input is not a web address with a domain.
 */
export function normalizeLink(input: string) {
	const value = input.trim();
	if (!value) return null;

	const withScheme = /^https?:\/\//i.test(value) ? value : `https://${value}`;
	try {
		const url = new URL(withScheme);
		return url.hostname.includes(".") ? url.toString() : null;
	} catch {
		return null;
	}
}

/**
 * A starting title from the last part of a link, e.g. `.../chicken-tikka-traybake` becomes
 * "Chicken tikka traybake". Falls back to the domain. The user edits it afterwards.
 */
export function titleFromLink(link: string) {
	const { hostname, pathname } = new URL(link);
	const slug = pathname.split("/").filter(Boolean).pop() ?? "";
	const words = slug.replace(/\.(html?|php|aspx?)$/i, "").replace(/[-_]+/g, " ").trim();
	const title = words || hostname.replace(/^www\./, "");

	return title.charAt(0).toUpperCase() + title.slice(1);
}
