use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncWriteExt, sync::Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConnection {
    pub from_id: String,
    pub from_side: PinSide,
    pub to_id: String,
    pub to_side: PinSide,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SpatialTopology {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
    pub positions: HashMap<String, NodePosition>,
    pub connections: Vec<ServerConnection>,
}

impl Default for SpatialTopology {
    fn default() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
            positions: HashMap::new(),
            connections: Vec::new(),
        }
    }
}

impl SpatialTopology {
    pub fn normalize(&mut self) {
        self.zoom = if self.zoom.is_finite() {
            self.zoom.clamp(0.35, 2.0)
        } else {
            1.0
        };
        if !self.pan_x.is_finite() {
            self.pan_x = 0.0;
        }
        if !self.pan_y.is_finite() {
            self.pan_y = 0.0;
        }
        self.positions
            .retain(|_, pos| pos.x.is_finite() && pos.y.is_finite());

        let mut seen = HashSet::new();
        self.connections.retain(|connection| {
            if connection.from_id == connection.to_id {
                return false;
            }
            seen.insert(unordered_pair(&connection.from_id, &connection.to_id))
        });
    }

    pub fn ensure_positions(&mut self, ordered_ids: &[String]) -> bool {
        let mut changed = false;
        let mut slot = 0usize;
        for id in ordered_ids {
            if self.positions.contains_key(id) {
                continue;
            }
            loop {
                let col = (slot % 3) as f32;
                let row = (slot / 3) as f32;
                let x = 80.0 + col * 340.0;
                let y = 80.0 + row * 190.0;
                slot += 1;
                let overlaps = self
                    .positions
                    .values()
                    .any(|pos| (pos.x - x).abs() < 160.0 && (pos.y - y).abs() < 90.0);
                if !overlaps {
                    self.positions.insert(id.clone(), NodePosition { x, y });
                    changed = true;
                    break;
                }
            }
        }
        changed
    }

    pub fn add_connection(&mut self, connection: ServerConnection) -> bool {
        if connection.from_id == connection.to_id {
            return false;
        }
        let candidate = unordered_pair(&connection.from_id, &connection.to_id);
        if self
            .connections
            .iter()
            .any(|existing| unordered_pair(&existing.from_id, &existing.to_id) == candidate)
        {
            return false;
        }
        self.connections.push(connection);
        true
    }

    pub fn remove_connection(&mut self, index: usize) -> bool {
        if index < self.connections.len() {
            self.connections.remove(index);
            true
        } else {
            false
        }
    }

    pub fn remove_server(&mut self, id: &str) -> bool {
        let pos_removed = self.positions.remove(id).is_some();
        let before = self.connections.len();
        self.connections
            .retain(|connection| connection.from_id != id && connection.to_id != id);
        pos_removed || self.connections.len() != before
    }
}

fn unordered_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_owned(), b.to_owned())
    } else {
        (b.to_owned(), a.to_owned())
    }
}

#[derive(Clone, Default)]
pub struct TopologyStore {
    write_lock: Arc<Mutex<()>>,
    saved_revision: Arc<AtomicU64>,
}

impl TopologyStore {
    pub async fn load() -> (SpatialTopology, Self) {
        let topology = match Self::path() {
            Ok(path) => match fs::read(&path).await {
                Ok(bytes) => match serde_json::from_slice::<SpatialTopology>(&bytes) {
                    Ok(mut topology) => {
                        topology.normalize();
                        topology
                    }
                    Err(error) => {
                        eprintln!("Could not load spatial topology: {error}");
                        SpatialTopology::default()
                    }
                },
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    SpatialTopology::default()
                }
                Err(error) => {
                    eprintln!("Could not read spatial topology: {error}");
                    SpatialTopology::default()
                }
            },
            Err(error) => {
                eprintln!("Could not locate spatial topology: {error:#}");
                SpatialTopology::default()
            }
        };
        (topology, Self::default())
    }

    pub async fn save(&self, topology: SpatialTopology, revision: u64) -> Result<()> {
        let _guard = self.write_lock.lock().await;
        if revision <= self.saved_revision.load(Ordering::Acquire) {
            return Ok(());
        }

        let path = Self::path()?;
        let parent = path
            .parent()
            .context("Topology path has no parent directory")?;
        fs::create_dir_all(parent)
            .await
            .context("Could not create topology directory")?;
        let bytes =
            serde_json::to_vec_pretty(&topology).context("Could not serialize spatial topology")?;
        let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let result = async {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .await?;
            file.write_all(&bytes).await?;
            file.sync_all().await?;
            drop(file);
            fs::rename(&temporary, &path).await
        }
        .await;
        if result.is_err() {
            let _ = fs::remove_file(&temporary).await;
        }
        result.context("Could not atomically save spatial topology")?;
        self.saved_revision.store(revision, Ordering::Release);
        Ok(())
    }

    fn path() -> Result<PathBuf> {
        Ok(dirs::data_dir()
            .context("Could not find data directory")?
            .join("LocalCraft")
            .join("topology.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_clamps_zoom_cleans_non_finite_and_deduplicates_connections() {
        let mut topology = SpatialTopology {
            pan_x: f32::NAN,
            pan_y: f32::INFINITY,
            zoom: 8.0,
            positions: HashMap::from([
                ("a".into(), NodePosition { x: 10.0, y: 20.0 }),
                (
                    "bad".into(),
                    NodePosition {
                        x: f32::NAN,
                        y: 0.0,
                    },
                ),
            ]),
            connections: vec![
                ServerConnection {
                    from_id: "a".into(),
                    from_side: PinSide::Right,
                    to_id: "a".into(),
                    to_side: PinSide::Left,
                },
                ServerConnection {
                    from_id: "a".into(),
                    from_side: PinSide::Right,
                    to_id: "b".into(),
                    to_side: PinSide::Left,
                },
                ServerConnection {
                    from_id: "b".into(),
                    from_side: PinSide::Left,
                    to_id: "a".into(),
                    to_side: PinSide::Right,
                },
            ],
        };

        topology.normalize();

        assert_eq!(topology.pan_x, 0.0);
        assert_eq!(topology.pan_y, 0.0);
        assert_eq!(topology.zoom, 2.0);
        assert!(topology.positions.contains_key("a"));
        assert!(!topology.positions.contains_key("bad"));
        assert_eq!(topology.connections.len(), 1);
    }

    #[test]
    fn ensure_positions_skips_occupied_cells() {
        let mut topology = SpatialTopology::default();
        topology
            .positions
            .insert("existing".into(), NodePosition { x: 90.0, y: 90.0 });

        let changed =
            topology.ensure_positions(&["existing".into(), "second".into(), "third".into()]);

        assert!(changed);
        assert_eq!(
            topology.positions.get("existing"),
            Some(&NodePosition { x: 90.0, y: 90.0 })
        );
        assert_eq!(
            topology.positions.get("second"),
            Some(&NodePosition { x: 420.0, y: 80.0 })
        );
        assert_eq!(
            topology.positions.get("third"),
            Some(&NodePosition { x: 760.0, y: 80.0 })
        );
        assert!(!topology.ensure_positions(&["existing".into(), "second".into(), "third".into()]));
    }

    #[test]
    fn add_connection_and_remove_server_preserve_consistency() {
        let mut topology = SpatialTopology::default();
        assert!(!topology.add_connection(ServerConnection {
            from_id: "a".into(),
            from_side: PinSide::Left,
            to_id: "a".into(),
            to_side: PinSide::Right,
        }));
        assert!(topology.add_connection(ServerConnection {
            from_id: "a".into(),
            from_side: PinSide::Right,
            to_id: "b".into(),
            to_side: PinSide::Left,
        }));
        assert!(!topology.add_connection(ServerConnection {
            from_id: "b".into(),
            from_side: PinSide::Left,
            to_id: "a".into(),
            to_side: PinSide::Right,
        }));
        topology
            .positions
            .insert("a".into(), NodePosition { x: 0.0, y: 0.0 });
        topology
            .positions
            .insert("b".into(), NodePosition { x: 300.0, y: 0.0 });

        assert!(topology.remove_server("a"));
        assert!(!topology.positions.contains_key("a"));
        assert!(topology.connections.is_empty());
        assert!(topology.positions.contains_key("b"));
    }

    #[test]
    fn topology_round_trips_through_json() {
        let mut topology = SpatialTopology {
            pan_x: -45.5,
            pan_y: 12.0,
            zoom: 1.25,
            positions: HashMap::from([("s1".into(), NodePosition { x: 100.0, y: 200.0 })]),
            connections: vec![ServerConnection {
                from_id: "s1".into(),
                from_side: PinSide::Right,
                to_id: "s2".into(),
                to_side: PinSide::Left,
            }],
        };
        let json = serde_json::to_string(&topology).expect("serialize");
        let mut decoded: SpatialTopology = serde_json::from_str(&json).expect("deserialize");
        topology.normalize();
        decoded.normalize();
        assert_eq!(decoded.pan_x, topology.pan_x);
        assert_eq!(decoded.pan_y, topology.pan_y);
        assert_eq!(decoded.zoom, topology.zoom);
        assert_eq!(decoded.positions, topology.positions);
        assert_eq!(decoded.connections, topology.connections);
    }
}
