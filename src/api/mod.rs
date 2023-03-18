//! HTTP API for updating dynamic TXT record responses.
//!
//! # API Endpoints
//!
//! ## `/healthcheck` (GET)
//!   
//!   Returns HTTP 200 (OK) and the JSON body `{"ok":"healthy"}` when the service is operational.
//!
//! ## `/register` (POST)
//!
//!   Returns HTTP 501 (Not Implemented).
//!
//!   This endpoint is not provided by ACME Crab. It is
//!   expected that users configure their ACME client to use the ACME DNS API provided by
//!   ACME Crab as if a user account had already been registered. No username/password
//!   header authentication is performed by ACME Crab's update endpoint. ACLs are handled
//!   with [cryptokey routing].
//!
//!   [cryptokey routing]: https://www.wireguard.com/#cryptokey-routing
//!
//! ## `/update` (POST)
//!
//!   Expects a JSON request body of the form:
//!
//!   ```json
//!   { "subdomain": "test", "txt": "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX" }
//!   ```
//!  
//!  Where `subdomain` is a subdomain of the ACME Crab domain, registered in the configuration
//!  ACL. The client `POST`ing the update must have a source IP address within a network specified
//!  in the ACL entry for the `subdomain`.
//!
//!  The `txt` value must be a valid [RFC-8555][RFC-8555] [DNS-01] challenge response.
//!  
//!  For successful updates, returns HTTP 200 (OK) and a JSON response body of the form:
//!
//!  ```json
//!  { "txt": "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX" }
//!  ```
//!  In the response, `txt` contains the echoed `txt` value from the client request.
//!
//! [RFC-8555]: https://www.rfc-editor.org/rfc/rfc8555
//! [DNS-01]: https://www.rfc-editor.org/rfc/rfc8555#section-8.4

mod api_error;
mod model;
mod routes;
pub mod server;

pub use server::new;
