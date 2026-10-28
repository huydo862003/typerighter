---
_type: Task
_label: "Architecture diagram"
status: "in progress"
priority: "medium"
project: fref("/projects/website-redesign.td")
---

## System Architecture

The following D2 diagram shows the high-level architecture:

```d2
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

```mermaid
graph LR
    A[Developer] -->|push| B[GitHub]
    B -->|trigger| C[CI/CD]
    C -->|build| D[Docker Image]
    D -->|deploy| E[Production]
```
