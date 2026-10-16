use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

pub const LIFETIME: Duration = Duration::from_secs(8 * 60 * 60);
const MAX_CLIENTS: usize = 128;
struct Grant {
    name: String,
    key: String,
    created: Instant,
    denied: bool,
}
#[derive(Default)]
pub struct Registry {
    grants: BTreeMap<String, Grant>,
    next: u64,
}
pub struct Access {
    pub key: String,
    pub token: String,
    pub denied: bool,
    pub expires_in_seconds: u64,
}
pub fn principal(client: &str) -> &str {
    client
        .rsplit_once(" · сессия ")
        .filter(|(_, suffix)| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()))
        .map_or(client, |(name, _)| name)
}
fn valid(token: &str) -> bool {
    token.len() == 32
        && token
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
impl Registry {
    pub fn reap(&mut self) -> Vec<String> {
        let mut removed = Vec::new();
        self.grants.retain(|_, grant| {
            if grant.created.elapsed() >= LIFETIME {
                removed.push(grant.key.clone());
                false
            } else {
                true
            }
        });
        removed
    }
    pub fn clear(&mut self) {
        self.grants.clear();
    }
    pub fn deny(&mut self, key: &str) {
        for grant in self.grants.values_mut().filter(|grant| grant.key == key) {
            grant.denied = true;
        }
    }
    pub fn request(&mut self, client: &str, token: Option<&str>) -> Result<Access, String> {
        if let Some(token) = token {
            let grant = self.lookup(client, token)?;
            return Ok(Access {
                key: grant.key.clone(),
                token: token.into(),
                denied: grant.denied,
                expires_in_seconds: LIFETIME
                    .as_secs()
                    .saturating_sub(grant.created.elapsed().as_secs()),
            });
        }
        if self.grants.len() >= MAX_CLIENTS {
            return Err("too_large: лимит разрешений MCP; отзовите доступ в настройках".into());
        }
        let next = self
            .next
            .checked_add(1)
            .ok_or("too_large: исчерпаны ID разрешений")?;
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce)
            .map_err(|e| format!("internal: ключ доступа не создан: {e}"))?;
        let token = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
        if self.grants.contains_key(&token) {
            return Err("internal: повтор ключа доступа; повторите запрос".into());
        }
        let name = principal(client).to_string();
        let key = format!("{name} · доступ {next}");
        self.grants.insert(
            token.clone(),
            Grant {
                name,
                key: key.clone(),
                created: Instant::now(),
                denied: false,
            },
        );
        self.next = next;
        Ok(Access {
            key,
            token,
            denied: false,
            expires_in_seconds: LIFETIME.as_secs(),
        })
    }
    fn lookup(&self, client: &str, token: &str) -> Result<&Grant, String> {
        if !valid(token) {
            return Err("invalid_args: client_access должен содержать32 hex символа".into());
        }
        self.grants.get(token).filter(|grant|grant.name==principal(client) && grant.created.elapsed()<LIFETIME)
            .ok_or_else(||"permission_denied: неизвестный, истёкший или чужой client_access; вызовите request_access".into())
    }
    pub fn resolve(&self, client: &str, token: &str) -> Result<String, String> {
        let grant = self.lookup(client, token)?;
        if grant.denied {
            return Err("permission_denied: доступ запрещён в TrineDocs".into());
        }
        Ok(grant.key.clone())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handle_survives_session_changes_but_is_bound_to_its_client() -> Result<(), String> {
        let mut registry = Registry::default();
        let access = registry.request("Editor · сессия 1", None)?;
        assert_eq!(
            registry.resolve("Editor · сессия 2", &access.token)?,
            access.key
        );
        assert!(!access.key.contains(&access.token));
        assert!(registry.resolve("Other · сессия 3", &access.token).is_err());
        registry.deny(&access.key);
        assert!(
            registry
                .resolve("Editor · сессия 2", &access.token)
                .is_err()
        );
        assert!(
            registry
                .request("Editor · сессия 2", Some(&access.token))?
                .denied
        );
        Ok(())
    }
    #[test]
    fn expired_and_revoked_handles_cannot_be_reused() -> Result<(), String> {
        let mut registry = Registry::default();
        let access = registry.request("Editor", None)?;
        registry
            .grants
            .get_mut(&access.token)
            .ok_or("grant")?
            .created = Instant::now() - LIFETIME;
        assert!(registry.resolve("Editor", &access.token).is_err());
        assert_eq!(registry.reap(), vec![access.key]);
        let access = registry.request("Editor", None)?;
        registry.clear();
        assert!(registry.resolve("Editor", &access.token).is_err());
        Ok(())
    }
}
