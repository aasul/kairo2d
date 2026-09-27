use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_RESOURCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, thiserror::Error)]
#[error("resource identifier space exhausted")]
pub struct HandleExhausted;

fn next_id() -> Result<u64, HandleExhausted> {
    NEXT_RESOURCE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| HandleExhausted)
}

// IDs are never recycled, including across project reloads. A stale handle cannot
// silently start referring to an unrelated resource in a replacement manager.
macro_rules! resource_handle {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub struct $name(u64);

        impl $name {
            pub fn allocate() -> Result<Self, HandleExhausted> {
                next_id().map(Self)
            }
        }
    };
}

resource_handle!(TextureHandle);
resource_handle!(SoundHandle);
resource_handle!(VoiceHandle);
resource_handle!(BodyHandle);
resource_handle!(FontHandle);

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn identifiers_do_not_repeat_across_threads() {
        let threads: Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(|| (0..1000).map(|_| next_id().unwrap()).collect::<Vec<_>>())
            })
            .collect();
        let ids: Vec<_> = threads
            .into_iter()
            .flat_map(|t| t.join().unwrap())
            .collect();
        assert_eq!(ids.iter().copied().collect::<HashSet<_>>().len(), ids.len());
    }

    #[test]
    fn handles_are_stable_map_keys() {
        let a = TextureHandle::allocate().unwrap();
        let b = TextureHandle::allocate().unwrap();
        let mut textures = HashMap::new();
        textures.insert(a, "player");
        textures.insert(b, "background");
        let copied = a;
        assert_eq!(textures.get(&copied), Some(&"player"));
        assert_eq!(textures.remove(&a), Some("player"));
        assert_eq!(textures.get(&b), Some(&"background"));
        assert!(!textures.contains_key(&copied));
    }
}
