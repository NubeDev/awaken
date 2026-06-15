// Deterministic site card gradients, ported from the demo's per-site `grad`.
// Picked by index so a site always gets the same look across reloads.

const GRADS = [
  '258 84% 64%,174 70% 50%',
  '200 86% 58%,258 84% 68%',
  '357 84% 60%,32 92% 56%',
  '32 92% 56%,150 64% 48%',
  '150 64% 48%,200 86% 58%',
  '174 70% 50%,258 84% 66%',
]

export const siteGradient = (i: number): string => `linear-gradient(135deg,hsl(${GRADS[i % GRADS.length]}))`
