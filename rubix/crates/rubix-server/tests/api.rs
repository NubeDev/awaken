//! API integration tests, one module per resource.

mod api_tests {
    mod harness;

    mod agent;
    mod agent_call;
    mod auth;
    mod boards;
    mod bus;
    mod command;
    mod dashboards;
    mod dispatch;
    mod flow;
    mod his;
    mod mcp;
    mod orgs;
    mod points;
    mod query;
    mod rollup;
    mod rules;
    mod runs;
    mod seed;
    mod sites;
    mod sparks;
    mod tenancy;
    mod tools;
    mod widgets;
}
