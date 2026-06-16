/**
 * Ambient types for the WS-02 Node-side enum source of truth
 * (`nhp/collections/enums.mjs`), so the drift test can import it without a
 * `@ts-ignore`. Declares only the named exports the UI mirror compares against.
 */
declare module '*/collections/enums.mjs' {
  export const NET_TYPE: string[]
  export const PROTOCOL: string[]
  export const FN_CODE: string[]
  export const DATATYPE: string[]
  export const BYTE_ORDER: string[]
  export const CHART_TYPE: string[]
  export const STATUS: string[]
  export const ENUMS: Record<string, string[]>
}
