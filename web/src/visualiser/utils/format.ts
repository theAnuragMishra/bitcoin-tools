export function sats(n: number): string {
  if (n >= 1e8) return `${(n / 1e8).toFixed(8)} BTC`;
  return `${n.toLocaleString()} sat`;
}

export function shortHex(h: string, chars = 8): string {
  if (!h || h.length <= chars * 2) return h;
  return `${h.slice(0, chars)}…${h.slice(-chars)}`;
}
