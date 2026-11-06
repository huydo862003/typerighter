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

The token validation logic:

```c{3,5-7}
#include <jwt.h>
#include <time.h>

int validate_token(const char *token, jwt_t **out) {
    jwt_t *jwt = NULL;
    int ret = jwt_decode(&jwt, token, (unsigned char *)SECRET, strlen(SECRET));
    if (ret != 0) return -1;
    if (jwt_get_grant_int(jwt, "exp") < time(NULL)) {
        jwt_free(jwt);
        return -1;
    }
    *out = jwt;
    return 0;
}
```

Session management with refresh token rotation:

```rust{2-5}
fn rotate_refresh_token(old_token: &str, redis: &mut Connection) -> Result<String> {
    let session: String = redis.get(format!("session:{old_token}"))?;
    redis.del(format!("session:{old_token}"))?;
    let new_token = generate_token();
    redis.set_ex(format!("session:{new_token}"), &session, 2592000)?;
    Ok(new_token)
}
```

Rate limit check using a sliding window:

```cpp{4-6}
#include <chrono>
#include <unordered_map>

bool check_rate_limit(const std::string &ip, std::unordered_map<std::string, int> &attempts) {
    auto &count = attempts[ip];
    if (++count > 10) return false;
    return true;
}
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

  ```c{3-5}
  struct auth_response login(const char *email, const char *password) {
      user_t *user = db_find_user(email);
      if (!user || !verify_password(password, user->hash))
          return (struct auth_response){ .error = AUTH_INVALID };
      return issue_tokens(user->id);
  }
  ```

- `POST /auth/refresh` verifies the refresh token, revokes the old one, and issues
  a new pair. This implements refresh token rotation to limit replay attacks.

  ```rust{2-4}
  fn refresh(token: &str, redis: &mut Connection) -> Result<TokenPair> {
      let user_id: String = redis.get(&format!("refresh:{token}"))?;
      redis.del(&format!("refresh:{token}"))?;
      let pair = TokenPair::generate(&user_id);
      redis.set_ex(&format!("refresh:{}", pair.refresh), &user_id, 2592000)?;
      Ok(pair)
  }
  ```

- `POST /auth/logout` revokes the session and refresh token immediately.

  ```cpp
  void logout(const std::string &session_id, redis::Client &redis) {
      redis.del("session:" + session_id);
      redis.del("refresh:" + session_id);
  }
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
