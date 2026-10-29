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

## Auth Middleware

```typescript{title="auth.ts" line-numbers}
import { NextFunction, Request, Response } from 'express';
import jwt from 'jsonwebtoken';

export function authMiddleware(req: Request, res: Response, next: NextFunction) {
  const token = req.headers.authorization?.split(' ')[1];
  if (!token) {
    return res.status(401).json({ error: 'Unauthorized' });
  }
  try {
    const payload = jwt.verify(token, process.env.JWT_SECRET!);
    req.user = payload;
    next();
  } catch {
    res.status(401).json({ error: 'Invalid token' });
  }
}
```

## Error Codes

```typescript{line-numbers=10}
export const ERROR_CODES = {
  UNAUTHORIZED: 401,
  FORBIDDEN: 403,
  NOT_FOUND: 404,
  INTERNAL: 500,
};
```

## Reference Implementation

![server bootstrap](https://github.com/vuejs/vite/blob/main/packages/vite/src/node/server/index.ts#L150-L165)

## Logo

![Typedown logo](/typedown.svg "The Typedown project logo")
