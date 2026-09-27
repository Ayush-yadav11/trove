// Picks the part of a matched chunk worth showing and splits it into
// highlighted/plain segments. Rendered with {#each}, never as HTML, so file
// contents can't inject markup.

export interface Segment {
  text: string;
  hit: boolean;
}

const WINDOW = 240;
const LEAD = 70;

// Same list the keyword search ignores (quillrag src/search.rs), so the
// highlights match what actually drove the result.
const STOPWORDS = new Set(
  ("a about an and are as at be by can do does for from how i in into is it its me my of on or " +
    "our that the their this to was we what when where which who why will with you your").split(" "),
);

function terms(query: string): string[] {
  const words = (query.toLowerCase().match(/[\p{L}\p{N}]{2,}/gu) ?? []).filter(
    (w) => !STOPWORDS.has(w),
  );
  // Longest first so "invoice" wins over "in" at the same position.
  return [...new Set(words)].sort((a, b) => b.length - a.length);
}

function escapeRe(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function snippet(text: string, query: string): Segment[] {
  const flat = text.replace(/\s+/g, " ").trim();
  const ts = terms(query);
  // Match at word starts only: "loan" highlights "loans" but "can" never
  // lights up inside "scan".
  const re = ts.length
    ? new RegExp(`(?<![\\p{L}\\p{N}])(?:${ts.map(escapeRe).join("|")})`, "giu")
    : null;

  // Center the window on the first keyword hit; semantic-only matches (no
  // shared words) just show the start of the chunk.
  let start = 0;
  const first = re ? flat.search(re) : -1;
  if (first > LEAD) {
    start = flat.lastIndexOf(" ", first - LEAD) + 1;
  }
  let end = Math.min(flat.length, start + WINDOW);
  if (end < flat.length) {
    const space = flat.lastIndexOf(" ", end);
    if (space > start) end = space;
  }
  const body = flat.slice(start, end);
  const prefix = start > 0 ? "…" : "";
  const suffix = end < flat.length ? "…" : "";

  if (!re) return [{ text: prefix + body + suffix, hit: false }];

  const out: Segment[] = [];
  let last = 0;
  re.lastIndex = 0;
  for (const m of body.matchAll(re)) {
    const i = m.index ?? 0;
    if (i > last) out.push({ text: body.slice(last, i), hit: false });
    out.push({ text: m[0], hit: true });
    last = i + m[0].length;
  }
  if (last < body.length) out.push({ text: body.slice(last), hit: false });
  if (prefix) out.unshift({ text: prefix, hit: false });
  if (suffix) out.push({ text: suffix, hit: false });
  return out;
}

const KIND: Record<string, string> = {
  pdf: "PDF",
  md: "MD",
  markdown: "MD",
  txt: "TXT",
  png: "IMG",
  jpg: "IMG",
  jpeg: "IMG",
  webp: "IMG",
  bmp: "IMG",
  gif: "IMG",
  html: "HTML",
  htm: "HTML",
  json: "JSON",
  csv: "CSV",
  tsv: "CSV",
};

/** Short badge label for a file extension. */
export function kind(ext: string): string {
  return KIND[ext] ?? (ext ? ext.slice(0, 4).toUpperCase() : "FILE");
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = n / 1024;
  let u = 0;
  while (v >= 1024 && u < units.length - 1) {
    v /= 1024;
    u++;
  }
  return `${v < 10 ? v.toFixed(1) : Math.round(v)} ${units[u]}`;
}
