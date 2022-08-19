# TODO

- scope all queries to account
- add JWT issuing and verification to server
  - use CA_CERTIFICATE, CERTIFICATE and JWKS env vars
- simple login endpoint that issues JWT
- add auth middleware that verifies JWT
- make alert enqueuing re-enqueue if the previously delivered alert was delivered longer than ping_period + grace_period ago
