use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub struct Snapshot<T>(Arc<T>);

impl<T> Snapshot<T> {
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Clone for Snapshot<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> Deref for Snapshot<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone> DerefMut for Snapshot<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl<T> From<T> for Snapshot<T> {
    fn from(value: T) -> Self {
        Self(Arc::new(value))
    }
}

impl<T: Default> Default for Snapshot<T> {
    fn default() -> Self {
        T::default().into()
    }
}

impl<T: Serialize> Serialize for Snapshot<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.deref().serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Snapshot<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icy_board::{IcyBoard, icb_config::IcbConfig};

    #[test]
    fn cloning_and_serialization_do_not_require_clone_on_the_value() {
        #[derive(Default, Serialize, Deserialize)]
        struct Value {
            number: usize,
        }

        let snapshot = Snapshot::<Value>::default();
        let copy = snapshot.clone();
        assert!(snapshot.ptr_eq(&copy));
        assert_eq!(copy.number, 0);
        let encoded = toml::to_string(&copy).unwrap();
        assert_eq!(encoded, "number = 0\n");
        let decoded: Snapshot<Value> = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded.number, 0);
        assert!(!snapshot.ptr_eq(&decoded));
    }

    #[test]
    fn mutation_detaches_only_when_shared() {
        let mut current: Snapshot<Vec<String>> = vec!["original".into()].into();
        let original = current.clone();
        assert!(current.ptr_eq(&original));

        current[0].push_str(" edited");
        assert!(!current.ptr_eq(&original));
        assert_eq!(original[0], "original");
        assert_eq!(current[0], "original edited");

        let allocation = &*current as *const Vec<String>;
        current.push("second".into());
        assert_eq!(allocation, &*current as *const Vec<String>);
        assert_eq!(original.len(), 1);
    }

    #[test]
    fn configuration_snapshots_keep_nested_values_across_edits() {
        let mut board = IcyBoard::new();
        board.config.board.name = "Original board".into();
        board.config.func_keys[0] = "Original key".into();
        board.config.ppl_http.allowed_origins = vec!["https://original.example".into()];
        board.config.paths.help_path = "help".into();
        let original = board.configuration_snapshot();
        assert!(original.ptr_eq(&board.config));
        assert!(original.ptr_eq(&board.configuration_snapshot()));

        board.config.board.name = "Edited board".into();
        board.config.func_keys[0].push_str(" edited");
        board.config.ppl_http.allowed_origins[0].push_str("/edited");
        board.root_path = std::env::temp_dir().join("snapshot-board");
        board.resolve_paths();

        assert!(!original.ptr_eq(&board.config));
        assert_eq!(original.board.name, "Original board");
        assert_eq!(original.func_keys[0], "Original key");
        assert_eq!(original.ppl_http.allowed_origins, ["https://original.example"]);
        assert_eq!(original.paths.help_path, std::path::PathBuf::from("help"));
        assert_eq!(board.config.board.name, "Edited board");
        assert_eq!(board.config.func_keys[0], "Original key edited");
        assert_eq!(board.config.ppl_http.allowed_origins, ["https://original.example/edited"]);
        assert_eq!(board.config.paths.help_path, board.root_path.join("help"));

        let mut independent = original.clone();
        independent.board.name = "Independent board".into();
        assert_eq!(original.board.name, "Original board");
        assert_eq!(board.config.board.name, "Edited board");
        assert!(!independent.ptr_eq(&original));
    }

    #[test]
    fn configuration_toml_schema_is_unchanged() {
        let mut config = IcbConfig::new();
        config.board.name = "Snapshot schema test".into();
        config.func_keys[0] = "test key".into();
        config.ppl_http.allowed_origins = vec!["https://example.org".into()];
        let legacy = toml::to_string(&config).unwrap();
        let snapshot: Snapshot<IcbConfig> = config.into();
        assert_eq!(toml::to_string(&snapshot).unwrap(), legacy);

        let decoded: Snapshot<IcbConfig> = toml::from_str(&legacy).unwrap();
        assert_eq!(toml::to_string(&decoded).unwrap(), legacy);
        let plain: IcbConfig = toml::from_str(&toml::to_string(&decoded).unwrap()).unwrap();
        assert_eq!(toml::to_string(&plain).unwrap(), legacy);
    }
}
