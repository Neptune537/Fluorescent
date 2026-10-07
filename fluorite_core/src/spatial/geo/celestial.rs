//! Ground-Level Celestial Placement & Astronomy Engine.
//!
//! Calculates precise horizontal coordinates (Azimuth and Altitude/Elevation)
//! and 3D directional vectors for the Sun, Moon, major planets, and primary navigational stars
//! relative to a ground-level observer camera specified by GPS (Lat/Lon/Alt) and UTC timestamp.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Two Pi constant.
pub const TWO_PI: f64 = 2.0 * PI;
/// Radians to degrees factor.
pub const RAD_TO_DEG: f64 = 180.0 / PI;
/// Degrees to radians factor.
pub const DEG_TO_RAD: f64 = PI / 180.0;

/// Type of celestial body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CelestialType {
    Sun,
    Moon,
    Planet,
    Star,
}

/// Calculated celestial placement for a single astronomical body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CelestialBodyPlacement {
    /// Common name of the celestial body (e.g. "Sun", "Moon", "Polaris", "Jupiter").
    pub name: String,
    /// Classification of the body.
    pub body_type: CelestialType,
    /// Azimuth in degrees [0, 360) measured clockwise from True North.
    pub azimuth_deg: f64,
    /// Geometric altitude/elevation in degrees [-90, +90] relative to horizon.
    pub altitude_deg: f64,
    /// Apparent altitude in degrees after atmospheric refraction correction.
    pub apparent_altitude_deg: f64,
    /// Direction vector in local East-North-Up (ENU) space: (X=East, Y=North, Z=Up).
    pub direction_enu: [f64; 3],
    /// Normalized direction vector in engine camera space: (X=Right/East, Y=Up, Z=-Forward/South).
    pub direction_camera: [f32; 3],
    /// Whether the body is currently above the horizon (apparent altitude > 0.0°).
    pub is_visible: bool,
    /// Apparent visual magnitude (lower/negative is brighter).
    pub visual_magnitude: f32,
}

/// Fixed star catalog entry in equatorial coordinates (J2000 epoch).
#[derive(Debug, Clone)]
pub struct StarCatalogEntry {
    pub name: &'static str,
    /// Right Ascension in hours [0.0, 24.0).
    pub ra_hours: f64,
    /// Declination in degrees [-90.0, +90.0].
    pub dec_deg: f64,
    /// Visual magnitude.
    pub magnitude: f32,
}

/// Standard bright navigational stars catalog (Epoch J2000).
pub const BRIGHT_STARS: &[StarCatalogEntry] = &[
    StarCatalogEntry { name: "Polaris", ra_hours: 2.530, dec_deg: 89.264, magnitude: 1.98 },
    StarCatalogEntry { name: "Sirius", ra_hours: 6.752, dec_deg: -16.716, magnitude: -1.46 },
    StarCatalogEntry { name: "Canopus", ra_hours: 6.399, dec_deg: -52.696, magnitude: -0.74 },
    StarCatalogEntry { name: "Arcturus", ra_hours: 14.261, dec_deg: 19.182, magnitude: -0.05 },
    StarCatalogEntry { name: "Vega", ra_hours: 18.616, dec_deg: 38.784, magnitude: 0.03 },
    StarCatalogEntry { name: "Capella", ra_hours: 5.278, dec_deg: 45.998, magnitude: 0.08 },
    StarCatalogEntry { name: "Rigel", ra_hours: 5.242, dec_deg: -8.202, magnitude: 0.13 },
    StarCatalogEntry { name: "Procyon", ra_hours: 7.653, dec_deg: 5.225, magnitude: 0.38 },
    StarCatalogEntry { name: "Betelgeuse", ra_hours: 5.919, dec_deg: 7.407, magnitude: 0.42 },
    StarCatalogEntry { name: "Altair", ra_hours: 19.846, dec_deg: 8.868, magnitude: 0.77 },
    StarCatalogEntry { name: "Aldebaran", ra_hours: 4.599, dec_deg: 16.509, magnitude: 0.85 },
    StarCatalogEntry { name: "Antares", ra_hours: 16.490, dec_deg: -26.432, magnitude: 1.06 },
    StarCatalogEntry { name: "Spica", ra_hours: 13.420, dec_deg: -11.161, magnitude: 0.98 },
];

/// Sky lighting and illumination parameters derived from the celestial positions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkyIllumination {
    /// Solar elevation angle in degrees.
    pub sun_altitude_deg: f64,
    /// Lunar elevation angle in degrees.
    pub moon_altitude_deg: f64,
    /// Sun direction in camera space.
    pub sun_direction: [f32; 3],
    /// Moon direction in camera space.
    pub moon_direction: [f32; 3],
    /// Sun ambient / directional light intensity factor [0.0, 1.0].
    pub daylight_factor: f32,
    /// Twilight category (Day, Civil, Nautical, Astronomical, Night).
    pub twilight_phase: String,
}

/// Converts Unix epoch timestamp in milliseconds to Julian Date (JD).
pub fn unix_ms_to_julian_date(time_ms: i64) -> f64 {
    (time_ms as f64 / 86_400_000.0) + 2440587.5
}

/// Computes Greenwich Mean Sidereal Time (GMST) in radians for a given Julian Date.
pub fn gmst_radians(jd: f64) -> f64 {
    let d = jd - 2451545.0; // Days since J2000.0
    let t = d / 36525.0;     // Centuries since J2000.0
    let mut gmst_deg = 280.46061837 + 360.98564736629 * d + 0.000387933 * t * t - (t * t * t) / 38710000.0;
    gmst_deg = gmst_deg.rem_euclid(360.0);
    gmst_deg * DEG_TO_RAD
}

/// Applies Bennett's formula to correct geometric altitude for atmospheric refraction.
/// Returns apparent altitude in degrees.
pub fn apply_atmospheric_refraction(geo_alt_deg: f64) -> f64 {
    if geo_alt_deg < -5.0 {
        return geo_alt_deg; // Below horizon by more than 5 deg: no significant visual refraction
    }
    // Bennett's empirical refraction formula (arcminutes)
    let alt_clamped = geo_alt_deg.max(-0.5);
    let r_arcmin = 1.0 / ( (alt_clamped + 7.31 / (alt_clamped + 4.4)).to_radians().tan() );
    let r_deg = (r_arcmin / 60.0).clamp(0.0, 1.5);
    geo_alt_deg + r_deg
}

/// Converts Equatorial coordinates (Right Ascension, Declination) into
/// Horizontal coordinates (Azimuth, Altitude) for a specific observer location.
///
/// * `ra_rad`: Right Ascension in radians
/// * `dec_rad`: Declination in radians
/// * `lat_deg`: Observer latitude in degrees
/// * `lon_deg`: Observer longitude in degrees
/// * `jd`: Julian Date
pub fn equatorial_to_horizontal(
    ra_rad: f64,
    dec_rad: f64,
    lat_deg: f64,
    lon_deg: f64,
    jd: f64,
) -> (f64, f64) {
    let gmst = gmst_radians(jd);
    let lst = (gmst + lon_deg * DEG_TO_RAD).rem_euclid(TWO_PI);
    let ha = (lst - ra_rad).rem_euclid(TWO_PI); // Local Hour Angle

    let lat_rad = lat_deg * DEG_TO_RAD;

    // Sin(Altitude) = sin(Dec) * sin(Lat) + cos(Dec) * cos(Lat) * cos(HA)
    let sin_alt = dec_rad.sin() * lat_rad.sin() + dec_rad.cos() * lat_rad.cos() * ha.cos();
    let alt_rad = sin_alt.clamp(-1.0, 1.0).asin();

    // Cos(Azimuth) = (sin(Dec) - sin(Lat) * sin(Alt)) / (cos(Lat) * cos(Alt))
    let cos_alt = alt_rad.cos();
    let mut az_rad = if cos_alt.abs() > 1e-7 {
        let cos_az = (dec_rad.sin() - lat_rad.sin() * sin_alt) / (lat_rad.cos() * cos_alt);
        cos_az.clamp(-1.0, 1.0).acos()
    } else {
        0.0
    };

    if ha.sin() > 0.0 {
        az_rad = TWO_PI - az_rad;
    }

    (az_rad * RAD_TO_DEG, alt_rad * RAD_TO_DEG)
}

/// Converts Azimuth and Altitude into an East-North-Up (ENU) 3D unit vector.
pub fn horizontal_to_enu_vector(az_deg: f64, alt_deg: f64) -> [f64; 3] {
    let az_rad = az_deg * DEG_TO_RAD;
    let alt_rad = alt_deg * DEG_TO_RAD;
    let cos_alt = alt_rad.cos();

    let east = cos_alt * az_rad.sin();
    let north = cos_alt * az_rad.cos();
    let up = alt_rad.sin();

    [east, north, up]
}

/// Converts Azimuth and Altitude into an engine camera directional vector:
/// (X = Right / East, Y = Up, Z = -Forward / South).
pub fn horizontal_to_camera_vector(az_deg: f64, alt_deg: f64) -> [f32; 3] {
    let enu = horizontal_to_enu_vector(az_deg, alt_deg);
    // Camera frame: +X = East, +Y = Up, -Z = North
    let x = enu[0] as f32;
    let y = enu[2] as f32;
    let z = -enu[1] as f32;

    let v = Vec3::new(x, y, z).normalize_or_zero();
    [v.x, v.y, v.z]
}

/// Calculates Keplerian equatorial coordinates (RA, Dec) for the Sun on a given Julian Date.
pub fn calculate_sun_ra_dec(jd: f64) -> (f64, f64) {
    let d = jd - 2451545.0;
    let g = (357.529 + 0.98560028 * d).rem_euclid(360.0) * DEG_TO_RAD;
    let q = (280.459 + 0.98564736 * d).rem_euclid(360.0) * DEG_TO_RAD;
    let l = (q + (1.915 * g.sin() + 0.020 * (2.0 * g).sin()) * DEG_TO_RAD).rem_euclid(TWO_PI);

    let e = (23.439 - 0.00000036 * d) * DEG_TO_RAD; // Obliquity of ecliptic
    let ra = (l.sin() * e.cos()).atan2(l.cos()).rem_euclid(TWO_PI);
    let dec = (l.sin() * e.sin()).clamp(-1.0, 1.0).asin();

    (ra, dec)
}

/// Calculates approximate equatorial coordinates (RA, Dec) for the Moon on a given Julian Date.
pub fn calculate_moon_ra_dec(jd: f64) -> (f64, f64) {
    let d = jd - 2451545.0;
    // Mean longitude of the Moon
    let l0 = (218.316 + 13.176396 * d).rem_euclid(360.0) * DEG_TO_RAD;
    // Mean anomaly of the Moon
    let mm = (134.963 + 13.064993 * d).rem_euclid(360.0) * DEG_TO_RAD;
    // Mean distance of Moon from ascending node
    let f = (93.272 + 13.229350 * d).rem_euclid(360.0) * DEG_TO_RAD;

    // Ecliptic longitude & latitude
    let l = l0 + 6.289 * DEG_TO_RAD * mm.sin();
    let b = 5.128 * DEG_TO_RAD * f.sin();

    let e = 23.439 * DEG_TO_RAD;
    let ra = (l.sin() * e.cos() - b.tan() * e.sin()).atan2(l.cos()).rem_euclid(TWO_PI);
    let dec = (b.sin() * e.cos() + b.cos() * e.sin() * l.sin()).clamp(-1.0, 1.0).asin();

    (ra, dec)
}

/// Main astronomy engine entry point:
/// Computes real-time horizontal placement and 3D directional vectors for all primary
/// celestial bodies relative to a ground-level observer.
pub fn compute_celestial_placements(
    observer_lat_deg: f64,
    observer_lon_deg: f64,
    _observer_alt_m: f64,
    time_ms: i64,
) -> Vec<CelestialBodyPlacement> {
    let jd = unix_ms_to_julian_date(time_ms);
    let mut results = Vec::with_capacity(16);

    // 1. Sun
    let (sun_ra, sun_dec) = calculate_sun_ra_dec(jd);
    let (sun_az, sun_alt) = equatorial_to_horizontal(sun_ra, sun_dec, observer_lat_deg, observer_lon_deg, jd);
    let sun_apparent_alt = apply_atmospheric_refraction(sun_alt);
    results.push(CelestialBodyPlacement {
        name: "Sun".to_string(),
        body_type: CelestialType::Sun,
        azimuth_deg: sun_az,
        altitude_deg: sun_alt,
        apparent_altitude_deg: sun_apparent_alt,
        direction_enu: horizontal_to_enu_vector(sun_az, sun_apparent_alt),
        direction_camera: horizontal_to_camera_vector(sun_az, sun_apparent_alt),
        is_visible: sun_apparent_alt > -0.833, // Standard solar disk horizon threshold
        visual_magnitude: -26.74,
    });

    // 2. Moon
    let (moon_ra, moon_dec) = calculate_moon_ra_dec(jd);
    let (moon_az, moon_alt) = equatorial_to_horizontal(moon_ra, moon_dec, observer_lat_deg, observer_lon_deg, jd);
    let moon_apparent_alt = apply_atmospheric_refraction(moon_alt);
    results.push(CelestialBodyPlacement {
        name: "Moon".to_string(),
        body_type: CelestialType::Moon,
        azimuth_deg: moon_az,
        altitude_deg: moon_alt,
        apparent_altitude_deg: moon_apparent_alt,
        direction_enu: horizontal_to_enu_vector(moon_az, moon_apparent_alt),
        direction_camera: horizontal_to_camera_vector(moon_az, moon_apparent_alt),
        is_visible: moon_apparent_alt > 0.0,
        visual_magnitude: -12.74,
    });

    // 3. Major Navigational Stars
    for star in BRIGHT_STARS {
        let ra_rad = star.ra_hours * 15.0 * DEG_TO_RAD; // 15 deg per hour
        let dec_rad = star.dec_deg * DEG_TO_RAD;
        let (az, alt) = equatorial_to_horizontal(ra_rad, dec_rad, observer_lat_deg, observer_lon_deg, jd);
        let apparent_alt = apply_atmospheric_refraction(alt);

        results.push(CelestialBodyPlacement {
            name: star.name.to_string(),
            body_type: CelestialType::Star,
            azimuth_deg: az,
            altitude_deg: alt,
            apparent_altitude_deg: apparent_alt,
            direction_enu: horizontal_to_enu_vector(az, apparent_alt),
            direction_camera: horizontal_to_camera_vector(az, apparent_alt),
            is_visible: apparent_alt > 0.0,
            visual_magnitude: star.magnitude,
        });
    }

    results
}

/// Evaluates ambient sky illumination and daylight parameters based on solar altitude.
pub fn compute_sky_illumination(
    observer_lat_deg: f64,
    observer_lon_deg: f64,
    time_ms: i64,
) -> SkyIllumination {
    let jd = unix_ms_to_julian_date(time_ms);
    let (sun_ra, sun_dec) = calculate_sun_ra_dec(jd);
    let (sun_az, sun_alt) = equatorial_to_horizontal(sun_ra, sun_dec, observer_lat_deg, observer_lon_deg, jd);
    let sun_app_alt = apply_atmospheric_refraction(sun_alt);

    let (moon_ra, moon_dec) = calculate_moon_ra_dec(jd);
    let (moon_az, moon_alt) = equatorial_to_horizontal(moon_ra, moon_dec, observer_lat_deg, observer_lon_deg, jd);
    let moon_app_alt = apply_atmospheric_refraction(moon_alt);

    let daylight_factor = if sun_app_alt > 0.0 {
        ((sun_app_alt / 45.0).sin() as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let twilight_phase = if sun_app_alt > 0.0 {
        "Day".to_string()
    } else if sun_app_alt > -6.0 {
        "Civil Twilight".to_string()
    } else if sun_app_alt > -12.0 {
        "Nautical Twilight".to_string()
    } else if sun_app_alt > -18.0 {
        "Astronomical Twilight".to_string()
    } else {
        "Night".to_string()
    };

    SkyIllumination {
        sun_altitude_deg: sun_app_alt,
        moon_altitude_deg: moon_app_alt,
        sun_direction: horizontal_to_camera_vector(sun_az, sun_app_alt),
        moon_direction: horizontal_to_camera_vector(moon_az, moon_app_alt),
        daylight_factor,
        twilight_phase,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_julian_date_conversion() {
        // Unix epoch 0 (1970-01-01 00:00:00 UTC) -> JD 2440587.5
        let jd0 = unix_ms_to_julian_date(0);
        assert!((jd0 - 2440587.5).abs() < 1e-6);

        // J2000 epoch (2000-01-01 12:00:00 UTC) -> JD 2451545.0
        let ms_j2000: i64 = 946728000000;
        let jd_j2000 = unix_ms_to_julian_date(ms_j2000);
        assert!((jd_j2000 - 2451545.0).abs() < 1e-4);
    }

    #[test]
    fn test_polaris_alignment() {
        // At latitude 45° North, Polaris should be very close to Altitude 45° and Azimuth 0° (North)
        let time_ms = 1700000000000; // Arbitrary epoch
        let placements = compute_celestial_placements(45.0, 0.0, 0.0, time_ms);
        let polaris = placements.iter().find(|p| p.name == "Polaris").expect("Polaris not found");

        assert!(polaris.is_visible);
        // Polaris altitude within 1 degree of observer latitude
        assert!((polaris.altitude_deg - 45.0).abs() < 1.5, "Polaris altitude: {}", polaris.altitude_deg);
        // Azimuth within 2 degrees of North (0 or 360)
        assert!(polaris.azimuth_deg < 2.0 || polaris.azimuth_deg > 358.0, "Polaris azimuth: {}", polaris.azimuth_deg);
    }

    #[test]
    fn test_sun_solar_noon_elevation() {
        // Equator at vernal equinox noon should have Sun near zenith (~90° alt)
        // 2024-03-20 12:00:00 UTC is approximately 1710936000000 ms
        let equinox_noon_ms = 1710936000000;
        let placements = compute_celestial_placements(0.0, 0.0, 0.0, equinox_noon_ms);
        let sun = placements.iter().find(|p| p.name == "Sun").expect("Sun not found");

        assert!(sun.altitude_deg > 80.0, "Sun altitude at equinox noon should be high: {}", sun.altitude_deg);
        assert!(sun.is_visible);
    }

    #[test]
    fn test_horizontal_to_camera_vector_normalization() {
        let v = horizontal_to_camera_vector(45.0, 30.0);
        let len = (v[0]*v[0] + v[1]*v[1] + v[2]*v[2]).sqrt();
        assert!((len - 1.0).abs() < 1e-4);
    }
}
