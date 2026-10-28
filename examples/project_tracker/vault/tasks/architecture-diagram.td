---
_type: Task
_label: "Architecture diagram"
status: "in progress"
priority: "medium"
project: fref("/projects/website-redesign.td")
---

## System Architecture

The following D2 diagram shows the high-level architecture:

```d2{title="System Architecture"}
Web Client -> API Gateway: HTTPS
API Gateway -> Auth Service: validate token
API Gateway -> App Server: forward request
App Server -> Database: query
App Server -> Cache: read/write
Database -> App Server: result
Cache -> App Server: cached result
App Server -> API Gateway: response
API Gateway -> Web Client: JSON
```

## Deployment Flow

```mermaid{title="CI/CD Pipeline"}
graph LR
    A[Developer] -->|push| B[GitHub]
    B -->|trigger| C[CI/CD]
    C -->|build| D[Docker Image]
    D -->|deploy| E[Production]
```

## Token Payload

```json{title="token-payload.json"}
{
  "sub": "user-id",
  "exp": 1720000000,
  "iat": 1719996400,
  "scope": "read write"
}
```

## Session Storage

```bash{title="Redis session setup"}
redis-cli SET "session:abc123" '{"userId":"u1","refresh":"tok"}' EX 2592000
```

## Logo

![Typedown logo](/typedown.svg "The Typedown project logo")
