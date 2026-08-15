//! Capture geolocation from video container tags and still EXIF.

use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use exif::{In, Reader, Tag, Value};
use serde::{Deserialize, Serialize};

use crate::project::is_image;

const MAX_EXIF_SAMPLES: usize = 20;

/// WGS84 fix extracted from a capture. `source` says which tag family produced it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoFix {
    pub lat: f64,
    pub lon: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<f64>,
    pub source: GeoSource,
}

/// Which metadata family produced a [`GeoFix`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GeoSource {
    Quicktime,
    Ffmetadata,
    Exif,
}

/// Parses ISO 6709 (`+52.52+013.40+12/` and degree/minute variants).
pub fn parse_iso6709(input: &str) -> Option<GeoFix> {
    let trimmed = input.trim().trim_matches('"').trim_end_matches('/');
    let (lat_tok, rest) = next_signed(trimmed)?;
    let (lon_tok, rest) = next_signed(rest)?;
    let alt = if rest.is_empty() {
        None
    } else {
        let (alt_tok, leftover) = next_signed(rest)?;
        if !leftover.is_empty() {
            return None;
        }
        Some(parse_signed_float(alt_tok)?)
    };
    let lat = parse_coord(lat_tok, false)?;
    let lon = parse_coord(lon_tok, true)?;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return None;
    }
    Some(GeoFix {
        lat,
        lon,
        alt,
        source: GeoSource::Ffmetadata,
    })
}

/// Scans FFmpeg ffmetadata / `-i` dumps for QuickTime ISO6709 and `location=` tags.
pub fn geo_from_ffmetadata(text: &str) -> Option<GeoFix> {
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.contains("iso6709") || lower.contains("location") {
            if let Some(value) = tag_value(line) {
                if let Some(mut fix) = parse_iso6709(value) {
                    fix.source = if lower.contains("iso6709") {
                        GeoSource::Quicktime
                    } else {
                        GeoSource::Ffmetadata
                    };
                    return Some(fix);
                }
            }
        }
        if let Some(mut fix) = find_iso6709(line) {
            fix.source = GeoSource::Quicktime;
            return Some(fix);
        }
    }
    None
}

/// Reads GPS EXIF from a JPEG (or other still kamadak-exif understands).
pub fn geo_from_jpeg(path: &Path) -> Option<GeoFix> {
    let file = File::open(path).ok()?;
    let exif = Reader::new()
        .read_from_container(&mut BufReader::new(file))
        .ok()?;
    gps_from_exif(&exif)
}

/// Averages GPS from the first stills that carry EXIF, up to [`MAX_EXIF_SAMPLES`].
pub fn geo_from_image_dir(dir: &Path) -> Option<GeoFix> {
    if !dir.is_dir() {
        return None;
    }
    let mut paths: Vec<_> = fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file() && is_image(path))
        .collect();
    paths.sort();
    let mut lats = Vec::new();
    let mut lons = Vec::new();
    let mut alts = Vec::new();
    for path in paths {
        let Some(fix) = geo_from_jpeg(&path) else {
            continue;
        };
        lats.push(fix.lat);
        lons.push(fix.lon);
        if let Some(alt) = fix.alt {
            alts.push(alt);
        }
        if lats.len() >= MAX_EXIF_SAMPLES {
            break;
        }
    }
    if lats.is_empty() {
        return None;
    }
    Some(GeoFix {
        lat: mean(&lats),
        lon: mean(&lons),
        alt: if alts.is_empty() {
            None
        } else {
            Some(mean(&alts))
        },
        source: GeoSource::Exif,
    })
}

fn gps_from_exif(exif: &exif::Exif) -> Option<GeoFix> {
    let lat_ref = field_ascii(exif, Tag::GPSLatitudeRef)?;
    let lon_ref = field_ascii(exif, Tag::GPSLongitudeRef)?;
    let lat = field_rationals(exif, Tag::GPSLatitude).and_then(|r| dms_to_deg(&r))?;
    let lon = field_rationals(exif, Tag::GPSLongitude).and_then(|r| dms_to_deg(&r))?;
    let south = lat_ref.starts_with('S') || lat_ref.starts_with('s');
    let west = lon_ref.starts_with('W') || lon_ref.starts_with('w');
    let alt = field_rationals(exif, Tag::GPSAltitude)
        .and_then(|r| r.first().map(rational_f64))
        .map(|value| {
            let below = field_u8(exif, Tag::GPSAltitudeRef).unwrap_or(0) != 0;
            if below {
                -value
            } else {
                value
            }
        });
    Some(GeoFix {
        lat: if south { -lat } else { lat },
        lon: if west { -lon } else { lon },
        alt,
        source: GeoSource::Exif,
    })
}

fn field_ascii(exif: &exif::Exif, tag: Tag) -> Option<String> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    match &field.value {
        Value::Ascii(chunks) => chunks
            .first()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .map(|s| s.trim_end_matches('\0').to_string()),
        other => Some(other.display_as(tag).to_string()),
    }
}

fn field_rationals(exif: &exif::Exif, tag: Tag) -> Option<Vec<exif::Rational>> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    match &field.value {
        Value::Rational(values) => Some(values.clone()),
        _ => None,
    }
}

fn field_u8(exif: &exif::Exif, tag: Tag) -> Option<u8> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    match &field.value {
        Value::Byte(values) => values.first().copied(),
        _ => None,
    }
}

fn dms_to_deg(parts: &[exif::Rational]) -> Option<f64> {
    if parts.len() < 3 {
        return None;
    }
    Some(
        rational_f64(&parts[0]) + rational_f64(&parts[1]) / 60.0 + rational_f64(&parts[2]) / 3600.0,
    )
}

fn rational_f64(value: &exif::Rational) -> f64 {
    if value.denom == 0 {
        0.0
    } else {
        value.num as f64 / value.denom as f64
    }
}

fn next_signed(input: &str) -> Option<(&str, &str)> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    let sign = s.chars().next()?;
    if sign != '+' && sign != '-' {
        return None;
    }
    let rest = &s[sign.len_utf8()..];
    let end = rest
        .find(['+', '-'])
        .map(|i| sign.len_utf8() + i)
        .unwrap_or(s.len());
    if end <= sign.len_utf8() {
        return None;
    }
    Some((&s[..end], &s[end..]))
}

fn parse_coord(token: &str, is_lon: bool) -> Option<f64> {
    let sign = if token.starts_with('-') { -1.0 } else { 1.0 };
    let num = token.trim_start_matches(['+', '-']);
    if num.is_empty() {
        return None;
    }
    let int_part = num.split_once('.').map(|(i, _)| i).unwrap_or(num);
    if int_part.is_empty() || !int_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let max_deg_digits = if is_lon { 3 } else { 2 };
    let value = match int_part.len() {
        n if n <= max_deg_digits => num.parse::<f64>().ok()?,
        n if n == max_deg_digits + 2 => {
            let deg: f64 = int_part[..max_deg_digits].parse().ok()?;
            let min = rest_float(
                &int_part[max_deg_digits..],
                num.split_once('.').map(|(_, f)| f),
            )?;
            deg + min / 60.0
        }
        n if n == max_deg_digits + 4 => {
            let deg: f64 = int_part[..max_deg_digits].parse().ok()?;
            let min: f64 = int_part[max_deg_digits..max_deg_digits + 2].parse().ok()?;
            let sec = rest_float(
                &int_part[max_deg_digits + 2..],
                num.split_once('.').map(|(_, f)| f),
            )?;
            deg + min / 60.0 + sec / 3600.0
        }
        _ => return None,
    };
    Some(sign * value)
}

fn rest_float(int_rest: &str, frac: Option<&str>) -> Option<f64> {
    let text = match frac {
        Some(frac) => format!("{int_rest}.{frac}"),
        None => int_rest.to_string(),
    };
    text.parse().ok()
}

fn parse_signed_float(token: &str) -> Option<f64> {
    token.parse().ok()
}

fn tag_value(line: &str) -> Option<&str> {
    line.split_once('=')
        .or_else(|| line.split_once(':'))
        .map(|(_, value)| value.trim())
}

fn find_iso6709(line: &str) -> Option<GeoFix> {
    let bytes = line.as_bytes();
    for (i, ch) in line.char_indices() {
        if ch != '+' && ch != '-' {
            continue;
        }
        if i + 1 >= bytes.len() || !bytes[i + 1].is_ascii_digit() {
            continue;
        }
        let slice = &line[i..];
        let end = slice
            .find(|c: char| c.is_ascii_whitespace() || c == '"' || c == ',')
            .unwrap_or(slice.len());
        if let Some(fix) = parse_iso6709(&slice[..end]) {
            return Some(fix);
        }
    }
    None
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn iso6709_iphone_decimal_degrees() {
        let fix = parse_iso6709("+52.520008+013.404954+012.345/").unwrap();
        assert!((fix.lat - 52.520008).abs() < 1e-6);
        assert!((fix.lon - 13.404954).abs() < 1e-6);
        assert!((fix.alt.unwrap() - 12.345).abs() < 1e-6);
    }

    #[test]
    fn iso6709_negative_hemisphere() {
        let fix = parse_iso6709("-33.8688+151.2093/").unwrap();
        assert!((fix.lat + 33.8688).abs() < 1e-4);
        assert!((fix.lon - 151.2093).abs() < 1e-4);
        assert!(fix.alt.is_none());
    }

    #[test]
    fn iso6709_rejects_garbage() {
        assert!(parse_iso6709("Berlin").is_none());
        assert!(parse_iso6709("+99.0+013.0/").is_none());
    }

    #[test]
    fn ffmetadata_reads_quicktime_tag() {
        let dump = "\
;FFMETADATA1
major_brand=qt
com.apple.quicktime.location.ISO6709=+52.520008+013.404954+012.345/
";
        let fix = geo_from_ffmetadata(dump).unwrap();
        assert_eq!(fix.source, GeoSource::Quicktime);
        assert!((fix.lat - 52.520008).abs() < 1e-6);
        assert!((fix.lon - 13.404954).abs() < 1e-6);
    }

    #[test]
    fn ffmetadata_reads_location_equals() {
        let dump = "location=+48.137154+011.576124/\n";
        let fix = geo_from_ffmetadata(dump).unwrap();
        assert_eq!(fix.source, GeoSource::Ffmetadata);
        assert!((fix.lat - 48.137154).abs() < 1e-6);
    }

    #[test]
    fn ffmetadata_ignores_named_place() {
        assert!(geo_from_ffmetadata("location=Berlin\n").is_none());
    }

    #[test]
    fn jpeg_exif_gps_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("shot.jpg");
        let bytes = jpeg_with_gps(52.52, 13.405, Some(12.0));
        File::create(&path).unwrap().write_all(&bytes).unwrap();
        let fix = geo_from_jpeg(&path).expect("EXIF GPS");
        assert_eq!(fix.source, GeoSource::Exif);
        assert!((fix.lat - 52.52).abs() < 1e-4);
        assert!((fix.lon - 13.405).abs() < 1e-4);
        assert!((fix.alt.unwrap() - 12.0).abs() < 1e-3);
    }

    #[test]
    fn image_dir_averages_first_fixes() {
        let dir = tempdir().unwrap();
        let a = jpeg_with_gps(10.0, 20.0, None);
        let b = jpeg_with_gps(12.0, 24.0, None);
        fs::write(dir.path().join("a.jpg"), a).unwrap();
        fs::write(dir.path().join("b.jpg"), b).unwrap();
        fs::write(dir.path().join("notes.txt"), b"nope").unwrap();
        let fix = geo_from_image_dir(dir.path()).unwrap();
        assert!((fix.lat - 11.0).abs() < 1e-4);
        assert!((fix.lon - 22.0).abs() < 1e-4);
    }

    /// Builds a 1×1 JPEG whose APP1 EXIF IFD carries GPSLatitude/Longitude.
    fn jpeg_with_gps(lat: f64, lon: f64, alt: Option<f64>) -> Vec<u8> {
        let exif = gps_exif(lat, lon, alt);
        let mut jpeg = Vec::new();
        jpeg.extend_from_slice(&[0xFF, 0xD8]);
        jpeg.extend_from_slice(&[0xFF, 0xE1]);
        let len = (exif.len() + 2) as u16;
        jpeg.extend_from_slice(&len.to_be_bytes());
        jpeg.extend_from_slice(&exif);
        jpeg.extend_from_slice(MINIMAL_JPEG_TAIL);
        jpeg
    }

    fn gps_exif(lat: f64, lon: f64, alt: Option<f64>) -> Vec<u8> {
        let lat_ref = if lat < 0.0 { b"S\0" } else { b"N\0" };
        let lon_ref = if lon < 0.0 { b"W\0" } else { b"E\0" };
        let lat_dms = deg_to_dms_rationals(lat.abs());
        let lon_dms = deg_to_dms_rationals(lon.abs());
        let alt_rational = alt.unwrap_or(0.0);
        let alt_num = (alt_rational.abs() * 1000.0).round() as u32;

        let gps_count: u16 = if alt.is_some() { 6 } else { 4 };
        let ifd0_off = 8u32;
        let gps_ifd_off = ifd0_off + 2 + 12 + 4;
        let data_off = gps_ifd_off + 2 + u32::from(gps_count) * 12 + 4;

        let mut tiff = Vec::new();
        tiff.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
        tiff.extend_from_slice(&ifd0_off.to_le_bytes());

        push_u16(&mut tiff, 1);
        push_entry(&mut tiff, 0x8825, 4, 1, gps_ifd_off);
        push_u32(&mut tiff, 0);

        push_u16(&mut tiff, gps_count);
        push_entry(
            &mut tiff,
            1,
            2,
            2,
            u32::from_le_bytes([lat_ref[0], lat_ref[1], 0, 0]),
        );
        push_entry(&mut tiff, 2, 5, 3, data_off);
        push_entry(
            &mut tiff,
            3,
            2,
            2,
            u32::from_le_bytes([lon_ref[0], lon_ref[1], 0, 0]),
        );
        push_entry(&mut tiff, 4, 5, 3, data_off + 24);
        if alt.is_some() {
            push_entry(&mut tiff, 5, 1, 1, 0);
            push_entry(&mut tiff, 6, 5, 1, data_off + 48);
        }
        push_u32(&mut tiff, 0);

        for (num, den) in lat_dms {
            push_u32(&mut tiff, num);
            push_u32(&mut tiff, den);
        }
        for (num, den) in lon_dms {
            push_u32(&mut tiff, num);
            push_u32(&mut tiff, den);
        }
        if alt.is_some() {
            push_u32(&mut tiff, alt_num);
            push_u32(&mut tiff, 1000);
        }

        let mut exif = b"Exif\0\0".to_vec();
        exif.extend_from_slice(&tiff);
        exif
    }

    fn deg_to_dms_rationals(deg: f64) -> [(u32, u32); 3] {
        let d = deg.floor();
        let min_f = (deg - d) * 60.0;
        let m = min_f.floor();
        let s = (min_f - m) * 60.0;
        [
            (d as u32, 1),
            (m as u32, 1),
            ((s * 10_000.0).round() as u32, 10_000),
        ]
    }

    fn push_entry(buf: &mut Vec<u8>, tag: u16, typ: u16, count: u32, value: u32) {
        push_u16(buf, tag);
        push_u16(buf, typ);
        push_u32(buf, count);
        push_u32(buf, value);
    }

    fn push_u16(buf: &mut Vec<u8>, value: u16) {
        buf.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(buf: &mut Vec<u8>, value: u32) {
        buf.extend_from_slice(&value.to_le_bytes());
    }

    /// SOF/SOS/EOI for a 1×1 grey JPEG so EXIF readers accept the container.
    const MINIMAL_JPEG_TAIL: &[u8] = &[
        0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07,
        0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F,
        0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22,
        0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27,
        0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00,
        0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xDA, 0x00, 0x08, 0x01,
        0x01, 0x00, 0x00, 0x3F, 0x00, 0x7F, 0xFF, 0xD9,
    ];
}
