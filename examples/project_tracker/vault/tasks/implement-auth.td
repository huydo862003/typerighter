---
_type: Task
_label: "Implement authentication flow"
status: "in progress"
priority: "high"
blocked: true
due: "2026-07-10"
project: fref("/projects/website-redesign.td")
assignee: fref("/people/alice.td")
tags:
  - "security"
  - "authentication"
  - "jwt"
  - "redis"
  - "session-management"
  - "rate-limiting"
  - "backend"
  - "api"
---

:::td-status-badge{status="blocked"}
Blocked: awaiting security team sign-off on session token format.
:::

Covers login, logout, session management, and password reset.
OAuth integration (GitHub, Google) is out of scope for this iteration.

## Scope

| Feature | Included |
|---------|----------|
| Email/password login | Yes |
| Session management | Yes |
| Password reset | Yes |
| OAuth (GitHub) | No |
| OAuth (Google) | No |

## Implementation Notes

Use short-lived JWT access tokens with a refresh token rotation strategy.

The token payload structure:

```json{2,5}
{
  "sub": "user-id",
  "exp": 1720000000,
  "iat": 1719996400,
  "scope": "read write"
}
```

Session storage in Redis:

```bash
redis-cli SET "session:abc123" '{"userId":"u1","refresh":"tok"}' EX 2592000
```

::: details Rate limiting strategy
Rate-limit login attempts to 10 per minute per IP using a sliding window counter.

```python
def check_rate_limit(ip: str) -> bool:
    key = f"login_attempts:{ip}"
    current = redis.incr(key)
    if current == 1:
        redis.expire(key, 60)
    return current <= 10
```
:::

## RFC Reference

> The JWT spec (RFC 7519) requires the `exp` claim to be a NumericDate:
>
> ```
> NumericDate: a JSON numeric value representing the number
> of seconds from 1970-01-01T00:00:00Z UTC
> ```
>
> Validate on every request, do not cache decoded claims across requests.

## Endpoints

- `POST /auth/login` accepts email and password, issues a short-lived access token
  and a long-lived refresh token. The access token expires in 15 minutes.

  ```json
  { "access_token": "...", "refresh_token": "...", "expires_in": 900 }
  ```

- `POST /auth/refresh` verifies the refresh token, revokes the old one, and issues
  a new pair. This implements refresh token rotation to limit replay attacks.

  ```json
  { "access_token": "...", "refresh_token": "..." }
  ```

- `POST /auth/logout` deletes the session entry from Redis immediately.

  ```bash
  redis-cli DEL "session:abc123"
  ```

## Blockers

::: danger Blocked
Session token format pending sign-off from security team.
:::

- Need final decision on cookie vs `Authorization` header for token transport
- Security audit for the password reset flow

## Complexity Estimate

The estimated effort uses a weighted formula:

$$
  E = \sum_{i=1}^{n} w_i \cdot c_i
$$

where $w_i$ is the weight factor and $c_i$ is the complexity score for each sub-task.
