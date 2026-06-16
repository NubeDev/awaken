/**
 * Default shape for a freshly-added register row (DOMAIN-MODEL §register). Sane
 * Modbus defaults (a holding-register float32, big-endian) so a new row is valid
 * the moment it appears; the admin then edits it.
 */
import type { RegisterDef } from '@/api/records'

export function blankRegister(index: number): RegisterDef {
  return {
    key: `register_${index + 1}`,
    name: `Register ${index + 1}`,
    address: 0,
    fn_code: 'read_holding',
    datatype: 'float32',
    word_count: 2,
    byte_order: 'big',
    scale: 1,
    offset: 0,
    signed: false,
    unit: '',
    quantity: '',
    history: true,
    chart_type: 'line',
    chart_group: '',
    precision: 2,
  }
}
