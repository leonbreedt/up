# TODO

- tighten up security:
  - ensure all queries filter by account
  - projects are bound to an account
  - checks are bound to a project
  - current user can only see projects for accounts they exist in **and** have been added to.
  - must specify project ID when working with checks and notifications
  - only an admin can create/update projects or assign users to them
  - only an editor can create/update/delete checks in projects
  - a viewer cannot change any data

- add JWT issuing to server via simple login endpoint
  - use CA_CERTIFICATE, CERTIFICATE
  - 
- make alert enqueuing re-enqueue if the previously delivered alert was delivered longer than ping_period + grace_period ago
