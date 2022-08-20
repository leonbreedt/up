# TODO

- tighten up security:
  - add updated_by and deleted_by columns
  - only an admin can create/update projects or assign users to them
  - only an editor can create/update/delete checks in projects
  - a viewer cannot change any data

- add unit tests

- add JWT issuing to server via simple login endpoint
  - use CA_CERTIFICATE, CERTIFICATE

- make alert enqueuing re-enqueue if the previously delivered alert was delivered longer than ping_period + grace_period ago
