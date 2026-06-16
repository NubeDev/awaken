/**
 * Ambient types for the WS-03 Node-side tag source of truth
 * (`nhp/seed/tags.mjs`), so the parity test (`tags.unit.test.ts`) can import it
 * without a `@ts-ignore`. Declares only the tag functions the UI mirror
 * (`tags.ts`) is asserted against. Vite resolves the `.mjs` for the test run.
 */
declare module '*/seed/tags.mjs' {
  export const tenantTag: (key: string) => string
  export const siteTag: (key: string) => string
  export const gatewayTag: (key: string) => string
  export const networkTag: (key: string) => string
  export const meterTag: (key: string) => string
  export const groupTag: (group: string) => string
  export const quantityTag: (quantity: string) => string
  export const meterTypeTag: (key: string) => string
  export const siteTags: (ctx: { tenant: string }) => string[]
  export const gatewayTags: (ctx: { tenant: string; site: string }) => string[]
  export const networkTags: (ctx: {
    tenant: string
    site: string
    gateway: string
  }) => string[]
  export const meterTags: (ctx: {
    tenant: string
    site: string
    gateway: string
    network: string
    meterType: string
  }) => string[]
  export const registerTags: (
    ctx: {
      tenant: string
      site: string
      gateway: string
      network: string
      meter: string
    },
    register: { chart_group?: string; quantity?: string }
  ) => string[]
}
