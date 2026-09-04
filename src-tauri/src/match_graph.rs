//! Match-graph quality from a COLMAP database. Weak graphs trigger rematch.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use rusqlite::{Connection, OpenFlags};

/// COLMAP `pair_id` stride (`kMaxNumImages` in 4.1.1).
const MAX_IMAGE_ID: u64 = 2_147_483_647;
const MIN_INLIERS: u32 = 15;
const MIN_COVERAGE: f32 = 0.85;
const MIN_RIGIDITY: f32 = 0.3;
const MIN_MEDIAN_DEGREE: f32 = 2.0;

/// COLMAP 4.1.1 `TwoViewGeometry::ConfigurationType`.
const CONFIG_CALIBRATED: i32 = 2;
const CONFIG_UNCALIBRATED: i32 = 3;
const CONFIG_CALIBRATED_RIG: i32 = 9;

/// One geometrically verified image pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchPair {
    pub left: u32,
    pub right: u32,
    pub inliers: u32,
    pub config: i32,
}

/// Images plus verified two-view geometries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchGraph {
    pub image_ids: Vec<u32>,
    pub pairs: Vec<MatchPair>,
}

impl MatchGraph {
    /// True when sequential matching left holes or almost no rigid pairs.
    pub fn needs_escalate(&self) -> bool {
        if self.image_ids.is_empty() {
            return false;
        }
        self.coverage() < MIN_COVERAGE
            || self.rigidity() < MIN_RIGIDITY
            || self.median_degree() < MIN_MEDIAN_DEGREE
    }

    /// Fraction of images in the largest connected component.
    pub fn coverage(&self) -> f32 {
        let n = self.image_ids.len();
        if n == 0 {
            return 1.0;
        }
        self.giant_component() as f32 / n as f32
    }

    /// Fraction of inlier pairs that are essential/fundamental, not homography.
    pub fn rigidity(&self) -> f32 {
        let verified: Vec<_> = self
            .pairs
            .iter()
            .copied()
            .filter(|pair| pair.inliers >= MIN_INLIERS)
            .collect();
        if verified.is_empty() {
            return 0.0;
        }
        let rigid = verified.iter().filter(|pair| is_rigid(pair.config)).count();
        rigid as f32 / verified.len() as f32
    }

    /// Median number of verified neighbors per image, including isolates.
    pub fn median_degree(&self) -> f32 {
        let mut degrees = self.degrees();
        if degrees.is_empty() {
            return 0.0;
        }
        degrees.sort_unstable();
        let n = degrees.len();
        if n % 2 == 1 {
            degrees[n / 2] as f32
        } else {
            (degrees[n / 2 - 1] + degrees[n / 2]) as f32 / 2.0
        }
    }

    fn giant_component(&self) -> usize {
        let adj = self.adjacency();
        let mut seen = HashSet::new();
        let mut best = 0;
        for &start in &self.image_ids {
            if !seen.insert(start) {
                continue;
            }
            let mut size = 0;
            let mut queue = VecDeque::from([start]);
            while let Some(node) = queue.pop_front() {
                size += 1;
                for &next in adj.get(&node).into_iter().flatten() {
                    if seen.insert(next) {
                        queue.push_back(next);
                    }
                }
            }
            best = best.max(size);
        }
        best
    }

    fn degrees(&self) -> Vec<u32> {
        let adj = self.adjacency();
        self.image_ids
            .iter()
            .map(|id| adj.get(id).map(|n| n.len() as u32).unwrap_or(0))
            .collect()
    }

    fn adjacency(&self) -> HashMap<u32, Vec<u32>> {
        let mut adj: HashMap<u32, Vec<u32>> =
            self.image_ids.iter().map(|id| (*id, Vec::new())).collect();
        for pair in &self.pairs {
            if pair.inliers < MIN_INLIERS {
                continue;
            }
            adj.entry(pair.left).or_default().push(pair.right);
            adj.entry(pair.right).or_default().push(pair.left);
        }
        adj
    }
}

/// Reads COLMAP `database.db`. `None` when the file is missing or not a COLMAP DB.
pub fn inspect(path: &Path) -> Option<MatchGraph> {
    if !path.is_file() {
        return None;
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    let image_ids = load_image_ids(&conn)?;
    let pairs = load_pairs(&conn)?;
    Some(MatchGraph { image_ids, pairs })
}

fn load_image_ids(conn: &Connection) -> Option<Vec<u32>> {
    let mut stmt = conn.prepare("SELECT image_id FROM images").ok()?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0)).ok()?;
    Some(
        rows.filter_map(|row| row.ok())
            .filter_map(|id| u32::try_from(id).ok())
            .collect(),
    )
}

fn load_pairs(conn: &Connection) -> Option<Vec<MatchPair>> {
    let mut stmt = conn
        .prepare("SELECT pair_id, rows, config FROM two_view_geometries")
        .ok()?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i32>(2)?,
            ))
        })
        .ok()?;
    Some(
        rows.filter_map(|row| row.ok())
            .filter_map(|(pair_id, rows, config)| {
                let inliers = u32::try_from(rows).ok()?;
                let (left, right) = pair_id_to_image_ids(pair_id as u64);
                Some(MatchPair {
                    left,
                    right,
                    inliers,
                    config,
                })
            })
            .collect(),
    )
}

fn is_rigid(config: i32) -> bool {
    matches!(
        config,
        CONFIG_CALIBRATED | CONFIG_UNCALIBRATED | CONFIG_CALIBRATED_RIG
    )
}

fn image_ids_to_pair_id(left: u32, right: u32) -> u64 {
    let (min, max) = if left < right {
        (left as u64, right as u64)
    } else {
        (right as u64, left as u64)
    };
    min * MAX_IMAGE_ID + max
}

fn pair_id_to_image_ids(pair_id: u64) -> (u32, u32) {
    let right = pair_id % MAX_IMAGE_ID;
    let left = (pair_id - right) / MAX_IMAGE_ID;
    (left as u32, right as u32)
}

/// True when the sparse model is too small to keep (fake bins yield `None` and skip).
pub fn reconstruction_too_small(
    registered: Option<u32>,
    selected: u32,
    min_model_size: u32,
) -> bool {
    registered.is_some_and(|n| n < min_model_size || n.saturating_mul(4) < selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::tempdir;

    const CALIBRATED: i32 = 2;
    const PANORAMIC: i32 = 5;
    const PLANAR_OR_PANORAMIC: i32 = 6;

    fn chain(n: u32, config: i32) -> MatchGraph {
        let image_ids: Vec<u32> = (1..=n).collect();
        let pairs = (1..n)
            .map(|i| MatchPair {
                left: i,
                right: i + 1,
                inliers: 40,
                config,
            })
            .collect();
        MatchGraph { image_ids, pairs }
    }

    #[test]
    fn pair_id_uses_colmap_max_image_id() {
        assert_eq!(image_ids_to_pair_id(1, 2), MAX_IMAGE_ID + 2);
        assert_eq!(image_ids_to_pair_id(2, 1), MAX_IMAGE_ID + 2);
        assert_eq!(pair_id_to_image_ids(MAX_IMAGE_ID + 2), (1, 2));
    }

    #[test]
    fn calibrated_neighbor_chain_stays_on_first_pass() {
        let graph = chain(10, CALIBRATED);
        assert!((graph.coverage() - 1.0).abs() < 1e-6);
        assert!((graph.rigidity() - 1.0).abs() < 1e-6);
        assert!((graph.median_degree() - 2.0).abs() < 1e-6);
        assert!(!graph.needs_escalate());
    }

    #[test]
    fn disconnected_images_need_escalate() {
        let graph = MatchGraph {
            image_ids: (1..=10).collect(),
            pairs: vec![
                MatchPair {
                    left: 1,
                    right: 2,
                    inliers: 40,
                    config: CALIBRATED,
                },
                MatchPair {
                    left: 2,
                    right: 3,
                    inliers: 40,
                    config: CALIBRATED,
                },
            ],
        };
        assert!((graph.coverage() - 0.3).abs() < 1e-6);
        assert!(graph.needs_escalate());
    }

    #[test]
    fn panoramic_chain_needs_escalate() {
        let graph = chain(10, PANORAMIC);
        assert!((graph.coverage() - 1.0).abs() < 1e-6);
        assert!(graph.rigidity() < 1e-6);
        assert!(graph.needs_escalate());
    }

    #[test]
    fn planar_or_panoramic_is_not_rigid() {
        let graph = chain(8, PLANAR_OR_PANORAMIC);
        assert!(graph.needs_escalate());
    }

    #[test]
    fn inspect_skips_garbage_files() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("database.db");
        std::fs::write(&path, b"fake-colmap-db").unwrap();
        assert!(inspect(&path).is_none());
        assert!(inspect(&dir.path().join("missing.db")).is_none());
    }

    #[test]
    fn inspect_reads_verified_pairs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("database.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE images (image_id INTEGER PRIMARY KEY, name TEXT);
             CREATE TABLE two_view_geometries (
                 pair_id INTEGER PRIMARY KEY,
                 rows INTEGER NOT NULL,
                 config INTEGER NOT NULL
             );",
        )
        .unwrap();
        for id in 1..=4 {
            conn.execute(
                "INSERT INTO images (image_id, name) VALUES (?1, ?2)",
                (id, format!("f{id}.jpg")),
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO two_view_geometries (pair_id, rows, config) VALUES (?1, 40, 2)",
            [image_ids_to_pair_id(1, 2) as i64],
        )
        .unwrap();
        drop(conn);
        let graph = inspect(&path).expect("colmap db");
        assert_eq!(graph.image_ids.len(), 4);
        assert_eq!(graph.pairs.len(), 1);
        assert_eq!(graph.pairs[0].left, 1);
        assert_eq!(graph.pairs[0].right, 2);
        assert!(graph.needs_escalate());
    }

    #[test]
    fn reconstruction_gate_skips_unreadable_counts() {
        assert!(!reconstruction_too_small(None, 40, 6));
        assert!(reconstruction_too_small(Some(2), 40, 6));
        assert!(reconstruction_too_small(Some(8), 40, 6));
        assert!(!reconstruction_too_small(Some(10), 40, 6));
        assert!(!reconstruction_too_small(Some(6), 8, 6));
    }
}
