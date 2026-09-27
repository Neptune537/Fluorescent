//! Autonomous NPC Daily Routines and Utility Schedules.
//!
//! Models 24-hour diurnal life simulation: sleep, morning commute, job tasks,
//! social conversations, dining, shopping, and vehicular driving behaviors.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// High-level daily activity category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityType {
    Sleeping,
    CommutingPedestrian,
    DrivingVehicle,
    WorkingJob,
    Dining,
    SocializingConversation,
    WanderingLeisure,
}

/// Time-slotted schedule entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub start_hour: f32, // 0.0 to 24.0
    pub end_hour: f32,
    pub activity: ActivityType,
    pub destination_name: String,
    pub destination_pos: Vec3,
}

/// Full daily schedule for an NPC archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySchedule {
    pub entries: Vec<ScheduleEntry>,
}

impl Default for DailySchedule {
    fn default() -> Self {
        Self {
            entries: vec![
                ScheduleEntry {
                    start_hour: 0.0,
                    end_hour: 7.0,
                    activity: ActivityType::Sleeping,
                    destination_name: "Apartment".into(),
                    destination_pos: Vec3::new(10.0, 0.0, 10.0),
                },
                ScheduleEntry {
                    start_hour: 7.0,
                    end_hour: 8.5,
                    activity: ActivityType::DrivingVehicle,
                    destination_name: "DowntownOffice".into(),
                    destination_pos: Vec3::new(500.0, 0.0, 300.0),
                },
                ScheduleEntry {
                    start_hour: 8.5,
                    end_hour: 17.5,
                    activity: ActivityType::WorkingJob,
                    destination_name: "WorkplaceDesk".into(),
                    destination_pos: Vec3::new(510.0, 15.0, 310.0),
                },
                ScheduleEntry {
                    start_hour: 17.5,
                    end_hour: 19.0,
                    activity: ActivityType::CommutingPedestrian,
                    destination_name: "TransitPlaza".into(),
                    destination_pos: Vec3::new(200.0, 0.0, 150.0),
                },
                ScheduleEntry {
                    start_hour: 19.0,
                    end_hour: 22.0,
                    activity: ActivityType::SocializingConversation,
                    destination_name: "BistroCafe".into(),
                    destination_pos: Vec3::new(80.0, 0.0, 60.0),
                },
                ScheduleEntry {
                    start_hour: 22.0,
                    end_hour: 24.0,
                    activity: ActivityType::Sleeping,
                    destination_name: "Apartment".into(),
                    destination_pos: Vec3::new(10.0, 0.0, 10.0),
                },
            ],
        }
    }
}

/// Runtime state of an NPC living world routine.
#[derive(Debug, Clone)]
pub struct NpcRoutineState {
    pub npc_id: u32,
    pub schedule: DailySchedule,
    pub current_activity: ActivityType,
    pub target_destination: Vec3,
    pub conversation_partner_id: Option<u32>,
}

impl NpcRoutineState {
    pub fn new(npc_id: u32, schedule: DailySchedule) -> Self {
        Self {
            npc_id,
            schedule,
            current_activity: ActivityType::Sleeping,
            target_destination: Vec3::ZERO,
            conversation_partner_id: None,
        }
    }

    /// Evaluates active activity based on the current world clock hour (0.0 .. 24.0).
    pub fn update_clock(&mut self, time_of_day_hours: f32) {
        let hour = time_of_day_hours.rem_euclid(24.0);

        for entry in &self.schedule.entries {
            if hour >= entry.start_hour && hour < entry.end_hour {
                self.current_activity = entry.activity;
                self.target_destination = entry.destination_pos;
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_daily_schedule() {
        let mut npc = NpcRoutineState::new(1, DailySchedule::default());

        npc.update_clock(3.0);
        assert_eq!(npc.current_activity, ActivityType::Sleeping);

        npc.update_clock(12.0);
        assert_eq!(npc.current_activity, ActivityType::WorkingJob);

        npc.update_clock(20.0);
        assert_eq!(npc.current_activity, ActivityType::SocializingConversation);
    }
}
