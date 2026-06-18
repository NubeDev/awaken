/**
 * Drift guard: the UI enum mirror (`options.ts`) MUST equal the WS-02 Node source
 * of truth (`nhp/collections/enums.mjs`). The `.mjs` lives outside `src` so it
 * can't enter the tsc build, but Vite resolves it for the test — so this keeps the
 * two in sync without duplicating the lists into the build. See options.ts header.
 */
import { describe, expect, it } from 'vitest'
import * as node from '../../../collections/enums.mjs'
import * as ui from './options'

const pairs: [readonly string[], string[]][] = [
  [ui.NET_TYPE, node.NET_TYPE],
  [ui.PROTOCOL, node.PROTOCOL],
  [ui.FN_CODE, node.FN_CODE],
  [ui.DATATYPE, node.DATATYPE],
  [ui.BYTE_ORDER, node.BYTE_ORDER],
  [ui.CHART_TYPE, node.CHART_TYPE],
  [ui.STATUS, node.STATUS],
  [ui.QUANTITY, node.QUANTITY],
]

describe('NHP enum options mirror nhp/collections/enums.mjs', () => {
  it.each(pairs)('set %# matches', (uiSet, nodeSet) => {
    expect([...uiSet]).toEqual(nodeSet)
  })
})
