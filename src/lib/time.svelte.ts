// A clock ticking every minute, so the ages shown stay current.
export const clock = $state({ now: Date.now() });
setInterval(() => (clock.now = Date.now()), 60_000);

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** `now`, `5m`, `3h`, `2d`, `3w`, `4mo`, `1y`. */
export function age(ms: number, now: number): string {
  const d = Math.max(0, now - ms);
  if (d < MINUTE) return "now";
  if (d < HOUR) return `${Math.floor(d / MINUTE)}m`;
  if (d < DAY) return `${Math.floor(d / HOUR)}h`;
  if (d < 14 * DAY) return `${Math.floor(d / DAY)}d`;
  if (d < 60 * DAY) return `${Math.floor(d / (7 * DAY))}w`;
  if (d < 365 * DAY) return `${Math.floor(d / (30 * DAY))}mo`;
  return `${Math.floor(d / (365 * DAY))}y`;
}

const dateFormat = new Intl.DateTimeFormat(undefined, {
  weekday: "short",
  day: "numeric",
  month: "short",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});

export const longDate = (ms: number) => dateFormat.format(ms);
