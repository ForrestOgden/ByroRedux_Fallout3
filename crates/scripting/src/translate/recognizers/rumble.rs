//! Recognizer: `defaultRumbleOnActivate` → [`RumbleOnActivate`].
//!
//! A **per-script** recognizer (the long tail): this script's behavior —
//! an `OnActivate` that shakes camera/controller, then a latent
//! `Utility.wait` that splits the handler across the
//! [`rumble_on_activate_system`](crate::papyrus_demo) / `rumble_tick_system`
//! pair — is bespoke enough not to fold into a generic family. But the
//! recognizer is not hardcoded: it extracts the five `Auto Property`
//! defaults from the parsed `.psc` AST (`Property.initial_value`), so a
//! mod that re-tunes `cameraIntensity` etc. is honored. This is the M47.2
//! "promote, don't duplicate" step — the same `RumbleOnActivate` the
//! M47.0 `register_spawners` path hard-built now flows from the AST
//! through the [`translate_script`](crate::translate::translate_script)
//! boundary.

use byroredux_papyrus::ast::{Expr, Script, ScriptItem};

use crate::papyrus_demo::{RumbleOnActivate, RumbleState};
use crate::translate::archetype::{RecognizeCtx, Recognized};
use crate::translate::source::ScriptSource;

/// The script header name this recognizer claims (case-insensitive).
const SCRIPT_NAME: &str = "defaultRumbleOnActivate";

pub fn recognize(ctx: &RecognizeCtx<'_>) -> Option<Recognized> {
    let ScriptSource::PapyrusSource(script) = ctx.source else {
        return None;
    };
    if !script.name.node.eq_ignore_case(SCRIPT_NAME) {
        return None;
    }

    // Extract the five Auto-property defaults from the AST; fall back to
    // the .psc-documented defaults for any the parser didn't surface. A
    // property that is *present but non-literal* declines the whole
    // recognizer (`?`) rather than coercing to the default — upholding the
    // chain-wide decline invariant (#1909). (Papyrus auto-property inits
    // are literal-only, so in practice `?` only ever fires on absence,
    // which `unwrap_or` handles.)
    //
    // M47.2 — the placed reference's VMAD wins over the script default, as
    // in the shipping runtime (a VMAD property value is the per-REFR
    // override of the `Auto Property` initializer). A VMAD value under the
    // wrong type tag declines, the `two_state_activator` rule.
    let d = RumbleOnActivate::default();
    let float = |name: &str, default: f32| -> Option<f32> {
        Some(match vmad_float(ctx, name)? {
            Some(value) => value,
            None => float_prop(script, name)?.unwrap_or(default),
        })
    };
    let rumble = RumbleOnActivate {
        camera_intensity: float("cameraIntensity", d.camera_intensity)?,
        duration: float("duration", d.duration)?,
        repeatable: match vmad_bool(ctx, "repeatable")? {
            Some(value) => value,
            None => bool_prop(script, "repeatable")?.unwrap_or(d.repeatable),
        },
        shake_left: float("shakeLeft", d.shake_left)?,
        shake_right: float("shakeRight", d.shake_right)?,
        // `Auto State active` — boots into Active, like the .psc.
        state: RumbleState::Active,
    };

    Some(Recognized::new(
        format!("rumble_on_activate@{}", script.name.node),
        move |world, entity| {
            // `RumbleOnActivate` is Copy — captured by value.
            if let Some(mut q) = world.query_mut::<RumbleOnActivate>() {
                q.insert(entity, rumble);
            }
        },
    ))
}

/// The VMAD-authored value of `name` on this script, with the same
/// three-case contract as [`float_prop`]: absent → `Some(None)`, a float (or
/// an int, which Papyrus widens) → `Some(Some(v))`, any other type → `None`.
fn vmad_float(ctx: &RecognizeCtx<'_>, name: &str) -> Option<Option<f32>> {
    use byroredux_plugin::esm::records::script_instance::PropertyValue;
    let Some(property) = ctx
        .script_instance
        .and_then(|instance| instance.script(SCRIPT_NAME))
        .and_then(|script| script.property(name))
    else {
        return Some(None);
    };
    match property.value {
        PropertyValue::Float(value) => Some(Some(value)),
        PropertyValue::Int32(value) => Some(Some(value as f32)),
        _ => None,
    }
}

/// Boolean sibling of [`vmad_float`].
fn vmad_bool(ctx: &RecognizeCtx<'_>, name: &str) -> Option<Option<bool>> {
    use byroredux_plugin::esm::records::script_instance::PropertyValue;
    let Some(property) = ctx
        .script_instance
        .and_then(|instance| instance.script(SCRIPT_NAME))
        .and_then(|script| script.property(name))
    else {
        return Some(None);
    };
    match property.value {
        PropertyValue::Bool(value) => Some(Some(value)),
        _ => None,
    }
}

/// The initial-value expression of the named `Property`, if present.
fn prop_init<'a>(script: &'a Script, name: &str) -> Option<&'a Expr> {
    script.body.iter().find_map(|item| match &item.node {
        ScriptItem::Property(p) if p.name.node.eq_ignore_case(name) => {
            p.initial_value.as_ref().map(|sp| &sp.node)
        }
        _ => None,
    })
}

/// Extract a float auto-property. The nested `Option` distinguishes the
/// three cases the caller must treat differently (#1909):
/// - `Some(Some(v))` — present with a literal value.
/// - `Some(None)` — absent (or no initializer) → caller uses its default.
/// - `None` — present but *non-literal* → caller declines.
fn float_prop(script: &Script, name: &str) -> Option<Option<f32>> {
    match prop_init(script, name) {
        None => Some(None),
        Some(Expr::FloatLit(f)) => Some(Some(*f as f32)),
        Some(Expr::IntLit(i)) => Some(Some(*i as f32)),
        Some(_) => None,
    }
}

/// Boolean sibling of [`float_prop`] with the same three-case contract.
fn bool_prop(script: &Script, name: &str) -> Option<Option<bool>> {
    match prop_init(script, name) {
        None => Some(None),
        Some(Expr::BoolLit(b)) => Some(Some(*b)),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translate::translate_script;
    use byroredux_core::ecs::world::World;
    use byroredux_papyrus::parse_script;
    use byroredux_plugin::esm::reader::GameKind;

    const SRC: &str = include_str!("../../../../../docs/r5/source/defaultRumbleOnActivate.psc");

    #[test]
    fn recognizes_rumble_and_extracts_psc_defaults() {
        let (script, errors) = parse_script(SRC).expect("rumble .psc parses");
        assert!(errors.is_empty(), "clean parse: {errors:?}");
        let source = ScriptSource::PapyrusSource(&script);

        let recognized = translate_script(&source, GameKind::Skyrim, None, None)
            .expect("rumble archetype recognized");
        assert_eq!(
            recognized.archetype,
            "rumble_on_activate@defaultRumbleOnActivate"
        );

        // Run the spawn against a real world and read back the component —
        // proves AST → canonical component end to end.
        let mut world = World::new();
        crate::register(&mut world);
        let entity = world.spawn();
        (recognized.spawn)(&mut world, entity);

        let q = world
            .query::<RumbleOnActivate>()
            .expect("RumbleOnActivate registered");
        let r = q.get(entity).expect("rumble component spawned");
        assert_eq!(r.camera_intensity, 0.25);
        assert_eq!(r.duration, 0.25);
        assert!(r.repeatable);
        assert_eq!(r.shake_left, 0.25);
        assert_eq!(r.shake_right, 0.25);
        assert_eq!(r.state, RumbleState::Active);
    }

    #[test]
    fn declines_a_different_script() {
        let (script, _) =
            parse_script("ScriptName SomethingElse extends ObjectReference\n").expect("parses");
        let source = ScriptSource::PapyrusSource(&script);
        assert!(translate_script(&source, GameKind::Skyrim, None, None).is_none());
    }

    #[test]
    fn declines_on_non_literal_property() {
        // A property present with a *non-literal* initializer (here a bare
        // identifier) declines rather than coercing to the .psc default —
        // the chain-wide "decline, don't coerce" invariant (#1909).
        let (script, errors) = parse_script(
            "ScriptName defaultRumbleOnActivate extends ObjectReference\n\
             Float Property cameraIntensity = someGlobalValue Auto\n",
        )
        .expect("parses");
        assert!(errors.is_empty(), "clean parse: {errors:?}");
        let source = ScriptSource::PapyrusSource(&script);
        assert!(
            translate_script(&source, GameKind::Skyrim, None, None).is_none(),
            "non-literal property value must decline, not coerce to default"
        );
    }

    /// M47.2 — a VMAD value under the wrong type tag declines rather than
    /// silently taking the script default.
    #[test]
    fn a_mistyped_vmad_property_declines() {
        use byroredux_plugin::esm::records::script_instance::{
            PropertyValue, ScriptInstance, ScriptInstanceData, ScriptProperty,
        };
        let (script, errors) = parse_script(SRC).expect("source parses");
        assert!(errors.is_empty());
        let vmad = |value| ScriptInstanceData {
            scripts: vec![ScriptInstance {
                name: SCRIPT_NAME.into(),
                status: 1,
                properties: vec![ScriptProperty {
                    name: "duration".into(),
                    status: 1,
                    value,
                }],
            }],
            ..Default::default()
        };
        let translate = |data: &ScriptInstanceData| {
            translate_script(
                &ScriptSource::PapyrusSource(&script),
                GameKind::Skyrim,
                Some(data),
                None,
            )
        };
        assert!(translate(&vmad(PropertyValue::Bool(true))).is_none());
        // An Int widens, as Papyrus does.
        let recognized = translate(&vmad(PropertyValue::Int32(2))).expect("recognized");
        let mut world = World::new();
        crate::register(&mut world);
        let entity = world.spawn();
        (recognized.spawn)(&mut world, entity);
        assert_eq!(world.get::<RumbleOnActivate>(entity).unwrap().duration, 2.0);
    }
}
