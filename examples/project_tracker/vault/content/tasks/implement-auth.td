---
_type: Task
title: "Implement authentication flow"
status: "in_progress"
priority: "high"
blocked: true
due: "2026-07-10"
project: fref("projects/website-redesign.td")
assignee: fref("people/alice.td")
---

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

## Blockers

::: danger Blocked
Session token format pending sign-off from security team.
:::

- Need final decision on cookie vs `Authorization` header for token transport
- Security audit for the password reset flow

## Complexity Estimate

The estimated effort uses a weighted formula:

$$E = \sum_{i=1}^{n} w_i \cdot c_i$$

where $w_i$ is the weight factor and $c_i$ is the complexity score for each sub-task.
