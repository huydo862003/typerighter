---
_label: "JSON-RPC 2.0"
_icon: icon.document
---

References: [www.jsonrpc.org](https://www.jsonrpc.org/).

## Background

This section walkthrough the background knowledge required to understand JSON-RPC.

### JSON - Serialization Format

JSON (Javascript Object Notation) is simply a serialization format that is:

- Portable.
- Readable.
- Capable of representing most plain values.

3 basic components that universally present in almost any programming languages:

- Primitives: true, false, null, string, number.
- Objects: collections of name/value pairs.
- Arrays: ordered lists of values.

With these properties, JSON is usually used as a data-interchange format, typically seen in web APIs.

### RPC - Just Some Wrapper Functions

RPC (remote procedure call) is a type of client-server API, in which a process triggers the execution of a procedure in the context of a different address space.

As far as I know, RPC consists of 3 parts:

- The server defines and publishes some well-known methods.
- The client which will want to trigger these methods.
- Some stubs that represent the remote methods, and the client will call these stubs like normal functions.

Essentially, from the point of view of the client, RPC is a just a normal function, which wraps around some logic to notify the remote processes.

## Forewords

At first glance, JSON-RPC seems like "just use JSON for RPC calls". But that alone wouldn't warrant a spec. You could say the same about XML-RPC, or any other serialization format.

What makes JSON-RPC distinct is that it defines a formal protocol (currently at version 2.0) for how requests, responses, errors, notifications, and batches are structured. The JSON part is the encoding. The RPC part is the protocol.

To keep things clear:

- **RPC-via-JSON**: The generic idea of sending RPC calls encoded as JSON. No standard structure. Each implementation invents its own message format, error handling, and conventions.
- **JSON-RPC**: A specific protocol with defined rules:
  - Fixed message structure (`jsonrpc`, `id`, `method`, `params`, `result`, `error`).
  - Notifications (requests without `id` that expect no response).
  - Batch requests (array of messages processed together).
  - Standardized error codes (-32700 through -32603).
  - Transport-agnostic (works over HTTP, WebSocket, stdio, etc.).

JSON-RPC is one way to do RPC-via-JSON, but not all RPC-via-JSON is JSON-RPC.

## Overview

JSON-RPC is an RPC protocol that has some noticeable properties:

- **Stateless**: States do not linger across requests, each request is treated unagnostic of whether any specific requests come before or after it.
- **Light-weight**: This is a bit vague, but I think the spec is light-weight and not excessively verbose. Implementations can easily implement the spec.
- **Transport-agnostic**: The spec only defines data structures and rules for processing them. It doesn't care about the transport means: sockets, HTTP, stdio, WebSocket, in-process message passing, whatever.
- **Simple**: The spec itself says it's "designed to be simple!". It does seem simple given that it fits in a short HTML page.

## Conventions

Note that capitalization is DELIBERATE!

Some boring but worth-noting conventions from the spec:

- The spec uses RFC 2119 keywords (MUST, SHOULD, MAY, etc.).
- The type system is just JSON's:
  - 4 primitives (String, Number, Boolean, Null).
  - 2 structured types (Object, Array).
  - Always capitalized in the spec.
- All member names are **case-sensitive**.
- "function", "method", and "procedure" are used interchangeably.
- **Client**: Sends Request objects, handles Response objects.
  - Always capitalized.
- **Server**: Handles Request objects, sends Response objects.
  - Always capitalized.

One interesting note: A single program can act as both client and server at the same time. For example, program A serves requests from B, while also sending its own requests back to B. The spec acknowledges this is possible but says "this specification does not address that layer of complexity." In other words: If both sides send requests with the same `id`, or if you need to distinguish "my requests" from "their requests," that's your problem.

The LSP is a real-world example of this: `typedown-lsp` is a server (handles `textDocument/hover` from the editor), but also a client (sends `client/registerCapability` TO the editor). Both sides need to track which `id`s belong to which direction.

## Compatibility

JSON-RPC 2.0 is not backward-compatible with 1.0. But it's easy to tell them apart: 2.0 messages always have `"jsonrpc": "2.0"`, 1.0 does not. The spec recommends that 2.0 implementations try to handle 1.0 messages gracefully.

> Not relevant to us since we only target 2.0.

## Request Object

> Definition: An RPC call is equivalent to sending a **Request** object to a **Server**.

A Request is just a JSON Object with these members:

| Member    | Type                                                                                     | Required  | Description                    |
| --------- | ---------------------------------------------------------------------------------------- | --------- | ------------------------------ |
| `jsonrpc` | String                                                                                   | MUST      | Always `"2.0"`                 |
| `method`  | String                                                                                   | MUST      | Name of the method to invoke   |
| `params`  | Object or Array                                                                          | MAY       | Arguments to the method        |
| `id`      | String, Number (SHOULD not contain fractional parts), or Null (SHOULD avoid in Requests) | See below | Correlates request to response |

Some notes:

- Method names starting with `rpc.` are reserved by the spec.
- `params` can be omitted entirely if the method takes no arguments.
- If `id` is present, the server MUST reply with the same `id` in the Response. If `id` is absent, it's a Notification (covered later).
- `id` SHOULD NOT be Null in Requests (Null is used for responses with unknown id, and 1.0 used Null for notifications, so it's ambiguous).
- `id` SHOULD NOT be a fractional Number (many decimal fractions can't be represented exactly in binary).

Example:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "buildFile",
  "params": { "path": "content/alice.td" }
}
```

### Notification

A Notification is a Request **without** an `id` member (not `id: null`, literally absent). It signals that the client doesn't care about the response.

- The server MUST NOT reply to a Notification. Not even errors.
- Since there's no response, the client has no way to know if the call succeeded or failed.
- This applies even within batch requests.

```json
{
  "jsonrpc": "2.0",
  "method": "fileChanged",
  "params": { "path": "content/alice.td" }
}
```

> Note the key distinction: A Request with `"id": null` is NOT a Notification. A Notification has no `id` member at all.

### Parameter Structures

If `params` is present, it MUST be either:

- **By-position** (Array): Values in the order the server expects. `["content/alice.td", true]`
- **By-name** (Object): Keys matching the server's expected parameter names, case-sensitive. `{ "path": "content/alice.td", "force": true }`

Missing expected names MAY result in an error.

## Response Object

> Definition: The server MUST reply with a **Response** to every Request, except Notifications.

A Response is a JSON Object with these members:

| Member    | Type                    | Required        | Description                                                                                          |
| --------- | ----------------------- | --------------- | ---------------------------------------------------------------------------------------------------- |
| `jsonrpc` | String                  | MUST            | Always `"2.0"`                                                                                       |
| `result`  | any                     | MUST on success | The return value of the method. MUST NOT exist on error.                                             |
| `error`   | Object                  | MUST on error   | Error details (see Error Object below). MUST NOT exist on success.                                   |
| `id`      | String, Number, or Null | MUST            | Same as the `id` from the Request. Null if the request `id` couldn't be detected (e.g. parse error). |

The key constraint: Exactly one of `result` or `error` MUST be present. Never both, never neither.

Success example:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { "markdown": "..." }
}
```

Error example:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": { "code": -32601, "message": "Method not found" }
}
```

### Error Object

The `error` member in a Response is an Object with:

| Member    | Type             | Required | Description                            |
| --------- | ---------------- | -------- | -------------------------------------- |
| `code`    | Number (integer) | MUST     | Error type identifier                  |
| `message` | String           | MUST     | Short, single-sentence description     |
| `data`    | any              | MAY      | Additional info, defined by the server |

### Reserved Error Codes

The range -32768 to -32000 is reserved. Predefined codes:

| Code             | Message          | Meaning                            |
| ---------------- | ---------------- | ---------------------------------- |
| -32700           | Parse error      | Invalid JSON received              |
| -32600           | Invalid Request  | JSON is not a valid Request object |
| -32601           | Method not found | Method does not exist              |
| -32602           | Invalid params   | Invalid method parameters          |
| -32603           | Internal error   | Internal JSON-RPC error            |
| -32000 to -32099 | Server error     | Implementation-defined             |

Everything outside -32768 to -32000 is free for application-defined errors.

> These are borrowed from XML-RPC's [fault codes](http://xmlrpc-epi.sourceforge.net/specs/rfc.fault_codes.php).

## Batch

A client can send multiple Requests at once by wrapping them in an Array:

```json
[
  { "jsonrpc": "2.0", "id": 1, "method": "getOutput", "params": { "path": "a.td" } },
  { "jsonrpc": "2.0", "id": 2, "method": "getOutput", "params": { "path": "b.td" } },
  { "jsonrpc": "2.0", "method": "fileChanged", "params": { "path": "c.td" } }
]
```

Rules:

- The server responds with an Array of Response objects.
- One Response per Request, except Notifications (which get no response).
- The server MAY process them concurrently, in any order.
- The Response array MAY be in any order. The client matches them up by `id`.
- If the batch itself is invalid JSON or an empty Array, the server MUST respond with a **single (non-array) Response object**.
- If every item in the batch is a Notification (meaning zero Responses), the server MUST NOT return an empty Array. It returns nothing at all.

## Extensions

Method names starting with `rpc.` are reserved for system extensions. Each extension is defined in its own spec. All are OPTIONAL.

> The spec doesn't define any built-in extensions. It just reserves the namespace. I haven't seen any widely adopted `rpc.*` extensions in practice.
