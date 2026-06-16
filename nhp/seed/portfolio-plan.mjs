// The shape of the mock portfolio the seed builds (SEED.md, WS-03 task 2): 2
// tenants × 2 sites; per site 1–2 gateways, each with a mix of 485 + ethernet
// networks (with max_devices caps) and several meters stamped from the two
// meter-types. This is data, not logic — the builder (portfolio.mjs) walks it.
//
// One register on a 485 bus is unit/slave-addressed; an ethernet (modbus-TCP)
// network addresses by ip:port and a unit id. `params` is free-form JSON on the
// network record (WS-02: rubix has no json FieldType; it passes through).
//
// Meters reference a meter-type by key (`mt`); the builder resolves it to the
// type's record id + version at stamp time (DOMAIN-MODEL §versioning:
// stamp-on-create). `addr` is the Modbus unit/slave id on the bus.

// A 485 (Modbus-RTU) network: serial line params, modest device cap.
const net485 = (key, name, maxDevices, meters) => ({
  key,
  name,
  net_type: '485',
  protocol: 'modbus',
  max_devices: maxDevices,
  params: { baud: 9600, parity: 'none', stop_bits: 1, data_bits: 8 },
  meters,
});

// An ethernet (Modbus-TCP) network: ip/port params, larger device cap.
const netEth = (key, name, maxDevices, ip, meters) => ({
  key,
  name,
  net_type: 'ethernet',
  protocol: 'modbus',
  max_devices: maxDevices,
  params: { ip, port: 502 },
  meters,
});

const meter = (key, name, mt, addr) => ({ key, name, mt, addr });

// Two tenants, each two sites; sites carry 1–2 gateways; gateways mix 485 +
// ethernet networks; networks carry several meters off both types.
export const PORTFOLIO = [
  {
    key: 'acme',
    name: 'Acme Industries',
    namespace: 'acme',
    sites: [
      {
        key: 'acme-hq',
        name: 'Acme HQ',
        address: '1 Market St, Springfield',
        timezone: 'America/New_York',
        geo: '39.7817,-89.6501',
        gateways: [
          {
            key: 'acme-hq-gw1',
            name: 'HQ Gateway 1',
            model: 'NHP-GW-200',
            host: '10.0.1.10',
            networks: [
              net485('acme-hq-gw1-485a', 'HQ RS485 A', 32, [
                meter('acme-hq-m1', 'Main Incomer', 'acme-pm5560', 1),
                meter('acme-hq-m2', 'Floor 1 DB', 'acme-pm5560', 2),
                meter('acme-hq-m3', 'Floor 2 DB', 'acme-em24', 3),
              ]),
              netEth('acme-hq-gw1-eth1', 'HQ Ethernet 1', 64, '10.0.2.0', [
                meter('acme-hq-m4', 'HVAC Plant', 'acme-pm5560', 10),
                meter('acme-hq-m5', 'Lighting', 'acme-em24', 11),
              ]),
            ],
          },
        ],
      },
      {
        key: 'acme-plant',
        name: 'Acme Plant',
        address: '500 Industrial Rd, Springfield',
        timezone: 'America/Chicago',
        geo: '39.7990,-89.6440',
        gateways: [
          {
            key: 'acme-plant-gw1',
            name: 'Plant Gateway 1',
            model: 'NHP-GW-200',
            host: '10.1.1.10',
            networks: [
              net485('acme-plant-gw1-485a', 'Plant RS485 A', 16, [
                meter('acme-plant-m1', 'Line A Main', 'acme-pm5560', 1),
                meter('acme-plant-m2', 'Line B Main', 'acme-pm5560', 2),
              ]),
            ],
          },
          {
            key: 'acme-plant-gw2',
            name: 'Plant Gateway 2',
            model: 'NHP-GW-100',
            host: '10.1.1.11',
            networks: [
              netEth('acme-plant-gw2-eth1', 'Plant Ethernet 1', 64, '10.1.2.0', [
                meter('acme-plant-m3', 'Compressor', 'acme-em24', 5),
                meter('acme-plant-m4', 'Welding Bay', 'acme-pm5560', 6),
              ]),
            ],
          },
        ],
      },
    ],
  },
  {
    key: 'globex',
    name: 'Globex Corporation',
    namespace: 'globex',
    sites: [
      {
        key: 'globex-tower',
        name: 'Globex Tower',
        address: '742 Evergreen Tce, Capital City',
        timezone: 'America/Los_Angeles',
        geo: '34.0522,-118.2437',
        gateways: [
          {
            key: 'globex-tower-gw1',
            name: 'Tower Gateway 1',
            model: 'NHP-GW-200',
            host: '172.16.1.10',
            networks: [
              net485('globex-tower-gw1-485a', 'Tower RS485 A', 32, [
                meter('globex-tower-m1', 'Tower Main', 'acme-pm5560', 1),
                meter('globex-tower-m2', 'Datacentre', 'acme-pm5560', 2),
              ]),
              netEth('globex-tower-gw1-eth1', 'Tower Ethernet 1', 48, '172.16.2.0', [
                meter('globex-tower-m3', 'Chillers', 'acme-em24', 7),
              ]),
            ],
          },
        ],
      },
      {
        key: 'globex-depot',
        name: 'Globex Depot',
        address: '88 Logistics Way, Capital City',
        timezone: 'America/Denver',
        geo: '34.0699,-118.2540',
        gateways: [
          {
            key: 'globex-depot-gw1',
            name: 'Depot Gateway 1',
            model: 'NHP-GW-100',
            host: '172.16.3.10',
            networks: [
              net485('globex-depot-gw1-485a', 'Depot RS485 A', 16, [
                meter('globex-depot-m1', 'Warehouse Main', 'acme-pm5560', 1),
                meter('globex-depot-m2', 'Cold Store', 'acme-em24', 2),
              ]),
            ],
          },
        ],
      },
    ],
  },
];
