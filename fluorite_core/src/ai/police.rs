//! Complex Police Response Tree and Tactical Law-Enforcement Dispatch.
//!
//! Simulates multi-tiered wanted systems, dynamic search perimeters,
//! roadblocks, SWAT deployment, and helicopter air support tracking.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Wanted Level (1 to 5 Stars).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PoliceHeatLevel {
    None = 0,
    OneStar = 1,   // Minor infraction: 1-2 cruisers, non-lethal pursuit
    TwoStar = 2,   // Aggressive pursuit: cruiser ramming, PIT maneuvers
    ThreeStar = 3, // High-speed chase: spike strips, police roadblocks
    FourStar = 4,  // Tactical SWAT intervention: assault vehicles, sniper perimeters
    FiveStar = 5, // Maximum military/special forces escalation: air support helicopters, armored APCs
}

/// Unit types dispatched into the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DispatchUnitType {
    PatrolCruiser,
    UndercoverInterceptor,
    RoadblockTeam,
    SwatArmoredVan,
    HelicopterAirSupport,
}

/// Dynamic law-enforcement tactical dispatch tree.
pub struct PoliceResponseTree {
    pub heat_level: PoliceHeatLevel,
    pub last_known_player_pos: Vec3,
    pub search_perimeter_radius: f32,
    pub units_dispatched: Vec<DispatchUnitType>,
    pub cooldown_timer: f32,
}

impl PoliceResponseTree {
    pub fn new() -> Self {
        Self {
            heat_level: PoliceHeatLevel::None,
            last_known_player_pos: Vec3::ZERO,
            search_perimeter_radius: 0.0,
            units_dispatched: Vec::new(),
            cooldown_timer: 0.0,
        }
    }

    /// Sets or escalates the heat level.
    pub fn escalate_heat(&mut self, new_heat: PoliceHeatLevel, player_pos: Vec3) {
        self.heat_level = new_heat;
        self.last_known_player_pos = player_pos;
        self.cooldown_timer = 60.0; // 60s cooldown to evade

        self.units_dispatched.clear();
        match self.heat_level {
            PoliceHeatLevel::None => {
                self.search_perimeter_radius = 0.0;
            }
            PoliceHeatLevel::OneStar => {
                self.search_perimeter_radius = 150.0;
                self.units_dispatched.push(DispatchUnitType::PatrolCruiser);
            }
            PoliceHeatLevel::TwoStar => {
                self.search_perimeter_radius = 250.0;
                self.units_dispatched.push(DispatchUnitType::PatrolCruiser);
                self.units_dispatched
                    .push(DispatchUnitType::UndercoverInterceptor);
            }
            PoliceHeatLevel::ThreeStar => {
                self.search_perimeter_radius = 400.0;
                self.units_dispatched.push(DispatchUnitType::PatrolCruiser);
                self.units_dispatched
                    .push(DispatchUnitType::UndercoverInterceptor);
                self.units_dispatched.push(DispatchUnitType::RoadblockTeam);
            }
            PoliceHeatLevel::FourStar => {
                self.search_perimeter_radius = 600.0;
                self.units_dispatched.push(DispatchUnitType::SwatArmoredVan);
                self.units_dispatched.push(DispatchUnitType::RoadblockTeam);
                self.units_dispatched
                    .push(DispatchUnitType::HelicopterAirSupport);
            }
            PoliceHeatLevel::FiveStar => {
                self.search_perimeter_radius = 1000.0;
                self.units_dispatched.push(DispatchUnitType::SwatArmoredVan);
                self.units_dispatched.push(DispatchUnitType::RoadblockTeam);
                self.units_dispatched
                    .push(DispatchUnitType::HelicopterAirSupport);
                self.units_dispatched
                    .push(DispatchUnitType::HelicopterAirSupport);
            }
        }
    }

    /// Ticks cooldown when player is out of line of sight of all units.
    pub fn tick_evasion(&mut self, dt: f32, player_current_pos: Vec3) -> bool {
        if self.heat_level == PoliceHeatLevel::None {
            return true;
        }

        let dist_from_search = (player_current_pos - self.last_known_player_pos).length();

        if dist_from_search > self.search_perimeter_radius {
            self.cooldown_timer -= dt;
            if self.cooldown_timer <= 0.0 {
                self.heat_level = PoliceHeatLevel::None;
                self.units_dispatched.clear();
                return true; // Successfully evaded
            }
        } else {
            // Player still inside search radius
            self.cooldown_timer = (self.cooldown_timer + dt * 0.5).min(60.0);
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_police_escalation_and_evasion() {
        let mut police = PoliceResponseTree::new();
        police.escalate_heat(PoliceHeatLevel::ThreeStar, Vec3::new(100.0, 0.0, 100.0));

        assert_eq!(police.heat_level, PoliceHeatLevel::ThreeStar);
        assert!(police
            .units_dispatched
            .contains(&DispatchUnitType::RoadblockTeam));

        // Player flees outside 400m perimeter
        let evaded = police.tick_evasion(70.0, Vec3::new(1000.0, 0.0, 1000.0));
        assert!(evaded);
        assert_eq!(police.heat_level, PoliceHeatLevel::None);
    }
}
