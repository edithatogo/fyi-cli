# Alaveteli requester and authority workflow map

Reference implementation: upstream Alaveteli commit
[`3966415`](https://github.com/mysociety/alaveteli/commit/3966415f0b2a9f445fa3bb25c06eaeb75ded6609).
The [coverage inventory](../../../docs/api-contract-inventory.md) records the
current code and synthetic evidence. A FAQ describes a site's user experience;
it does not establish the version or enabled features of another deployment.

| Actor and intent | Source-backed path | Contract and verification |
| --- | --- | --- |
| Anyone reads and searches | JSON representations and Atom feeds | Public read coverage must qualify the selected source's pages, feeds, pagination and attachments. [Alaveteli API](https://alaveteli.org/docs/developers/api/) |
| Requester registers and confirms | `POST /profile/sign_up`, then the emailed `/c/:email_token` confirmation | Use fictional mail interception and assert token expiry, replay and account state. [Pinned routes](https://github.com/mysociety/alaveteli/blob/3966415f0b2a9f445fa3bb25c06eaeb75ded6609/config/routes.rb); [WhatDoTheyKnow officer FAQ](https://www.whatdotheyknow.com/help/officers) |
| Requester signs in and out | `GET/POST /profile/sign_in`, `GET /profile/sign_out` in the pinned routes | Carry session and CSRF state; verify invalid/expired sessions and logout. Optional 2FA must be user-mediated. [Account FAQ](https://www.whatdotheyknow.com/help/account_management) |
| Requester creates a request | `/new/:url_name` GET/POST, with any required confirmation | A prefilled URL only prepares a form; prove one intercepted outbound message and the resulting request/event. [Alaveteli request management](https://alaveteli.org/docs/running/requests/) |
| Authority replies to an ordinary request | Reply to the request email, or a site's response form; Alaveteli associates incoming mail with the request | In the isolated environment inject fictional mail and attachment bytes and assert association. [Officer FAQ](https://www.whatdotheyknow.com/help/officers); [Alaveteli overview](https://alaveteli.org/docs/developers/overview/) |
| Requester follows up or classifies | Follow-up form/preview/create routes and request classification | Verify authenticated ownership, review/send separation, exactly one intercepted message, and state/readback. [Pinned routes](https://github.com/mysociety/alaveteli/blob/3966415f0b2a9f445fa3bb25c06eaeb75ded6609/config/routes.rb); [responses FAQ](https://www.whatdotheyknow.com/help/responses) |
| Authority imports an external request | Experimental authority API: `POST /api/v2/request.json`, `k`, `request_json` | This records an external request and canonical external URL; it is not the ordinary requester's new-request submission. [Pinned controller](https://github.com/mysociety/alaveteli/blob/3966415f0b2a9f445fa3bb25c06eaeb75ded6609/app/controllers/api_controller.rb); [API documentation](https://alaveteli.org/docs/developers/api/) |
| Authority adds imported correspondence or state | `POST /api/v2/request/:id.json` with `correspondence_json`, optional separate `state` and response-only `attachments[]`; `POST /api/v2/request/:id/update.json` for state | Require the authority key, ownership and external-request checks, preserve the returned URL and validate failure handling. Pinned controller and API prose differ on parameter names; qualify each deployment. |

The public [FYI introduction](https://fyi.org.nz/help/about) describes choosing
an authority, writing a request and publishing the reply. It is product
context, not authorization to create an account or send a live request.
