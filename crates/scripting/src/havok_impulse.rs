//! `ObjectReference.ApplyHavokImpulse(afX, afY, afZ, afMagnitude)` (M47.2).
//!
//! The CK page: "applies a Havok impulse force to this object of the specified
//! direction and magnitude"; it "will fail ... if the reference has no 3D or if a
//! zero-length force vector is used. Otherwise, the force vector will be
//! normalized before the force is applied." It gives no unit. The unit is the
//! game's own, read from `SkyrimSE.exe`'s native (disassembled from the
//! registration of the `"ApplyHavokImpulse"` string):
//!
//! 1. the direction is normalized and queued with the magnitude, unchanged;
//! 2. when the queued command runs, for each rigid body of the reference's
//!    collision it takes `magnitude × fGameplayImpulseScale × mult(layer)`, where
//!    `mult` is a game setting chosen by the body's Havok collision layer — layer
//!    8 and 32–33 `fGameplayImpulseMultBiped`, 10 `…Prop`, 14 `…Trap`, 20
//!    `…DebrisLarge`, every other layer `…Clutter` (the native calls the shared
//!    impulse-scale helper with its mass-normalisation flag clear, so the mass
//!    does not scale it);
//! 3. the direction times that value times `0.0142875` (the game-unit → Havok
//!    metre factor, a constant in the executable) is the linear impulse handed to
//!    `bhkRigidBody::ApplyLinearImpulse`, in Havok's kg·m/s.
//!
//! The engine's physics world is in Bethesda units, where one Havok metre is
//! `69.99125` units, so the impulse the engine applies is
//! `direction × scale × mult × magnitude × 0.0142875 × 69.99125` — equal to
//! `direction × scale × mult × magnitude` to within rounding. The scale (150) and
//! the multipliers (Trap 0.15, Biped 0.2, Prop 0.1, DebrisLarge 1.0, Clutter 1.0)
//! are the executable's defaults; `Skyrim.esm` authors the Biped, Clutter,
//! DebrisLarge and Prop multipliers itself (read from the master when present) and
//! leaves the scale and the Trap multiplier to the executable.
//!
//! A script's calls in one frame add: the impulse is linear, so the sum of
//! `direction × magnitude` is exact. The binary applies it in
//! `scripted_havok_impulse_system`.

use byroredux_core::ecs::resource::Resource;
use byroredux_core::ecs::sparse_set::SparseSetStorage;
use byroredux_core::ecs::storage::{Component, EntityId};
use byroredux_core::ecs::world::World;
use byroredux_plugin::esm::reader::GameKind;
use byroredux_plugin::esm::records::EsmIndex;

/// `fGameplayImpulseScale`, `…MultTrap`, `…MultBiped`, `…MultProp`,
/// `…MultDebrisLarge`, `…MultClutter` as `SkyrimSE.exe` builds them (the static
/// setting table; a master overrides any it authors).
const EXE_SCALE: f32 = 150.0;
const EXE_MULT_TRAP: f32 = 0.15;
const EXE_MULT_BIPED: f32 = 0.2;
const EXE_MULT_PROP: f32 = 0.1;
const EXE_MULT_DEBRIS_LARGE: f32 = 1.0;
const EXE_MULT_CLUTTER: f32 = 1.0;

/// The executable's game-unit → Havok-metre factor.
pub const GAME_UNITS_TO_HAVOK_METRES: f32 = 0.0142875;

/// The Havok layers the multiplier table names (Skyrim's `SkyrimLayer`).
const LAYER_BIPED: u8 = 8;
const LAYER_PROPS: u8 = 10;
const LAYER_TRAP: u8 = 14;
const LAYER_DEBRIS_LARGE: u8 = 20;
const LAYER_DEAD_BIPED: u8 = 32;
const LAYER_BIPED_NO_CC: u8 = 33;

/// The sum of `direction × magnitude` a script asked of a reference this frame,
/// in game coordinates (Z-up). One-shot: the binary applies and removes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HavokImpulseRequest {
    pub vector: [f32; 3],
}

impl Component for HavokImpulseRequest {
    type Storage = SparseSetStorage<Self>;
}

/// The impulse settings. `NOT_SAVED_BY_DESIGN`: derived from the load order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HavokImpulseRules {
    scale: f32,
    trap: f32,
    biped: f32,
    prop: f32,
    debris_large: f32,
    clutter: f32,
}

impl Resource for HavokImpulseRules {}

impl HavokImpulseRules {
    /// Skyrim's rules (the executable's defaults under the master's settings);
    /// `None` for the other games, whose natives were not read.
    pub fn from_index(index: &EsmIndex) -> Option<Self> {
        if index.game != GameKind::Skyrim {
            return None;
        }
        let setting = |name: &str, default: f32| index.game_setting_float(name).unwrap_or(default);
        Some(Self {
            scale: setting("fGameplayImpulseScale", EXE_SCALE),
            trap: setting("fGameplayImpulseMultTrap", EXE_MULT_TRAP),
            biped: setting("fGameplayImpulseMultBiped", EXE_MULT_BIPED),
            prop: setting("fGameplayImpulseMultProp", EXE_MULT_PROP),
            debris_large: setting("fGameplayImpulseMultDebrisLarge", EXE_MULT_DEBRIS_LARGE),
            clutter: setting("fGameplayImpulseMultClutter", EXE_MULT_CLUTTER),
        })
    }

    /// The layer's multiplier setting.
    pub fn layer_multiplier(&self, layer: u8) -> f32 {
        match layer {
            LAYER_BIPED | LAYER_DEAD_BIPED | LAYER_BIPED_NO_CC => self.biped,
            LAYER_PROPS => self.prop,
            LAYER_TRAP => self.trap,
            LAYER_DEBRIS_LARGE => self.debris_large,
            _ => self.clutter,
        }
    }

    /// The factor a request's `direction × magnitude` sum is multiplied by to
    /// give the impulse in Bethesda units for a body on `layer`:
    /// `scale × mult × 0.0142875 × havok_scale`.
    pub fn engine_factor(&self, layer: u8, havok_scale: f32) -> f32 {
        self.scale * self.layer_multiplier(layer) * GAME_UNITS_TO_HAVOK_METRES * havok_scale
    }
}

/// Queue `reference.ApplyHavokImpulse(x, y, z, magnitude)`: the direction is
/// normalized and added to the reference's pending request. `false` for a
/// zero-length or non-finite vector (the CK: the function fails).
pub fn request_impulse(world: &World, entity: EntityId, x: f32, y: f32, z: f32, magnitude: f32) -> bool {
    let length = (x * x + y * y + z * z).sqrt();
    if !(length.is_finite() && length > 0.0 && magnitude.is_finite()) {
        return false;
    }
    let add = [x / length * magnitude, y / length * magnitude, z / length * magnitude];
    let Some(mut requests) = world.query_mut::<HavokImpulseRequest>() else {
        return false;
    };
    match requests.get_mut(entity) {
        Some(pending) => {
            for (sum, delta) in pending.vector.iter_mut().zip(add) {
                *sum += delta;
            }
        }
        None => {
            requests.insert(entity, HavokImpulseRequest { vector: add });
        }
    }
    true
}

/// Install the rules from the load order.
pub fn install_havok_impulse_rules(world: &mut World, index: &EsmIndex) -> bool {
    match HavokImpulseRules::from_index(index) {
        Some(rules) => {
            world.insert_resource(rules);
            true
        }
        None => {
            world.remove_resource::<HavokImpulseRules>();
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> HavokImpulseRules {
        HavokImpulseRules::from_index(&EsmIndex { game: GameKind::Skyrim, ..Default::default() }).unwrap()
    }

    #[test]
    fn the_layer_picks_the_multiplier_the_executable_picks() {
        let rules = rules();
        assert_eq!(rules.layer_multiplier(14), 0.15, "Trap");
        assert_eq!(rules.layer_multiplier(8), 0.2, "Biped");
        assert_eq!(rules.layer_multiplier(32), 0.2, "dead biped");
        assert_eq!(rules.layer_multiplier(33), 0.2, "biped without character controller");
        assert_eq!(rules.layer_multiplier(10), 0.1, "Props");
        assert_eq!(rules.layer_multiplier(20), 1.0, "Debris Large");
        assert_eq!(rules.layer_multiplier(4), 1.0, "Clutter falls to the default");
        assert_eq!(rules.layer_multiplier(0), 1.0, "unidentified too");
    }

    /// 150 × mult × 0.0142875 × 69.99125 is 150 × mult to within rounding.
    #[test]
    fn the_unit_conversions_cancel_to_the_scale_and_multiplier() {
        let rules = rules();
        let factor = rules.engine_factor(14, 69.99125);
        assert!((factor - 150.0 * 0.15).abs() < 1.0e-3, "{factor}");
    }

    #[test]
    fn requests_normalize_sum_and_reject_a_zero_vector() {
        let mut world = World::new();
        world.register::<HavokImpulseRequest>();
        let entity = world.spawn();
        assert!(!request_impulse(&world, entity, 0.0, 0.0, 0.0, 5.0), "zero-length fails");
        assert!(request_impulse(&world, entity, 0.0, 0.0, -2.0, 15.0));
        assert!(request_impulse(&world, entity, 0.0, 0.0, -1.0, 50.0));
        let pending = *world.get::<HavokImpulseRequest>(entity).unwrap();
        assert_eq!(pending.vector, [0.0, 0.0, -65.0], "(0,0,-1)*15 + (0,0,-1)*50");
    }
}
