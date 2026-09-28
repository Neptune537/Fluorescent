use std::f64::consts::PI;

/// WGS84 ellipsoid semi-major axis (meters)
pub const WGS84_A: f64 = 6378137.0;
/// WGS84 ellipsoid semi-minor axis (meters)
pub const WGS84_B: f64 = 6356752.314245;
/// WGS84 eccentricity squared
pub const WGS84_E2: f64 = 0.0066943799901413156; // 1.0 - (WGS84_B * WGS84_B) / (WGS84_A * WGS84_A)

/// Convert Latitude, Longitude, Altitude (WGS84) to Earth-Centered, Earth-Fixed (ECEF)
/// Lat and Lon are in degrees. Altitude is in meters.
pub fn lat_lon_alt_to_ecef(lat: f64, lon: f64, alt: f64) -> (f64, f64, f64) {
    let lat_rad = lat * PI / 180.0;
    let lon_rad = lon * PI / 180.0;

    let sin_lat = lat_rad.sin();
    let cos_lat = lat_rad.cos();
    let sin_lon = lon_rad.sin();
    let cos_lon = lon_rad.cos();

    let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();

    let x = (n + alt) * cos_lat * cos_lon;
    let y = (n + alt) * cos_lat * sin_lon;
    let z = (n * (1.0 - WGS84_E2) + alt) * sin_lat;

    (x, y, z)
}

/// Convert ECEF to East-North-Up (ENU) given a reference Lat/Lon/Alt
/// Output ENU coordinates are in meters, returning (East, North, Up).
pub fn ecef_to_enu(x: f64, y: f64, z: f64, ref_lat: f64, ref_lon: f64, ref_alt: f64) -> (f64, f64, f64) {
    let (ref_x, ref_y, ref_z) = lat_lon_alt_to_ecef(ref_lat, ref_lon, ref_alt);

    let dx = x - ref_x;
    let dy = y - ref_y;
    let dz = z - ref_z;

    let lat_rad = ref_lat * PI / 180.0;
    let lon_rad = ref_lon * PI / 180.0;

    let sin_lat = lat_rad.sin();
    let cos_lat = lat_rad.cos();
    let sin_lon = lon_rad.sin();
    let cos_lon = lon_rad.cos();

    let e = -sin_lon * dx + cos_lon * dy;
    let n = -sin_lat * cos_lon * dx - sin_lat * sin_lon * dy + cos_lat * dz;
    let u = cos_lat * cos_lon * dx + cos_lat * sin_lon * dy + sin_lat * dz;

    (e, n, u)
}

/// Helper function to convert Lat/Lon/Alt directly to ENU with a given reference point.
pub fn lat_lon_alt_to_enu(
    lat: f64,
    lon: f64,
    alt: f64,
    ref_lat: f64,
    ref_lon: f64,
    ref_alt: f64,
) -> (f64, f64, f64) {
    let (x, y, z) = lat_lon_alt_to_ecef(lat, lon, alt);
    ecef_to_enu(x, y, z, ref_lat, ref_lon, ref_alt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lat_lon_alt_to_ecef() {
        // Equatorial point
        let (x, y, z) = lat_lon_alt_to_ecef(0.0, 0.0, 0.0);
        assert!((x - WGS84_A).abs() < 1.0); // Should be exactly a along x-axis
        assert!((y - 0.0).abs() < 1.0);
        assert!((z - 0.0).abs() < 1.0);

        // North pole
        let (x, y, z) = lat_lon_alt_to_ecef(90.0, 0.0, 0.0);
        assert!((x - 0.0).abs() < 1.0);
        assert!((y - 0.0).abs() < 1.0);
        assert!((z - WGS84_B).abs() < 1.0); // Should be exactly b along z-axis
    }
}
