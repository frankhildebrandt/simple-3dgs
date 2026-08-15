//! First registered COLMAP image → Three.js / Spark view pose.
//! Same Rx(180) as `SplatMesh.quaternion.set(1, 0, 0, 0)` so the camera matches the splat.

use std::fs;
use std::io::{self, Cursor, Read};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::PipelineError;
use crate::project::{ProjectLayout, VIEW_JSON};

/// Viewer camera in Spark/Three space (Y-up, look along -Z).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewPose {
    pub position: [f64; 3],
    /// Three.js `(x, y, z, w)`.
    pub quaternion: [f64; 4],
}

/// Writes `output/view.json` from `colmap/sparse/0/images.bin`. Missing or fake bins are skipped.
pub fn write_output_view(layout: &ProjectLayout) {
    let Some(pose) = first_view_pose(&layout.sparse_model_dir().join("images.bin")) else {
        return;
    };
    let _ = fs::create_dir_all(layout.output_dir());
    let _ = write_view_json(&layout.output_dir().join(VIEW_JSON), &pose);
}

pub fn write_view_json(path: &Path, pose: &ViewPose) -> Result<(), PipelineError> {
    let json = serde_json::to_vec_pretty(pose)
        .map_err(|err| PipelineError::message(format!("view.json: {err}")))?;
    fs::write(path, json)?;
    Ok(())
}

pub fn read_view_json(path: &Path) -> Option<ViewPose> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Earliest image name among registered cameras, converted into Spark space.
pub fn first_view_pose(images_bin: &Path) -> Option<ViewPose> {
    let bytes = fs::read(images_bin).ok()?;
    let mut images = parse_images_binary(&bytes).ok()?;
    if images.is_empty() {
        return None;
    }
    images.sort_by(|a, b| a.name.cmp(&b.name));
    Some(colmap_to_spark(&images[0]))
}

struct ColmapImage {
    name: String,
    qw: f64,
    qx: f64,
    qy: f64,
    qz: f64,
    tx: f64,
    ty: f64,
    tz: f64,
}

fn parse_images_binary(bytes: &[u8]) -> io::Result<Vec<ColmapImage>> {
    let mut cur = Cursor::new(bytes);
    let n = read_u64(&mut cur)?;
    let mut images = Vec::with_capacity(n.min(10_000) as usize);
    for _ in 0..n {
        let _id = read_u32(&mut cur)?;
        let qw = read_f64(&mut cur)?;
        let qx = read_f64(&mut cur)?;
        let qy = read_f64(&mut cur)?;
        let qz = read_f64(&mut cur)?;
        let tx = read_f64(&mut cur)?;
        let ty = read_f64(&mut cur)?;
        let tz = read_f64(&mut cur)?;
        let _camera_id = read_u32(&mut cur)?;
        let name = read_cstring(&mut cur)?;
        let n2d = read_u64(&mut cur)?;
        let skip = n2d
            .checked_mul(24)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "points2D overflow"))?;
        let pos = cur.position().saturating_add(skip);
        if pos > bytes.len() as u64 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "truncated points2D",
            ));
        }
        cur.set_position(pos);
        images.push(ColmapImage {
            name,
            qw,
            qx,
            qy,
            qz,
            tx,
            ty,
            tz,
        });
    }
    Ok(images)
}

/// COLMAP cam-from-world → Spark camera, matching the splat's Rx(180).
fn colmap_to_spark(image: &ColmapImage) -> ViewPose {
    let r_cfw = quat_to_mat(image.qw, image.qx, image.qy, image.qz);
    let r_wfc = transpose(r_cfw);
    let t = [image.tx, image.ty, image.tz];
    let center = mulv(r_wfc, t);
    let center = [-center[0], -center[1], -center[2]];
    let rx = RX180;
    let r = mul(mul(rx, r_wfc), rx);
    let position = mulv(rx, center);
    let q = mat_to_quat(r);
    ViewPose {
        position,
        quaternion: q,
    }
}

type Mat3 = [[f64; 3]; 3];

const RX180: Mat3 = [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]];

fn quat_to_mat(w: f64, x: f64, y: f64, z: f64) -> Mat3 {
    let xx = x * x;
    let yy = y * y;
    let zz = z * z;
    let xy = x * y;
    let xz = x * z;
    let yz = y * z;
    let wx = w * x;
    let wy = w * y;
    let wz = w * z;
    [
        [1.0 - 2.0 * (yy + zz), 2.0 * (xy - wz), 2.0 * (xz + wy)],
        [2.0 * (xy + wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz - wx)],
        [2.0 * (xz - wy), 2.0 * (yz + wx), 1.0 - 2.0 * (xx + yy)],
    ]
}

fn mat_to_quat(m: Mat3) -> [f64; 4] {
    let trace = m[0][0] + m[1][1] + m[2][2];
    if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        let w = 0.25 * s;
        let x = (m[2][1] - m[1][2]) / s;
        let y = (m[0][2] - m[2][0]) / s;
        let z = (m[1][0] - m[0][1]) / s;
        return [x, y, z, w];
    }
    if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
        let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
        [
            0.25 * s,
            (m[0][1] + m[1][0]) / s,
            (m[0][2] + m[2][0]) / s,
            (m[2][1] - m[1][2]) / s,
        ]
    } else if m[1][1] > m[2][2] {
        let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
        [
            (m[0][1] + m[1][0]) / s,
            0.25 * s,
            (m[1][2] + m[2][1]) / s,
            (m[0][2] - m[2][0]) / s,
        ]
    } else {
        let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
        [
            (m[0][2] + m[2][0]) / s,
            (m[1][2] + m[2][1]) / s,
            0.25 * s,
            (m[1][0] - m[0][1]) / s,
        ]
    }
}

fn transpose(m: Mat3) -> Mat3 {
    [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ]
}

fn mul(a: Mat3, b: Mat3) -> Mat3 {
    let mut out = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    out
}

fn mulv(m: Mat3, v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn read_u64(cur: &mut Cursor<&[u8]>) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    cur.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_u32(cur: &mut Cursor<&[u8]>) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    cur.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_f64(cur: &mut Cursor<&[u8]>) -> io::Result<f64> {
    let mut buf = [0u8; 8];
    cur.read_exact(&mut buf)?;
    Ok(f64::from_le_bytes(buf))
}

fn read_cstring(cur: &mut Cursor<&[u8]>) -> io::Result<String> {
    let mut bytes = Vec::new();
    loop {
        let mut one = [0u8; 1];
        cur.read_exact(&mut one)?;
        if one[0] == 0 {
            break;
        }
        bytes.push(one[0]);
        if bytes.len() > 4096 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "image name too long",
            ));
        }
    }
    String::from_utf8(bytes).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{ProjectLayout, VIEW_JSON};
    use std::io::Write;
    use tempfile::tempdir;

    fn push_image(buf: &mut Vec<u8>, id: u32, q: [f64; 4], t: [f64; 3], name: &str) {
        buf.extend_from_slice(&id.to_le_bytes());
        for v in q {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        for v in t {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(name.as_bytes());
        buf.push(0);
        buf.extend_from_slice(&0u64.to_le_bytes());
    }

    fn images_bin(images: &[(u32, [f64; 4], [f64; 3], &str)]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(images.len() as u64).to_le_bytes());
        for (id, q, t, name) in images {
            push_image(&mut buf, *id, *q, *t, name);
        }
        buf
    }

    #[test]
    fn identity_pose_stays_at_origin_with_identity_quat() {
        let image = ColmapImage {
            name: "frame_00001.jpg".into(),
            qw: 1.0,
            qx: 0.0,
            qy: 0.0,
            qz: 0.0,
            tx: 0.0,
            ty: 0.0,
            tz: 0.0,
        };
        let pose = colmap_to_spark(&image);
        assert!(pose.position.iter().all(|v| v.abs() < 1e-9));
        assert!((pose.quaternion[0]).abs() < 1e-9);
        assert!((pose.quaternion[1]).abs() < 1e-9);
        assert!((pose.quaternion[2]).abs() < 1e-9);
        assert!((pose.quaternion[3] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn translation_flips_y_and_z_into_spark_space() {
        let image = ColmapImage {
            name: "a.jpg".into(),
            qw: 1.0,
            qx: 0.0,
            qy: 0.0,
            qz: 0.0,
            tx: 1.0,
            ty: 2.0,
            tz: 3.0,
        };
        let pose = colmap_to_spark(&image);
        assert!((pose.position[0] + 1.0).abs() < 1e-9);
        assert!((pose.position[1] - 2.0).abs() < 1e-9);
        assert!((pose.position[2] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn first_view_pose_picks_earliest_name() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("images.bin");
        let bytes = images_bin(&[
            (2, [1.0, 0.0, 0.0, 0.0], [0.0, 0.0, 5.0], "frame_00002.jpg"),
            (1, [1.0, 0.0, 0.0, 0.0], [1.0, 2.0, 3.0], "frame_00001.jpg"),
        ]);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(&bytes).unwrap();
        let pose = first_view_pose(&path).unwrap();
        assert!((pose.position[0] + 1.0).abs() < 1e-9);
        assert!((pose.position[1] - 2.0).abs() < 1e-9);
        assert!((pose.position[2] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn garbage_bin_yields_no_pose() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("images.bin");
        fs::write(&path, b"images.bin").unwrap();
        assert!(first_view_pose(&path).is_none());
    }

    #[test]
    fn write_output_view_skips_garbage_and_writes_valid() {
        let dir = tempdir().unwrap();
        let layout = ProjectLayout::new(dir.path());
        layout.create().unwrap();
        fs::create_dir_all(layout.sparse_model_dir()).unwrap();
        fs::write(layout.sparse_model_dir().join("images.bin"), b"images.bin").unwrap();
        write_output_view(&layout);
        assert!(!layout.output_dir().join(VIEW_JSON).is_file());

        fs::write(
            layout.sparse_model_dir().join("images.bin"),
            images_bin(&[(1, [1.0, 0.0, 0.0, 0.0], [1.0, 2.0, 3.0], "frame_00001.jpg")]),
        )
        .unwrap();
        write_output_view(&layout);
        let pose = read_view_json(&layout.output_dir().join(VIEW_JSON)).unwrap();
        assert!((pose.position[0] + 1.0).abs() < 1e-9);
        assert!((pose.position[1] - 2.0).abs() < 1e-9);
        assert!((pose.position[2] - 3.0).abs() < 1e-9);
    }
}
