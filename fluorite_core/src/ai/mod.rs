//! Living World AI Subsystem.
//!
//! Provides ultra-high-throughput Line-Of-Sight (LOS) batch checking, crowd density flow fields,
//! autonomous NPC daily routine schedules, and procedural law-enforcement dispatch response trees.

pub mod crowd;
pub mod los;
pub mod police;
pub mod routine;

pub use crowd::{CrowdAgent, CrowdDensityField};
pub use los::{LosBatchQuery, LosBatchResult, LosChecker};
pub use police::{DispatchUnitType, PoliceHeatLevel, PoliceResponseTree};
pub use routine::{ActivityType, DailySchedule, NpcRoutineState};
