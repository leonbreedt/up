# TODO

- add integration tests for APIs

- add JWT issuing to server via simple login endpoint
  - use CA_CERTIFICATE, CERTIFICATE

- make alert enqueuing re-enqueue if the previously delivered alert was delivered longer than ping_period + grace_period ago
