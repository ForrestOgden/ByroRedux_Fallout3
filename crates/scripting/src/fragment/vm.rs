//! The Papyrus VM (M47.2): runs a fragment the effect lowering declines.
//!
//! The lowering in [`crate::translate::effects`] turns a fragment into a flat
//! effect list ahead of time, which is exact but cannot express a value it
//! only learns at run time: an object local assigned from a call
//! (`ObjectReference x = Alias.GetRef()`), a quest-script function that
//! returns a value, a loop. This module interprets such a fragment over its
//! decompiled AST instead. It owns only control flow and values; every native
//! still goes through the effect runtime:
//!
//! * a native *statement* (`x.Enable()`) is re-written with its evaluated
//!   receiver and arguments bound to synthetic names — a form as a property
//!   of a synthetic VMAD, a quest or the player through
//!   [`NativeBindings`], numbers as literals — and classified by the same
//!   primitive table the lowering uses, then applied by `apply_effects`;
//! * a native *read* (`x.IsDisabled()`, `Q.GetStage()`) is classified the
//!   same way as a [`crate::translate::effects::ValueExpr`] or as guards and
//!   evaluated by the fragment runtime;
//! * the object reads no effect models — alias references, `GetPlayer`,
//!   `GetLinkedRef`, base objects, `GetOwningQuest` — are read here.
//!
//! A call into a quest's script (`kmyQuest.Startup()`, or one returning a
//! value) runs that function here too, against the quest's own VMAD.
//!
//! Script state follows the CK's *States (Papyrus)*: a call runs the version
//! in the instance's current state down the `extends` chain, then the empty
//! state's; an instance starts in its `Auto State`. The compiler's generated
//! `GotoState` / `GetState` keep the state in `::State`, stored per script
//! among its owner's script variables. Script variables — numbers in
//! [`crate::QuestScriptVariables`], objects and strings in
//! [`crate::QuestScriptObjects`] — are keyed by the object the script is
//! attached to; an unwritten one reads what the script declares.
//!
//! `Utility.Wait` suspends the run anywhere in a statement call chain; the
//! frames ride an `Effect::Interpret` resume on the continuation queue. The
//! per-frame statement budget ([`super::budget`], #5763) suspends a run the
//! same way, between two statements, with a zero delay; see there for what
//! cannot be split.
//!
//! [`check_function`] decides ahead of time, against typed placeholders,
//! whether every statement and call of a fragment (and of the script
//! functions it reaches, in every state they have) is one this VM runs, so
//! populate installs `Effect::Interpret` only for fragments that run whole.
//! A run that still meets something it cannot do — a native that declines
//! its actual values — stops there and logs it.
//!
//! A call or member access through a cast or a typed variable reaches the
//! script the VM runs on another reference, as that reference's.
//!
//! Not modelled: structs.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use byroredux_core::ecs::storage::EntityId;
use byroredux_core::ecs::world::World;
use byroredux_papyrus::ast::{
    AssignOp, BinaryOp, CallArg, Expr, FunctionFlags, Identifier, Param, Script, ScriptItem, StateItem,
    Stmt, Type, UnaryOp,
};
use byroredux_plugin::esm::records::condition::ConditionStringId;
use byroredux_papyrus::span::{Span, Spanned};
use byroredux_plugin::esm::records::script_instance::{
    PropertyValue, ScriptInstance, ScriptInstanceData, ScriptProperty,
};

use super::effects::{apply_effects, evaluate_guard, evaluate_value, DeferredFragmentEffects};
use crate::papyrus_provider::MAX_PROVIDER_WAIT_SECONDS;
use crate::quest_stages::{QuestFormId, QuestObjectiveState, QuestStageAdvanced, QuestStageState};
use crate::translate::compose::vmad_property_name;
use crate::translate::effects::{
    classify_native_effect, classify_native_guard, classify_native_value, NativeBindings, VmFrame,
    VmResume, VmStep, VmStored,
};

/// The player's reference, `PlayerRef`.
const PLAYER_REF: u32 = 0x14;
/// Statements one run may execute before it is stopped (a runaway loop).
const MAX_STEPS: usize = 100_000;
/// Script-function nesting.
const MAX_DEPTH: usize = 16;

/// A Papyrus value.
#[derive(Debug, Clone)]
pub enum Value {
    None,
    Bool(bool),
    Int(i32),
    Float(f32),
    Str(String),
    /// Any form by FormID — a reference, a base object, a quest; the
    /// player's reference is `0x14`.
    Form(u32),
    /// A quest alias.
    Alias { quest: u32, alias: i16 },
    Array(Rc<RefCell<Vec<Value>>>),
    /// A Fallout 4 struct: its members by name. Structs, like arrays, are
    /// shared by reference.
    Struct(Rc<RefCell<Vec<(String, Value)>>>),
}

impl Value {
    fn truthy(&self) -> bool {
        match self {
            Value::None => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::Form(form) => *form != 0,
            Value::Alias { .. } | Value::Array(_) | Value::Struct(_) => true,
        }
    }

    fn number(&self) -> Option<f32> {
        match self {
            Value::Bool(b) => Some(f32::from(u8::from(*b))),
            Value::Int(i) => Some(*i as f32),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    /// An `Int` (or `Bool`) exactly — `number()` goes through an `f32`,
    /// which cannot hold every 32-bit integer.
    fn int(&self) -> Option<i32> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Bool(b) => Some(i32::from(*b)),
            _ => None,
        }
    }

    fn text(&self) -> String {
        match self {
            Value::None => "None".into(),
            Value::Bool(b) => if *b { "True" } else { "False" }.into(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => format!("{f:.6}"),
            Value::Str(s) => s.clone(),
            Value::Form(form) => format!("[Form <{form:08X}>]"),
            Value::Alias { quest, alias } => format!("[Alias <{quest:08X}:{alias}>]"),
            Value::Array(_) => "[Array]".into(),
            Value::Struct(_) => "[Struct]".into(),
        }
    }

    fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::None, Value::None) => true,
            (Value::None, _) | (_, Value::None) => false,
            (Value::Str(a), Value::Str(b)) => a.eq_ignore_ascii_case(b),
            (Value::Form(a), Value::Form(b)) => a == b,
            (Value::Alias { quest: a, alias: x }, Value::Alias { quest: b, alias: y }) => a == b && x == y,
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Struct(a), Value::Struct(b)) => Rc::ptr_eq(a, b),
            (a, b) => match (a.int(), b.int()) {
                (Some(a), Some(b)) => a == b,
                _ => match (a.number(), b.number()) {
                    (Some(a), Some(b)) => a == b,
                    _ => false,
                },
            },
        }
    }

    /// A struct's member `name`, case-insensitively.
    fn member(members: &[(String, Value)], name: &str) -> Option<usize> {
        members.iter().position(|(member, _)| member.eq_ignore_ascii_case(name))
    }

    fn default_for(ty: &Type) -> Value {
        match ty {
            Type::Bool => Value::Bool(false),
            Type::Int => Value::Int(0),
            Type::Float => Value::Float(0.0),
            Type::String => Value::Str(String::new()),
            _ => Value::None,
        }
    }

    fn from_property(value: &PropertyValue) -> Value {
        match value {
            PropertyValue::Object { form_id: 0, alias: -1 } => Value::None,
            PropertyValue::Object { form_id, alias: -1 } => Value::Form(*form_id),
            PropertyValue::Object { form_id, alias } => Value::Alias {
                quest: *form_id,
                alias: *alias,
            },
            PropertyValue::String(s) => Value::Str(s.clone()),
            PropertyValue::Int32(i) => Value::Int(*i),
            PropertyValue::Float(f) => Value::Float(*f),
            PropertyValue::Bool(b) => Value::Bool(*b),
            PropertyValue::Array(items) => Value::Array(Rc::new(RefCell::new(
                items.iter().map(Value::from_property).collect(),
            ))),
            PropertyValue::Struct(members) => Value::Struct(Rc::new(RefCell::new(
                members
                    .iter()
                    .map(|member| (member.name.clone(), Value::from_property(&member.value)))
                    .collect(),
            ))),
            PropertyValue::Unknown(_) => Value::None,
        }
    }
}

/// Why a run stopped.
#[derive(Debug)]
struct Stop(String);

type Run<T> = Result<T, Stop>;

fn stop<T>(why: impl Into<String>) -> Run<T> {
    Err(Stop(why.into()))
}

/// One function activation.
struct Frame {
    script: Arc<Script>,
    /// The object the script is attached to (`Self`): a quest, or a
    /// reference for a reference script. Its script variables are keyed by
    /// it.
    owner: QuestFormId,
    vmad: Option<ScriptInstanceData>,
    locals: HashMap<String, Value>,
}

enum Flow {
    Normal,
    Return(Value),
    /// `Utility.Wait(seconds)`: the path down to it in this frame, and the
    /// suspended frames of the call it waits in (outermost first).
    Suspend(f32, Vec<VmStep>, Vec<VmFrame>),
}

fn suspend_at(step: VmStep, flow: Flow) -> Flow {
    match flow {
        Flow::Suspend(seconds, mut path, frames) => {
            path.insert(0, step);
            Flow::Suspend(seconds, path, frames)
        }
        other => other,
    }
}

/// `frame`'s state at `path` in `code` for a [`VmResume`].
fn frame_state(frame: &Frame, code: &Code, function: &str, path: Vec<VmStep>) -> Run<VmFrame> {
    let mut locals = Vec::with_capacity(frame.locals.len());
    for (name, value) in &frame.locals {
        let Some(value) = store(value) else {
            return stop("an array local across a wait");
        };
        locals.push((name.clone(), value));
    }
    Ok(VmFrame {
        script: frame.script.name.node.0.clone(),
        function: function.to_owned(),
        owner: frame.owner.0,
        defined_in: code.script.name.node.0.clone(),
        state: code.state.clone(),
        path,
        locals,
    })
}

/// Every local of `frame` can be stored in a [`VmFrame`] (no array or struct).
fn locals_storable(frame: &Frame) -> bool {
    frame
        .locals
        .values()
        .all(|value| !matches!(value, Value::Array(_) | Value::Struct(_)))
}

fn restore_locals(frame: &VmFrame) -> HashMap<String, Value> {
    frame
        .locals
        .iter()
        .map(|(name, value)| (name.clone(), restore(value)))
        .collect()
}

fn store(value: &Value) -> Option<VmStored> {
    Some(match value {
        Value::None => VmStored::None,
        Value::Bool(b) => VmStored::Bool(*b),
        Value::Int(i) => VmStored::Int(*i),
        Value::Float(f) => VmStored::Float(*f),
        Value::Str(s) => VmStored::Str(s.clone()),
        Value::Form(form) => VmStored::Form(*form),
        Value::Alias { quest, alias } => VmStored::Alias {
            quest: *quest,
            alias: *alias,
        },
        Value::Array(_) | Value::Struct(_) => return None,
    })
}

fn restore(stored: &VmStored) -> Value {
    match stored {
        VmStored::None => Value::None,
        VmStored::Bool(b) => Value::Bool(*b),
        VmStored::Int(i) => Value::Int(*i),
        VmStored::Float(f) => Value::Float(*f),
        VmStored::Str(s) => Value::Str(s.clone()),
        VmStored::Form(form) => Value::Form(*form),
        VmStored::Alias { quest, alias } => Value::Alias {
            quest: *quest,
            alias: *alias,
        },
    }
}

/// The run's mutable context: the fragment runtime's own state.
struct Vm<'a> {
    world: &'a World,
    context: QuestFormId,
    stages: &'a mut QuestStageState,
    objectives: &'a mut QuestObjectiveState,
    deferred: &'a mut DeferredFragmentEffects,
    steps: usize,
    depth: usize,
    advances: Vec<QuestStageAdvanced>,
    /// The frame budget this run may spend (statements), the statements it
    /// has spent of it, and whether it suspended because that ran out.
    allowance: usize,
    spent: usize,
    yielded: bool,
    /// Calls for a value in progress, and calling frames holding an array or
    /// struct local: a run cannot suspend while either is on the stack.
    value_calls: usize,
    blocked_callers: usize,
}

/// Run `script`'s `function` attached to `context` — a quest's fragment, or
/// a reference script's event with its `args` — its properties bound by
/// `vmad`, from the start or from where `resume` says a `Utility.Wait`
/// suspended it. Returns the quest-stage advances its natives produced, and
/// the wait and resume point when it suspends again.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_fragment(
    script: &str,
    function: &str,
    args: &[VmStored],
    resume: Option<&VmResume>,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    stages: &mut QuestStageState,
    objectives: &mut QuestObjectiveState,
    deferred: &mut DeferredFragmentEffects,
) -> (Vec<QuestStageAdvanced>, Option<(f32, VmResume)>) {
    let Some(script) = deferred.vm_scripts.get(&script.to_ascii_lowercase()).cloned() else {
        log::warn!("Papyrus VM: script '{script}' is not installed");
        return (Vec::new(), None);
    };
    let mut vm = Vm {
        world,
        context,
        stages,
        objectives,
        deferred,
        // A run the frame budget suspended counts its earlier statements
        // against the cap; one a wait suspended starts over.
        steps: resume.map_or(0, |state| state.carried_steps as usize),
        depth: 0,
        advances: Vec::new(),
        allowance: world
            .try_resource::<super::VmFrameBudget>()
            .map_or(usize::MAX, |budget| budget.remaining()),
        spent: super::RUN_START_COST,
        yielded: false,
        value_calls: 0,
        blocked_callers: 0,
    };
    let frame = Frame {
        script: Arc::clone(&script),
        owner: context,
        vmad: vmad.cloned(),
        locals: HashMap::new(),
    };
    let run = match resume.and_then(|state| state.frames.split_first()) {
        Some((top, inner)) => vm.resume_frame(frame, top, inner),
        None => match vm.resolve(&script, context, function) {
            // An event the script does not handle in its current state:
            // nothing runs.
            None if super::reference::is_delivered_event(function) => Ok(Flow::Normal),
            None => stop(format!("no function {function}")),
            Some(code) => {
                let mut args: Vec<Value> = args.iter().map(restore).collect();
                // A topic-info fragment's speaker parameter is the line's
                // speaker, as the lowering binds it.
                let speaker_parameter = code.callable(function).is_ok_and(|callable| {
                    callable.params.first().is_some_and(|param| {
                        param
                            .name
                            .node
                            .0
                            .eq_ignore_ascii_case(crate::translate::effects::INFO_SPEAKER_PARAMETER)
                    })
                });
                if args.is_empty() && speaker_parameter {
                    let speaker = vm.deferred.speaker.and_then(|entity| vm.entity_form(entity));
                    args.push(speaker.map_or(Value::None, Value::Form));
                }
                vm.enter(frame, &code, function, args).and_then(|mut frame| {
                    let flow = vm.run_frame(&mut frame, &code, function, None, &[])?;
                    vm.callee_flow(flow, &frame, &code, function)
                })
            }
        },
    }
    .map(|flow| match flow {
        Flow::Suspend(seconds, _, frames) => {
            let carried_steps = if vm.yielded { vm.steps as u32 } else { 0 };
            Some((seconds, VmResume { frames, carried_steps }))
        }
        _ => None,
    });
    let suspended = match run {
        Ok(suspended) => suspended,
        Err(Stop(why)) => {
            log::warn!(
                "Papyrus VM: {}.{function} on {:08X} stopped: {why}",
                script.name.node.0,
                context.0
            );
            None
        }
    };
    // Settle with the frame's budget: what this run spent, and whether it was
    // cut short by it.
    if let Some(mut budget) = world.try_resource_mut::<super::VmFrameBudget>() {
        budget.charge(vm.spent);
        if vm.yielded {
            budget.note_yielded();
        }
    }
    (vm.advances, suspended)
}

/// What a call runs: a function's or an event's parameters and body.
struct Callable<'s> {
    params: &'s [Param],
    body: &'s [Spanned<Stmt>],
    /// A function's declared return type; an event returns nothing.
    returns: Option<&'s Spanned<Type>>,
}

/// `name` defined in `script` — in the state block `state`, or in the empty
/// state for `None` — as a function or an event. A `Native` function is not
/// one: its declaration (in `Quest`, `ObjectReference`, … which the script
/// library also serves) has no body, and the native bridge runs it.
fn callable_in<'s>(script: &'s Script, state: Option<&str>, name: &str) -> Option<Callable<'s>> {
    let named = |n: &Identifier| n.0.eq_ignore_ascii_case(name);
    let scripted = |f: &byroredux_papyrus::ast::Function| !f.flags.contains(FunctionFlags::NATIVE);
    match state {
        None => script.body.iter().find_map(|item| match &item.node {
            ScriptItem::Function(f) if named(&f.name.node) && scripted(f) => Some(Callable {
                params: &f.params,
                body: &f.body,
                returns: f.return_type.as_ref(),
            }),
            ScriptItem::Event(e) if named(&e.name.node) => Some(Callable {
                params: &e.params,
                body: &e.body,
                returns: None,
            }),
            _ => None,
        }),
        Some(state) => script.body.iter().find_map(|item| match &item.node {
            ScriptItem::State(block) if block.name.node.0.eq_ignore_ascii_case(state) => {
                block.body.iter().find_map(|item| match &item.node {
                    StateItem::Function(f) if named(&f.name.node) && scripted(f) => Some(Callable {
                        params: &f.params,
                        body: &f.body,
                        returns: f.return_type.as_ref(),
                    }),
                    StateItem::Event(e) if named(&e.name.node) => Some(Callable {
                        params: &e.params,
                        body: &e.body,
                        returns: None,
                    }),
                    _ => None,
                })
            }
            _ => None,
        }),
    }
}

/// Where a running body is: the script that defines it and its state block
/// (`None`: the empty state).
#[derive(Clone)]
struct Code {
    script: Arc<Script>,
    state: Option<String>,
}

impl Code {
    fn callable(&self, name: &str) -> Run<Callable<'_>> {
        match callable_in(&self.script, self.state.as_deref(), name) {
            Some(callable) => Ok(callable),
            None => stop(format!("no function {name}")),
        }
    }
}

/// `script` and the scripts it extends that `scripts` carries, most derived
/// first.
fn script_chain(script: &Arc<Script>, scripts: &HashMap<String, Arc<Script>>) -> Vec<Arc<Script>> {
    let mut chain = vec![Arc::clone(script)];
    while chain.len() < 16 {
        let Some(parent) = chain.last().and_then(|last| last.parent.as_ref()) else {
            break;
        };
        match scripts.get(&parent.node.0.to_ascii_lowercase()) {
            Some(parent) => chain.push(Arc::clone(parent)),
            None => break,
        }
    }
    chain
}

/// An engine class — `Form`, `ObjectReference`, `Quest`, … — declares
/// `Native` functions; no game script does. Its API, scripted wrappers
/// included (`Quest.SetStage` → `SetCurrentStageID`), is the native
/// bridge's, so calls never resolve into it.
fn is_engine_class(script: &Script) -> bool {
    let native = |f: &byroredux_papyrus::ast::Function| f.flags.contains(FunctionFlags::NATIVE);
    script.body.iter().any(|item| match &item.node {
        ScriptItem::Function(f) => native(f),
        ScriptItem::State(block) => block.body.iter().any(|item| matches!(&item.node, StateItem::Function(f) if native(f))),
        _ => false,
    })
}

/// The body a call of `name` runs on an instance of `script` in `state`.
/// CK *States (Papyrus)*: the current state down the `extends` chain, then
/// the empty state down the chain — stopping at the engine classes.
fn resolve_in(script: &Arc<Script>, state: &str, name: &str, scripts: &HashMap<String, Arc<Script>>) -> Option<Code> {
    let chain: Vec<_> = script_chain(script, scripts)
        .into_iter()
        .take_while(|script| !is_engine_class(script))
        .collect();
    let in_state = (!state.is_empty())
        .then(|| {
            chain.iter().find(|s| callable_in(s, Some(state), name).is_some()).map(|s| Code {
                script: Arc::clone(s),
                state: Some(state.to_owned()),
            })
        })
        .flatten();
    in_state.or_else(|| {
        chain.iter().find(|s| callable_in(s, None, name).is_some()).map(|s| Code {
            script: Arc::clone(s),
            state: None,
        })
    })
}

/// The state an instance of `script` starts in: its `Auto State`, or one a
/// script it extends declares; else the empty state.
fn auto_state(script: &Arc<Script>, scripts: &HashMap<String, Arc<Script>>) -> String {
    script_chain(script, scripts)
        .iter()
        .find_map(|s| {
            s.body.iter().find_map(|item| match &item.node {
                ScriptItem::State(block) if block.is_auto => Some(block.name.node.0.clone()),
                _ => None,
            })
        })
        .unwrap_or_default()
}

/// The names of `script`'s state blocks.
fn state_names(script: &Script) -> impl Iterator<Item = &str> {
    script.body.iter().filter_map(|item| match &item.node {
        ScriptItem::State(block) => Some(block.name.node.0.as_str()),
        _ => None,
    })
}

/// The decompiled `GotoState` / `GetState` keep a script's state in the
/// `::State` variable.
fn is_state_variable(name: &str) -> bool {
    name.eq_ignore_ascii_case("::state")
}

/// Where `script`'s state is kept among its owner's script variables — per
/// script, since one reference may carry several.
fn state_key(script: &Script) -> ConditionStringId {
    ConditionStringId::from_text(&format!("::state::{}", script.name.node.0.to_ascii_lowercase()))
}

/// The value a script variable or property declares (`Int n = 5`,
/// `Bool Property P = True Auto`), or an `AutoReadOnly` property's constant,
/// when it is a literal.
fn declared_default(script: &Script, name: &str) -> Option<Value> {
    let bare = vmad_property_name(name);
    let named = |n: &Identifier| n.0.eq_ignore_ascii_case(name) || n.0.eq_ignore_ascii_case(bare);
    script.body.iter().find_map(|item| {
        let initial = match &item.node {
            ScriptItem::Variable(v) if named(&v.name.node) => v.initial_value.as_ref(),
            ScriptItem::Property(p) if named(&p.name.node) => match &p.initial_value {
                Some(initial) => Some(initial),
                None => return getter_constant(p),
            },
            _ => return None,
        };
        literal_value(&initial?.node)
    })
}

fn literal_value(e: &Expr) -> Option<Value> {
    Some(match literal_of(e)? {
        Expr::IntLit(i) => Value::Int(i as i32),
        Expr::FloatLit(f) => Value::Float(f as f32),
        Expr::BoolLit(b) => Value::Bool(b),
        Expr::StringLit(text) => Value::Str(text),
        _ => Value::None,
    })
}

/// An `AutoReadOnly` property's constant: the compiler gives it a getter
/// that returns a literal (`ObjectReference.Motion_Keyframed` → 2).
fn getter_constant(property: &byroredux_papyrus::ast::Property) -> Option<Value> {
    let getter = property.getter.as_ref()?;
    match getter.body.as_slice() {
        [statement] => match &statement.node {
            Stmt::Return(Some(value)) => literal_value(&value.node),
            _ => None,
        },
        _ => None,
    }
}

/// The `Math` natives the VM computes itself.
const MATH_NATIVES: [&str; 8] = ["min", "max", "abs", "floor", "ceiling", "sqrt", "pow", "sin"];

/// `Math.<name>(values)`, by lowercase name (CK *Math Script* pages; `Min` /
/// `Max` are Fallout 4's). `None` arity or kind the VM cannot compute.
fn math_native(name: &str, values: &[Value]) -> Run<Option<Value>> {
    let mut numbers = Vec::with_capacity(values.len());
    for value in values {
        let Some(number) = value.number() else {
            return stop(format!("Math.{name} on {}", value.text()));
        };
        numbers.push(number);
    }
    let arity = if matches!(name, "min" | "max" | "pow") { 2 } else { 1 };
    if numbers.len() != arity {
        return stop(format!("Math.{name} takes {arity} argument(s)"));
    }
    let a = numbers[0];
    Ok(Some(match name {
        "min" => Value::Float(a.min(numbers[1])),
        "max" => Value::Float(a.max(numbers[1])),
        "abs" => Value::Float(a.abs()),
        "floor" => Value::Int(a.floor() as i32),
        "ceiling" => Value::Int(a.ceil() as i32),
        "sqrt" => Value::Float(a.sqrt()),
        "pow" => Value::Float(a.powf(numbers[1])),
        // Papyrus trigonometry takes degrees.
        "sin" => Value::Float(a.to_radians().sin()),
        _ => return Ok(None),
    }))
}

/// The array methods the VM runs (CK *Arrays (Papyrus)*).
const ARRAY_METHODS: [&str; 9] = [
    "find",
    "rfind",
    "findstruct",
    "rfindstruct",
    "add",
    "insert",
    "remove",
    "removelast",
    "clear",
];

/// The declaration of the struct type `name`: `Script#Struct` names its
/// script; a bare name is a struct of `script`'s chain.
fn struct_declaration(
    script: &Arc<Script>,
    name: &str,
    scripts: &HashMap<String, Arc<Script>>,
) -> Option<byroredux_papyrus::ast::Struct> {
    let (owners, struct_name) = match name.split_once('#') {
        Some((owner, struct_name)) => (vec![scripts.get(&owner.to_ascii_lowercase())?.clone()], struct_name),
        None => (script_chain(script, scripts), name),
    };
    owners.iter().find_map(|owner| {
        owner.body.iter().find_map(|item| match &item.node {
            ScriptItem::Struct(declaration) if declaration.name.node.0.eq_ignore_ascii_case(struct_name) => {
                Some(declaration.clone())
            }
            _ => None,
        })
    })
}

/// The engine classes whose `AutoReadOnly` constants a script reads on any
/// reference (`akRef.Motion_Keyframed`).
const CONSTANT_CLASSES: [&str; 3] = ["objectreference", "actor", "form"];

/// A constant property's value: an `AutoReadOnly` one's literal, as source
/// declares it or as the compiled getter returns it.
fn property_constant(property: &byroredux_papyrus::ast::Property) -> Option<Value> {
    let read_only = property
        .flags
        .contains(byroredux_papyrus::ast::PropertyFlags::AUTO_READ_ONLY);
    match &property.initial_value {
        Some(initial) if read_only => literal_value(&initial.node),
        _ => getter_constant(property),
    }
}

/// `name` as a constant property of `script` (not a parent).
fn constant_property<'s>(script: &'s Script, name: &str) -> Option<&'s byroredux_papyrus::ast::Property> {
    script.body.iter().find_map(|item| match &item.node {
        ScriptItem::Property(p) if p.name.node.0.eq_ignore_ascii_case(name) && property_constant(p).is_some() => Some(&**p),
        _ => None,
    })
}

/// The declared type of `name` in `script` or a parent script `scripts`
/// carries.
fn member_type_in(script: &Script, name: &str, scripts: &HashMap<String, Arc<Script>>) -> Option<Type> {
    let mut current: Option<&Script> = Some(script);
    for _ in 0..16 {
        let here = current?;
        if let Some(ty) = script_member_type(here, name) {
            return Some(ty);
        }
        current = here
            .parent
            .as_ref()
            .and_then(|parent| scripts.get(&parent.node.0.to_ascii_lowercase()))
            .map(|parent| &**parent);
    }
    None
}

/// The script's declared type for `name` (a property or a script variable).
fn script_member_type(script: &Script, name: &str) -> Option<Type> {
    let bare = vmad_property_name(name);
    script.body.iter().find_map(|item| match &item.node {
        ScriptItem::Property(p)
            if p.name.node.0.eq_ignore_ascii_case(bare) || p.name.node.0.eq_ignore_ascii_case(name) =>
        {
            Some(p.ty.node.clone())
        }
        ScriptItem::Variable(v)
            if v.name.node.0.eq_ignore_ascii_case(name) || v.name.node.0.eq_ignore_ascii_case(bare) =>
        {
            Some(v.ty.node.clone())
        }
        _ => None,
    })
}

/// `Game` / `Utility` / `Debug` / `Math` — the global-function classes.
fn static_class(e: &Expr) -> Option<&str> {
    let Expr::Ident(name) = e else {
        return None;
    };
    ["game", "utility", "debug", "math"]
        .into_iter()
        .find(|class| name.0.eq_ignore_ascii_case(class))
}

fn spanned<T>(node: T) -> Spanned<T> {
    Spanned::new(node, Span::new(0, 0))
}

fn ident(name: &str) -> Spanned<Expr> {
    spanned(Expr::Ident(Identifier(name.to_owned())))
}

/// Builds one synthetic native call: evaluated values bound to names.
#[derive(Default)]
struct Synth {
    bindings: NativeBindings,
    properties: Vec<ScriptProperty>,
}

impl Synth {
    fn bind(&mut self, vm: &Vm<'_>, value: &Value) -> Run<Spanned<Expr>> {
        let name = format!("__vm{}", self.properties.len() + self.bindings.players.len() + 1);
        let object = |form_id: u32, alias: i16| PropertyValue::Object { form_id, alias };
        Ok(match value {
            Value::None => spanned(Expr::NoneLit),
            Value::Bool(b) => spanned(Expr::BoolLit(*b)),
            Value::Int(i) => spanned(Expr::IntLit(i64::from(*i))),
            // An integral number binds as an Int literal: the primitives read
            // Int parameters with `int_arg`, Float ones with `as_num`, which
            // takes either.
            Value::Float(f) if f.fract() == 0.0 && f.abs() < 2_147_483_648.0 => {
                spanned(Expr::IntLit(*f as i64))
            }
            Value::Float(f) => spanned(Expr::FloatLit(f64::from(*f))),
            Value::Str(s) => spanned(Expr::StringLit(s.clone())),
            Value::Form(PLAYER_REF) => {
                self.bindings.players.insert(name.clone());
                ident(&name)
            }
            Value::Form(form) => {
                if vm.is_quest(*form) {
                    self.bindings.quests.insert(name.clone());
                }
                self.properties.push(ScriptProperty {
                    name: name.clone(),
                    status: 1,
                    value: object(*form, -1),
                });
                ident(&name)
            }
            // The effect runtime resolves an alias property against the
            // fragment's own quest.
            Value::Alias { quest, alias } if *quest == vm.context.0 => {
                self.properties.push(ScriptProperty {
                    name: name.clone(),
                    status: 1,
                    value: object(*quest, *alias),
                });
                ident(&name)
            }
            Value::Alias { .. } => return stop("another quest's alias as a native argument"),
            Value::Array(_) => return stop("an array as a native argument"),
            Value::Struct(_) => return stop("a struct as a native argument"),
        })
    }

    fn vmad(self) -> (NativeBindings, ScriptInstanceData) {
        (
            self.bindings,
            ScriptInstanceData {
                version: 5,
                object_format: 2,
                scripts: vec![ScriptInstance {
                    name: "<Papyrus VM>".into(),
                    status: 0,
                    properties: self.properties,
                }],
            },
        )
    }
}

impl Vm<'_> {
    fn is_quest_form(&self, value: &Value) -> bool {
        matches!(value, Value::Form(form) if self.is_quest(*form))
    }

    fn is_quest(&self, form: u32) -> bool {
        let quest = QuestFormId(form);
        self.deferred.quest_scripts.contains_key(&quest) || self.stages.is_started(quest)
    }

    fn tick(&mut self) -> Run<()> {
        self.steps += 1;
        self.spent += 1;
        if self.steps > MAX_STEPS {
            return stop("step limit");
        }
        Ok(())
    }

    /// The run has used its frame allowance and may stop here, between two
    /// statements: nothing on the stack needs a value back or holds a local
    /// the saved continuation cannot store.
    fn should_yield(&self, frame: &Frame) -> bool {
        self.spent >= self.allowance
            && self.value_calls == 0
            && self.blocked_callers == 0
            && locals_storable(frame)
    }

    /// The body a call of `name` on `owner`'s instance of `script` runs, in
    /// the instance's current state.
    fn resolve(&self, script: &Arc<Script>, owner: QuestFormId, name: &str) -> Option<Code> {
        let state = self.script_state(owner, script);
        resolve_in(script, &state, name, &self.deferred.vm_scripts)
    }

    /// `<quest>.RegisterForUpdate(…)` and the like on another quest: every
    /// script attached to it that handles the update registers (or all of
    /// them unregister).
    fn quest_update(&mut self, frame: &mut Frame, quest: u32, name: &str, args: &[CallArg]) -> Run<()> {
        let mut numbers = Vec::with_capacity(args.len());
        for arg in args {
            numbers.push(self.eval(frame, &arg.value.node)?.number());
        }
        let Some(request) = super::UpdateRequest::of_call(name, &numbers) else {
            return stop(format!("{name} without its numbers"));
        };
        let handler = super::UpdateRequest::handler(name).unwrap_or_default();
        let owner = QuestFormId(quest);
        let attached: Vec<String> = self
            .deferred
            .quest_scripts
            .get(&owner)
            .map(|vmad| vmad.scripts.iter().map(|script| script.name.clone()).collect())
            .unwrap_or_default();
        for name in attached {
            let script = self.deferred.vm_scripts.get(&name.to_ascii_lowercase()).cloned();
            let handles = script.is_some_and(|script| handler.is_empty() || self.resolve(&script, owner, handler).is_some());
            if handles {
                self.deferred.script_updates.push((quest, name, request));
            }
        }
        Ok(())
    }

    /// `GotoState(state)` for a script without the compiler's generated
    /// one: `OnEndState` in the old state, the switch, `OnBeginState` in the
    /// new one (CK *OnBeginState*: also when re-entering the same state).
    fn goto_state(&mut self, frame: &Frame, state: String) -> Run<()> {
        for (event, switch) in [("OnEndState", false), ("OnBeginState", true)] {
            if switch {
                let key = state_key(&frame.script);
                self.write_object(frame.owner, key, &Value::Str(state.clone()))?;
            }
            if let Some(code) = self.resolve(&frame.script, frame.owner, event) {
                let handler = Frame {
                    script: Arc::clone(&frame.script),
                    owner: frame.owner,
                    vmad: frame.vmad.clone(),
                    locals: HashMap::new(),
                };
                self.call_function(handler, &code, event, Vec::new())?;
            }
        }
        Ok(())
    }

    /// `owner`'s instance of `script`'s state: the one `GotoState` last set,
    /// else the auto state.
    fn script_state(&self, owner: QuestFormId, script: &Arc<Script>) -> String {
        match self.written_object(owner, state_key(script)) {
            Some(Value::Str(state)) => state,
            _ => auto_state(script, &self.deferred.vm_scripts),
        }
    }

    /// Run `function`'s body `code` in `frame` — from the start, or along
    /// `path`, with `inner` the suspended frames of the call the path ends in.
    fn run_frame(
        &mut self,
        frame: &mut Frame,
        code: &Code,
        function: &str,
        path: Option<&[VmStep]>,
        inner: &[VmFrame],
    ) -> Run<Flow> {
        if self.depth >= MAX_DEPTH {
            return stop("call depth");
        }
        let callable = code.callable(function)?;
        self.depth += 1;
        let flow = self.exec_block(frame, callable.body, path, inner, true);
        self.depth -= 1;
        flow
    }

    /// `frame` with the parameters of `name` (in `code`) bound to `args`.
    fn enter(&mut self, mut frame: Frame, code: &Code, name: &str, args: Vec<Value>) -> Run<Frame> {
        let callable = code.callable(name)?;
        let mut args = args.into_iter();
        for param in callable.params {
            let value = match args.next() {
                Some(value) => value,
                None => match &param.default {
                    Some(default) => self.eval(&mut frame, &default.node)?,
                    None => Value::default_for(&param.ty.node),
                },
            };
            frame
                .locals
                .insert(param.name.node.0.to_ascii_lowercase(), value);
        }
        Ok(frame)
    }

    /// A function called for its value; it may not wait.
    fn call_function(&mut self, frame: Frame, code: &Code, name: &str, args: Vec<Value>) -> Run<Value> {
        let mut frame = self.enter(frame, code, name, args)?;
        self.value_calls += 1;
        let flow = self.run_frame(&mut frame, code, name, None, &[]);
        self.value_calls -= 1;
        Ok(match flow? {
            Flow::Return(value) => value,
            Flow::Normal => Value::None,
            Flow::Suspend(..) => return stop("a wait in a function called for its value"),
        })
    }

    /// A called function's flow as its call statement's: a suspension adds
    /// the callee's frame to the chain.
    fn callee_flow(&mut self, flow: Flow, frame: &Frame, code: &Code, function: &str) -> Run<Flow> {
        Ok(match flow {
            Flow::Suspend(seconds, path, deeper) => {
                let mut frames = vec![frame_state(frame, code, function, path)?];
                frames.extend(deeper);
                Flow::Suspend(seconds, Vec::new(), frames)
            }
            _ => Flow::Normal,
        })
    }

    /// Continue the suspended frame `state` as `frame` (its script, owner
    /// and properties), with `inner` the frames of the call it waits in.
    fn resume_frame(&mut self, mut frame: Frame, state: &VmFrame, inner: &[VmFrame]) -> Run<Flow> {
        let Some(defined_in) = self
            .deferred
            .vm_scripts
            .get(&state.defined_in.to_ascii_lowercase())
            .cloned()
        else {
            return stop(format!("script {} is not installed", state.defined_in));
        };
        let code = Code {
            script: defined_in,
            state: state.state.clone(),
        };
        frame.locals = restore_locals(state);
        let flow = self.run_frame(&mut frame, &code, &state.function, Some(&state.path), inner)?;
        self.callee_flow(flow, &frame, &code, &state.function)
    }

    /// Continue the suspended callee chain `inner` (outermost first) — none
    /// when the path ended at the wait itself — called from `caller`.
    fn resume_callee(&mut self, caller: &Frame, inner: &[VmFrame]) -> Run<Flow> {
        let Some((callee, deeper)) = inner.split_first() else {
            return Ok(Flow::Normal);
        };
        let Some(script) = self
            .deferred
            .vm_scripts
            .get(&callee.script.to_ascii_lowercase())
            .cloned()
        else {
            return stop(format!("script {} is not installed", callee.script));
        };
        let owner = QuestFormId(callee.owner);
        // The properties the call bound (`script_call_target`): the caller's
        // own for a call into its own script, else the quest's.
        let vmad = if owner == caller.owner && Arc::ptr_eq(&script, &caller.script) {
            caller.vmad.clone()
        } else if let Some(vmad) = self.deferred.quest_scripts.get(&owner) {
            Some(vmad.clone())
        } else {
            // A script on another reference: its own properties.
            self.reference_scripts(owner.0)
                .into_iter()
                .find(|(attached, _)| Arc::ptr_eq(attached, &script))
                .map(|(_, vmad)| (*vmad).clone())
        };
        let frame = Frame {
            script,
            owner,
            vmad,
            locals: HashMap::new(),
        };
        self.resume_frame(frame, callee, deeper)
    }

    /// Run `body`, or — `resume` — continue it from the position a wait
    /// suspended it at.
    fn exec_block(
        &mut self,
        frame: &mut Frame,
        body: &[Spanned<Stmt>],
        resume: Option<&[VmStep]>,
        inner: &[VmFrame],
        // `body` is a function's own body, not a branch inside one: nothing of
        // this function follows its last statement.
        ends_function: bool,
    ) -> Run<Flow> {
        let mut start = 0;
        if let Some(path) = resume {
            let Some((VmStep::Statement(index), rest)) = path.split_first() else {
                return stop("a malformed resume path");
            };
            let index = *index as usize;
            let Some(statement) = body.get(index) else {
                return stop("a resume path past its block");
            };
            // The path ends at the wait itself or at the call it waits in.
            let flow = if rest.is_empty() {
                self.resume_callee(frame, inner)?
            } else {
                self.resume_statement(frame, &statement.node, rest, inner)?
            };
            match flow {
                Flow::Normal => {}
                flow => return Ok(suspend_at(VmStep::Statement(index as u32), flow)),
            }
            start = index + 1;
        }
        for (index, statement) in body.iter().enumerate().skip(start) {
            match self.exec(frame, &statement.node)? {
                Flow::Normal => {
                    // The frame budget ran out: suspend after this statement,
                    // as a zero-second `Utility.Wait` here would.
                    if !(ends_function && index + 1 == body.len()) && self.should_yield(frame) {
                        self.yielded = true;
                        return Ok(suspend_at(
                            VmStep::Statement(index as u32),
                            Flow::Suspend(0.0, Vec::new(), Vec::new()),
                        ));
                    }
                }
                flow => return Ok(suspend_at(VmStep::Statement(index as u32), flow)),
            }
        }
        Ok(Flow::Normal)
    }

    /// Continue inside the `If` branch or `While` body a wait suspended.
    fn resume_statement(
        &mut self,
        frame: &mut Frame,
        statement: &Stmt,
        path: &[VmStep],
        inner: &[VmFrame],
    ) -> Run<Flow> {
        let Some((VmStep::Branch(branch), rest)) = path.split_first() else {
            return stop("a malformed resume path");
        };
        match statement {
            Stmt::If {
                body,
                elseif_clauses,
                else_body,
                ..
            } => {
                let block = match *branch {
                    0 => body.as_slice(),
                    u32::MAX => match else_body {
                        Some(body) => body.as_slice(),
                        None => return stop("a resume path into a missing Else"),
                    },
                    k => match elseif_clauses.get(k as usize - 1) {
                        Some((_, body)) => body.as_slice(),
                        None => return stop("a resume path into a missing ElseIf"),
                    },
                };
                let flow = self.exec_block(frame, block, Some(rest), inner, false)?;
                Ok(suspend_at(VmStep::Branch(*branch), flow))
            }
            Stmt::While { condition, body } => {
                match self.exec_block(frame, body, Some(rest), inner, false)? {
                    Flow::Normal => self.exec_while(frame, &condition.node, body),
                    flow => Ok(suspend_at(VmStep::Branch(0), flow)),
                }
            }
            _ => stop("a resume path into a statement without a block"),
        }
    }

    fn exec_while(&mut self, frame: &mut Frame, condition: &Expr, body: &[Spanned<Stmt>]) -> Run<Flow> {
        while self.eval(frame, condition)?.truthy() {
            self.tick()?;
            match self.exec_block(frame, body, None, &[], false)? {
                Flow::Normal => {}
                flow => return Ok(suspend_at(VmStep::Branch(0), flow)),
            }
        }
        Ok(Flow::Normal)
    }

    fn exec(&mut self, frame: &mut Frame, statement: &Stmt) -> Run<Flow> {
        self.tick()?;
        match statement {
            Stmt::VarDecl(variable) => {
                let value = match &variable.initial_value {
                    Some(init) => self.eval(frame, &init.node)?,
                    None => Value::default_for(&variable.ty.node),
                };
                frame
                    .locals
                    .insert(variable.name.node.0.to_ascii_lowercase(), value);
            }
            Stmt::Assign { target, op, value } => {
                let value = self.eval(frame, &value.node)?;
                let value = match op {
                    AssignOp::Eq => value,
                    op => {
                        let current = self.eval(frame, &target.node)?;
                        let binary = match op {
                            AssignOp::PlusEq => BinaryOp::Add,
                            AssignOp::MinusEq => BinaryOp::Sub,
                            AssignOp::MulEq => BinaryOp::Mul,
                            AssignOp::DivEq => BinaryOp::Div,
                            AssignOp::ModEq => BinaryOp::Mod,
                            AssignOp::Eq => unreachable!(),
                        };
                        arithmetic(binary, &current, &value)?
                    }
                };
                self.assign(frame, &target.node, value)?;
            }
            Stmt::Return(value) => {
                let value = match value {
                    Some(value) => self.eval(frame, &value.node)?,
                    None => Value::None,
                };
                return Ok(Flow::Return(value));
            }
            Stmt::If {
                condition,
                body,
                elseif_clauses,
                else_body,
            } => {
                if self.eval(frame, &condition.node)?.truthy() {
                    let flow = self.exec_block(frame, body, None, &[], false)?;
                    return Ok(suspend_at(VmStep::Branch(0), flow));
                }
                for (k, (condition, body)) in elseif_clauses.iter().enumerate() {
                    if self.eval(frame, &condition.node)?.truthy() {
                        let flow = self.exec_block(frame, body, None, &[], false)?;
                        return Ok(suspend_at(VmStep::Branch(k as u32 + 1), flow));
                    }
                }
                if let Some(body) = else_body {
                    let flow = self.exec_block(frame, body, None, &[], false)?;
                    return Ok(suspend_at(VmStep::Branch(u32::MAX), flow));
                }
            }
            Stmt::While { condition, body } => {
                return self.exec_while(frame, &condition.node, body);
            }
            Stmt::ExprStmt(expression) => {
                return self.call_statement(frame, &expression.node);
            }
        }
        Ok(Flow::Normal)
    }

    fn assign(&mut self, frame: &mut Frame, target: &Expr, value: Value) -> Run<()> {
        match target {
            Expr::Ident(name) => {
                let key = name.0.to_ascii_lowercase();
                if let Some(slot) = frame.locals.get_mut(&key) {
                    *slot = value;
                    return Ok(());
                }
                if is_state_variable(&name.0) {
                    let key = state_key(&frame.script);
                    return self.write_object(frame.owner, key, &value);
                }
                // A script variable or property of the frame's quest script.
                self.write_quest_variable(frame.owner, &name.0, &value)
            }
            Expr::MemberAccess { object, member } => {
                let owner = self.eval(frame, &object.node)?;
                if let Value::Struct(members) = &owner {
                    let mut members = members.borrow_mut();
                    return match Value::member(&members, &member.node.0) {
                        Some(index) => {
                            members[index].1 = value;
                            Ok(())
                        }
                        None => stop(format!("struct member {}", member.node.0)),
                    };
                }
                if member.node.0.eq_ignore_ascii_case("value") && matches!(owner, Value::Form(_)) && !self.is_quest_form(&owner) {
                    let Some(number) = value.number() else {
                        return stop("a non-number GlobalVariable.Value");
                    };
                    let (callee, args) = set_value_call(object, &spanned(Expr::FloatLit(f64::from(number))));
                    return self.call_statement(frame, &Expr::Call { callee, args }).map(|_| ());
                }
                match owner {
                    Value::Form(quest) if self.is_quest(quest) => {
                        self.write_quest_variable(QuestFormId(quest), &member.node.0, &value)
                    }
                    // A variable of a script on another reference, kept
                    // under that reference.
                    Value::Form(reference) if reference != 0 && reference != PLAYER_REF => {
                        self.write_quest_variable(QuestFormId(reference), &member.node.0, &value)
                    }
                    _ => stop(format!("assignment to {}'s {}", owner.text(), member.node.0)),
                }
            }
            Expr::Index { object, index } => {
                let array = self.eval(frame, &object.node)?;
                let index = self.eval(frame, &index.node)?.number().unwrap_or(-1.0) as i64;
                let Value::Array(array) = array else {
                    return stop("index assignment to a non-array");
                };
                let mut items = array.borrow_mut();
                let Some(slot) = usize::try_from(index).ok().and_then(|i| items.get_mut(i)) else {
                    return stop("array index out of range");
                };
                *slot = value;
                Ok(())
            }
            _ => stop("unsupported assignment target"),
        }
    }

    /// A quest script variable: a number in the store `GetVMQuestVariable`
    /// and the lowered `SetQuestVariable` share, an object in
    /// [`crate::QuestScriptObjects`].
    fn write_quest_variable(&mut self, quest: QuestFormId, name: &str, value: &Value) -> Run<()> {
        let key = crate::QuestScriptVariables::property_key(name);
        if let Some(number) = value.number() {
            self.deferred.quest_variables.push((quest, key, number));
            return Ok(());
        }
        self.write_object(quest, key, value)
    }

    /// A non-numeric script variable write under `key`.
    fn write_object(&mut self, owner: QuestFormId, key: ConditionStringId, value: &Value) -> Run<()> {
        let object = match value {
            Value::None => crate::ScriptObject::None,
            Value::Form(form) => crate::ScriptObject::Form(*form),
            Value::Alias { quest, alias } => crate::ScriptObject::Alias {
                quest: *quest,
                alias: *alias,
            },
            Value::Str(text) => crate::ScriptObject::Str(text.clone()),
            other => return stop(format!("a {} write to a script variable", other.text())),
        };
        self.deferred.quest_objects.push((owner, key, object));
        Ok(())
    }

    /// An object or string a script wrote to `quest`'s variable `name`, if
    /// any.
    fn written_quest_object(&self, quest: QuestFormId, name: &str) -> Option<Value> {
        self.written_object(quest, crate::QuestScriptVariables::property_key(name))
    }

    fn written_object(&self, owner: QuestFormId, key: ConditionStringId) -> Option<Value> {
        let pending = self
            .deferred
            .quest_objects
            .iter()
            .rev()
            .find(|(q, k, _)| *q == owner && *k == key)
            .map(|(_, _, value)| value.clone());
        let stored = pending.or_else(|| {
            self.world
                .try_resource::<crate::QuestScriptObjects>()
                .and_then(|objects| objects.get(owner, key))
        });
        Some(match stored? {
            crate::ScriptObject::Form(form) => Value::Form(form),
            crate::ScriptObject::Alias { quest, alias } => Value::Alias { quest, alias },
            crate::ScriptObject::Str(text) => Value::Str(text),
            crate::ScriptObject::None => Value::None,
        })
    }

    fn read_quest_variable(&self, quest: QuestFormId, name: &str) -> Option<f32> {
        let key = crate::QuestScriptVariables::property_key(name);
        self.deferred
            .quest_variables
            .iter()
            .rev()
            .find(|(q, k, _)| *q == quest && *k == key)
            .map(|(_, _, value)| *value)
            .or_else(|| crate::quest_variable(self.world, quest, key))
    }

    fn property(vmad: Option<&ScriptInstanceData>, name: &str) -> Option<Value> {
        let bare = vmad_property_name(name);
        vmad?
            .scripts
            .iter()
            .find_map(|script| script.property(bare))
            .map(|property| Value::from_property(&property.value))
    }

    fn eval(&mut self, frame: &mut Frame, e: &Expr) -> Run<Value> {
        Ok(match e {
            Expr::IntLit(i) => Value::Int(*i as i32),
            Expr::FloatLit(f) => Value::Float(*f as f32),
            Expr::BoolLit(b) => Value::Bool(*b),
            Expr::StringLit(s) => Value::Str(s.clone()),
            Expr::NoneLit => Value::None,
            Expr::Ident(name) => self.read_ident(frame, &name.0)?,
            Expr::Cast { expr, target_type } => {
                let value = self.eval(frame, &expr.node)?;
                cast(value, &target_type.node)
            }
            Expr::UnaryOp { op, operand } => {
                let value = self.eval(frame, &operand.node)?;
                match op {
                    UnaryOp::Not => Value::Bool(!value.truthy()),
                    UnaryOp::Neg => match value {
                        Value::Int(i) => Value::Int(-i),
                        Value::Float(f) => Value::Float(-f),
                        other => return stop(format!("negating {}", other.text())),
                    },
                }
            }
            Expr::BinaryOp { left, op, right } => match op {
                BinaryOp::And => {
                    let l = self.eval(frame, &left.node)?;
                    if !l.truthy() {
                        Value::Bool(false)
                    } else {
                        Value::Bool(self.eval(frame, &right.node)?.truthy())
                    }
                }
                BinaryOp::Or => {
                    let l = self.eval(frame, &left.node)?;
                    if l.truthy() {
                        Value::Bool(true)
                    } else {
                        Value::Bool(self.eval(frame, &right.node)?.truthy())
                    }
                }
                op => {
                    let l = self.eval(frame, &left.node)?;
                    let r = self.eval(frame, &right.node)?;
                    binary(*op, &l, &r)?
                }
            },
            Expr::Index { object, index } => {
                let array = self.eval(frame, &object.node)?;
                let index = self.eval(frame, &index.node)?.number().unwrap_or(-1.0) as i64;
                let Value::Array(array) = array else {
                    return stop("indexing a non-array");
                };
                let items = array.borrow();
                match usize::try_from(index).ok().and_then(|i| items.get(i)) {
                    Some(value) => value.clone(),
                    None => return stop("array index out of range"),
                }
            }
            Expr::NewStruct { ty } => {
                let name = match &ty.node {
                    Type::Object(name) => name.0.clone(),
                    _ => String::new(),
                };
                match self.new_struct(frame, &name) {
                    Some(value) => value,
                    None => return stop(format!("no struct {name}")),
                }
            }
            Expr::New { ty, size } => {
                let size = self.eval(frame, &size.node)?.number().unwrap_or(0.0).max(0.0) as usize;
                Value::Array(Rc::new(RefCell::new(vec![Value::default_for(&ty.node); size])))
            }
            Expr::ArrayLit(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(self.eval(frame, &item.node)?);
                }
                Value::Array(Rc::new(RefCell::new(values)))
            }
            Expr::MemberAccess { object, member } => {
                let owner = self.eval(frame, &object.node)?;
                let name = &member.node.0;
                if name.eq_ignore_ascii_case("length") {
                    if let Value::Array(array) = &owner {
                        return Ok(Value::Int(array.borrow().len() as i32));
                    }
                }
                if let Value::Struct(members) = &owner {
                    let members = members.borrow();
                    return match Value::member(&members, name) {
                        Some(index) => Ok(members[index].1.clone()),
                        None => stop(format!("struct member {name}")),
                    };
                }
                self.read_member(&owner, name)?
            }
            Expr::Call { callee, args } => self.call_value(frame, &callee.node, args)?,
            Expr::Is { .. } | Expr::ParentAccess => return stop("unsupported expression"),
        })
    }

    fn read_ident(&mut self, frame: &mut Frame, name: &str) -> Run<Value> {
        let key = name.to_ascii_lowercase();
        if let Some(value) = frame.locals.get(&key) {
            return Ok(value.clone());
        }
        if key == "self" {
            return Ok(Value::Form(frame.owner.0));
        }
        if is_state_variable(name) {
            let script = Arc::clone(&frame.script);
            return Ok(Value::Str(self.script_state(frame.owner, &script)));
        }
        let script = Arc::clone(&frame.script);
        match self.read_script_member(frame.owner, &script, frame.vmad.as_ref(), name) {
            Some(value) => Ok(value),
            None => stop(format!("unknown name {name}")),
        }
    }

    /// `owner`'s instance of `script` (its properties `vmad`): the property
    /// or script variable `name` — what a script wrote, else the VMAD's
    /// value, else what the script declares. `None` when the script has no
    /// such member.
    fn read_script_member(
        &self,
        owner: QuestFormId,
        script: &Arc<Script>,
        vmad: Option<&ScriptInstanceData>,
        name: &str,
    ) -> Option<Value> {
        if let Some(value) = Self::property(vmad, name) {
            // A numeric property a script wrote reads the written value.
            if value.number().is_some() {
                if let Some(written) = self.read_quest_variable(owner, name) {
                    return Some(if value.is_float() {
                        Value::Float(written)
                    } else {
                        Value::Int(written as i32)
                    });
                }
            }
            // An object property a script re-pointed reads the new object.
            return Some(self.written_quest_object(owner, name).unwrap_or(value));
        }
        let declared = || {
            script_chain(script, &self.deferred.vm_scripts)
                .iter()
                .find_map(|script| declared_default(script, name))
        };
        Some(match member_type_in(script, name, &self.deferred.vm_scripts)? {
            ty @ (Type::Int | Type::Float | Type::Bool) => {
                let value = match self.read_quest_variable(owner, name) {
                    Some(value) => value,
                    None => declared().and_then(|value| value.number()).unwrap_or(0.0),
                };
                match ty {
                    Type::Int => Value::Int(value as i32),
                    Type::Bool => Value::Bool(value != 0.0),
                    _ => Value::Float(value),
                }
            }
            Type::String => self
                .written_quest_object(owner, name)
                .or_else(declared)
                .unwrap_or_else(|| Value::Str(String::new())),
            Type::Object(_) | Type::Var => self.written_quest_object(owner, name).unwrap_or(Value::None),
            Type::Array(_) => return None,
        })
    }

    /// The scripts the VM runs on the reference `form`, with their
    /// properties: (script, its VMAD), loaded references only.
    fn reference_scripts(&self, form: u32) -> Vec<(Arc<Script>, Arc<ScriptInstanceData>)> {
        let Some(entity) = super::ReferenceScriptIndex::lookup(self.world, form) else {
            return Vec::new();
        };
        let Some(attached) = self.world.get::<super::PapyrusReferenceScripts>(entity) else {
            return Vec::new();
        };
        attached
            .0
            .iter()
            .filter_map(|r| {
                let script = self.deferred.vm_scripts.get(&r.script.to_ascii_lowercase())?;
                Some((Arc::clone(script), Arc::clone(&r.vmad)))
            })
            .collect()
    }

    /// `quest.Variable` — another quest script's property or variable.
    fn read_member(&mut self, owner: &Value, name: &str) -> Run<Value> {
        let Value::Form(form) = owner else {
            return stop(format!("member {name} of {}", owner.text()));
        };
        if !self.is_quest(*form) {
            if let Some(value) = self.engine_constant(name) {
                return Ok(value);
            }
            // A member of a script the VM runs on that reference.
            let owner = QuestFormId(*form);
            for (script, vmad) in self.reference_scripts(*form) {
                if let Some(value) = self.read_script_member(owner, &script, Some(&vmad), name) {
                    return Ok(value);
                }
            }
        }
        let quest = QuestFormId(*form);
        let vmad = self.deferred.quest_scripts.get(&quest).cloned();
        // A member of a script attached to the quest the VM has installed:
        // what a script wrote, the VMAD's value, or what it declares.
        if let Some(vmad) = &vmad {
            for attached in &vmad.scripts {
                let Some(script) = self.deferred.vm_scripts.get(&attached.name.to_ascii_lowercase()).cloned() else {
                    continue;
                };
                if let Some(value) = self.read_script_member(quest, &script, Some(vmad), name) {
                    return Ok(value);
                }
            }
        }
        if let Some(value) = Self::property(vmad.as_ref(), name) {
            return Ok(value);
        }
        // `GlobalVariable.Value`.
        if name.eq_ignore_ascii_case("value") {
            return Ok(self
                .world
                .try_resource::<crate::Globals>()
                .and_then(|globals| globals.get(*form))
                .map_or(Value::None, Value::Float));
        }
        match self.read_quest_variable(quest, name) {
            Some(value) => Ok(Value::Float(value)),
            None => stop(format!("member {name} of {form:08X}")),
        }
    }

    /// A new instance of the struct type `name` (`Script#Struct`, or a
    /// struct of the running script's chain), its members at their defaults.
    fn new_struct(&self, frame: &Frame, name: &str) -> Option<Value> {
        let declaration = struct_declaration(&frame.script, name, &self.deferred.vm_scripts)?;
        let members = declaration
            .members
            .iter()
            .map(|member| {
                let value = member
                    .initial_value
                    .as_ref()
                    .and_then(|initial| literal_value(&initial.node))
                    .unwrap_or_else(|| Value::default_for(&member.ty.node));
                (member.name.node.0.clone(), value)
            })
            .collect();
        Some(Value::Struct(Rc::new(RefCell::new(members))))
    }

    /// An array method (CK *Arrays (Papyrus)*), when `callee` calls one on an
    /// array: `Find` / `RFind`, `FindStruct` / `RFindStruct`, `Add`, `Insert`,
    /// `Remove`, `RemoveLast`, `Clear`.
    fn array_call(&mut self, frame: &mut Frame, callee: &Expr, args: &[CallArg]) -> Run<Option<Value>> {
        let Expr::MemberAccess { object, member } = callee else {
            return Ok(None);
        };
        let name = member.node.0.to_ascii_lowercase();
        if !ARRAY_METHODS.contains(&name.as_str()) || static_class(&object.node).is_some() {
            return Ok(None);
        }
        let Value::Array(array) = self.eval(frame, &object.node)? else {
            return Ok(None);
        };
        let values = self.eval_args(frame, args)?;
        let int = |index: usize, default: i32| -> i32 { values.get(index).and_then(Value::int).unwrap_or(default) };
        let mut items = array.borrow_mut();
        let len = items.len() as i32;
        // The member a FindStruct compares.
        let matches = |item: &Value, member: Option<&str>, wanted: &Value| -> bool {
            match member {
                None => item.equals(wanted),
                Some(member) => match item {
                    Value::Struct(members) => {
                        let members = members.borrow();
                        Value::member(&members, member).is_some_and(|index| members[index].1.equals(wanted))
                    }
                    _ => false,
                },
            }
        };
        let (member, wanted, start_arg) = match name.as_str() {
            "findstruct" | "rfindstruct" => {
                let Some(Value::Str(member)) = values.first() else {
                    return stop(format!("{name} without a member name"));
                };
                (Some(member.clone()), values.get(1).cloned().unwrap_or(Value::None), 2)
            }
            _ => (None, values.first().cloned().unwrap_or(Value::None), 1),
        };
        Ok(Some(match name.as_str() {
            "find" | "findstruct" => {
                let start = int(start_arg, 0).max(0);
                (start..len)
                    .find(|&i| matches(&items[i as usize], member.as_deref(), &wanted))
                    .map_or(Value::Int(-1), Value::Int)
            }
            "rfind" | "rfindstruct" => {
                let start = int(start_arg, -1);
                let start = if start < 0 || start >= len { len - 1 } else { start };
                (0..=start)
                    .rev()
                    .find(|&i| matches(&items[i as usize], member.as_deref(), &wanted))
                    .map_or(Value::Int(-1), Value::Int)
            }
            "add" => {
                // "in the case of adding structs, they will all point at the
                // same struct" — the value is shared, not copied.
                for _ in 0..int(1, 1).max(0) {
                    items.push(wanted.clone());
                }
                Value::None
            }
            "insert" => {
                let index = int(1, 0).clamp(0, len) as usize;
                items.insert(index, wanted);
                Value::None
            }
            "remove" => {
                let index = int(0, 0);
                let count = int(1, 1).max(0);
                if index < 0 || index >= len {
                    return stop("array Remove out of range");
                }
                let end = (index + count).min(len);
                items.drain(index as usize..end as usize);
                Value::None
            }
            "removelast" => {
                items.pop();
                Value::None
            }
            "clear" => {
                items.clear();
                Value::None
            }
            _ => return Ok(None),
        }))
    }

    /// An engine class's constant `name`, from the installed classes.
    fn engine_constant(&self, name: &str) -> Option<Value> {
        CONSTANT_CLASSES.iter().find_map(|class| {
            let script = self.deferred.vm_scripts.get(*class)?;
            constant_property(script, name).and_then(property_constant)
        })
    }

    /// The quest script that defines `function` for the quest `quest`, if
    /// any, and the body it runs.
    fn quest_script_with(&self, quest: QuestFormId, function: &str) -> Option<(Arc<Script>, Code)> {
        let vmad = self.deferred.quest_scripts.get(&quest)?;
        vmad.scripts.iter().find_map(|attached| {
            let script = self.deferred.vm_scripts.get(&attached.name.to_ascii_lowercase())?;
            let code = self.resolve(script, quest, function)?;
            Some((Arc::clone(script), code))
        })
    }

    fn eval_args(&mut self, frame: &mut Frame, args: &[CallArg]) -> Run<Vec<Value>> {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.eval(frame, &arg.value.node)?);
        }
        Ok(values)
    }

    /// A call to a script function, when `callee` names one: the frame's own
    /// script (`Fn()`, `Self.Fn()`) or a quest's attached script.
    fn script_call(&mut self, frame: &mut Frame, callee: &Expr, args: &[CallArg]) -> Run<Option<Value>> {
        let Some((callee_frame, code)) = self.script_call_target(frame, callee)? else {
            return Ok(None);
        };
        let values = self.eval_args(frame, args)?;
        self.call_function(callee_frame, &code, &callee_name(callee), values).map(Some)
    }

    /// The fresh frame a call to a script function runs in, and the body it
    /// runs, when `callee` names one.
    fn script_call_target(&mut self, frame: &mut Frame, callee: &Expr) -> Run<Option<(Frame, Code)>> {
        let (receiver, name) = match callee {
            Expr::Ident(name) => (None, name.0.as_str()),
            Expr::MemberAccess { object, member } => (Some(&object.node), member.node.0.as_str()),
            _ => return Ok(None),
        };
        let own = |vm: &Self, frame: &Frame| {
            vm.resolve(&frame.script, frame.owner, name)
                .map(|code| (Arc::clone(&frame.script), frame.owner, frame.vmad.clone(), code))
        };
        let target = match receiver {
            None => own(self, frame),
            Some(object) if static_class(object).is_some() => None,
            // `ScriptName.Fn()`: a global function, which has no `Self`.
            Some(Expr::Ident(script_name)) if self.global_script(frame, &script_name.0).is_some() => {
                let script = self.global_script(frame, &script_name.0).expect("checked above");
                resolve_in(&script, "", name, &self.deferred.vm_scripts)
                    .map(|code| (script, QuestFormId(0), None, code))
            }
            Some(object) => match self.eval(frame, object)? {
                Value::Form(form) if form == frame.owner.0 && own(self, frame).is_some() => own(self, frame),
                Value::Form(form) if self.is_quest(form) => {
                    self.quest_script_with(QuestFormId(form), name).map(|(script, code)| {
                        let vmad = self.deferred.quest_scripts.get(&QuestFormId(form)).cloned();
                        (script, QuestFormId(form), vmad, code)
                    })
                }
                // A function of a script the VM runs on another reference.
                Value::Form(form) if form != 0 => self.reference_scripts(form).into_iter().find_map(|(script, vmad)| {
                    let code = self.resolve(&script, QuestFormId(form), name)?;
                    Some((script, QuestFormId(form), Some((*vmad).clone()), code))
                }),
                _ => None,
            },
        };
        Ok(target.map(|(script, owner, vmad, code)| {
            (
                Frame {
                    script,
                    owner,
                    vmad,
                    locals: HashMap::new(),
                },
                code,
            )
        }))
    }

    /// The script `name` names, when it is not a value in `frame`: the
    /// receiver of a global call.
    fn global_script(&self, frame: &Frame, name: &str) -> Option<Arc<Script>> {
        let key = name.to_ascii_lowercase();
        if key == "self" || frame.locals.contains_key(&key) {
            return None;
        }
        if Self::property(frame.vmad.as_ref(), name).is_some()
            || member_type_in(&frame.script, name, &self.deferred.vm_scripts).is_some()
        {
            return None;
        }
        self.deferred.vm_scripts.get(&key).cloned()
    }

    /// The object reads no effect models.
    fn object_read(&mut self, frame: &mut Frame, callee: &Expr, args: &[CallArg]) -> Run<Option<Value>> {
        let Expr::MemberAccess { object, member } = callee else {
            return Ok(None);
        };
        let name = member.node.0.to_ascii_lowercase();
        if let Some(class) = static_class(&object.node) {
            return Ok(match (class, name.as_str()) {
                ("game", "getplayer") => Some(Value::Form(PLAYER_REF)),
                // CK *GetForm - Game*: the form of that ID, or None when the
                // ID names no loaded plugin.
                ("game", "getform") => {
                    let values = self.eval_args(frame, args)?;
                    let Some(Value::Int(id)) = values.first() else {
                        return stop("GetForm without an Int form id");
                    };
                    let id = *id as u32;
                    let valid = id != 0
                        && self
                            .world
                            .try_resource::<crate::load_order::LoadOrderIdentity>()
                            .is_some_and(|identity| identity.pair(id).is_some());
                    Some(if valid { Value::Form(id) } else { Value::None })
                }
                ("math", function) if MATH_NATIVES.contains(&function) => {
                    let values = self.eval_args(frame, args)?;
                    math_native(function, &values)?
                }
                // CK *GetCurrentGameTime - Utility*: "game days passed".
                ("utility", "getcurrentgametime") => {
                    let hours = self
                        .world
                        .try_resource::<super::ScriptUpdates>()
                        .and_then(|updates| updates.game_hours())
                        .unwrap_or(0.0);
                    Some(Value::Float((hours / 24.0) as f32))
                }
                ("game", "getformfromfile") => {
                    let values = self.eval_args(frame, args)?;
                    // The id exactly: an f32 (`Value::number`) cannot hold
                    // every 32-bit form id.
                    let form_id = match values.first() {
                        Some(Value::Int(id)) => *id as u32,
                        _ => return stop("GetFormFromFile without an Int form id"),
                    };
                    let Some(Value::Str(file)) = values.get(1) else {
                        return stop("GetFormFromFile without a file");
                    };
                    let plugin = byroredux_core::form_id::PluginId::from_filename(file);
                    let form = self
                        .world
                        .try_resource::<crate::load_order::LoadOrderIdentity>()
                        .and_then(|identity| identity.form_from_file(form_id as u32, plugin));
                    Some(form.map_or(Value::None, Value::Form))
                }
                _ => None,
            });
        }
        const ALIAS_GETTERS: [&str; 6] = [
            "getref",
            "getreference",
            "getactorref",
            "getactorreference",
            "getshipref",
            "getshipreference",
        ];
        let reads = ALIAS_GETTERS.contains(&name.as_str())
            || matches!(
                name.as_str(),
                "getowningquest"
                    | "getlinkedref"
                    | "getnthlinkedref"
                    | "getbaseobject"
                    | "getactorbase"
                    | "getleveledactorbase"
                    | "getlocation"
                    | "getcurrentlocation"
                    | "getdialoguetarget"
            );
        if !reads {
            return Ok(None);
        }
        let owner = self.eval(frame, &object.node)?;
        Ok(match (&owner, name.as_str()) {
            (Value::Alias { quest, alias }, getter) if ALIAS_GETTERS.contains(&getter) => Some(
                self.deferred
                    .scene_actor_bindings
                    .resolve(QuestFormId(*quest), i32::from(*alias))
                    .and_then(|entity| self.entity_form(entity))
                    .map_or(Value::None, Value::Form),
            ),
            (Value::Alias { quest, .. }, "getowningquest") => Some(Value::Form(*quest)),
            // `LocationAlias.GetLocation()` — the location runtime's fill.
            (Value::Alias { quest, alias }, "getlocation") => Some(
                self.world
                    .try_resource::<crate::LocationAliasBindings>()
                    .and_then(|bindings| bindings.resolve(QuestFormId(*quest), i32::from(*alias)))
                    .map_or(Value::None, Value::Form),
            ),
            (Value::Form(form), "getcurrentlocation") => Some(
                self.entity_of(*form)
                    .and_then(|entity| crate::location::current_location(self.world, entity))
                    .map_or(Value::None, Value::Form),
            ),
            // CK *GetDialogueTarget*: "the actor this actor is currently in
            // dialogue with" — the player for the line's speaker, the speaker
            // for the player.
            (Value::Form(form), "getdialoguetarget") => {
                let speaker = self.deferred.speaker.and_then(|entity| self.entity_form(entity));
                Some(match speaker {
                    Some(speaker) if speaker == *form => Value::Form(PLAYER_REF),
                    Some(speaker) if *form == PLAYER_REF => Value::Form(speaker),
                    _ => Value::None,
                })
            }
            (Value::Form(_), "getowningquest") => Some(Value::Form(frame.owner.0)),
            (Value::Form(form), "getlinkedref") => {
                let keyword = match args.first() {
                    Some(arg) => match self.eval(frame, &arg.value.node)? {
                        Value::Form(keyword) => keyword,
                        _ => 0,
                    },
                    None => 0,
                };
                Some(self.linked_ref(*form, keyword).map_or(Value::None, Value::Form))
            }
            // CK *GetNthLinkedRef*: 0 is the object itself, n follows the
            // unkeyworded link n times.
            (Value::Form(form), "getnthlinkedref") => {
                let hops = match args.first() {
                    Some(arg) => self.eval(frame, &arg.value.node)?.number().unwrap_or(0.0),
                    None => 0.0,
                };
                let mut current = Some(*form);
                for _ in 0..(hops.max(0.0) as usize).min(64) {
                    current = current.and_then(|form| self.linked_ref(form, 0));
                }
                Some(current.map_or(Value::None, Value::Form))
            }
            (Value::Form(form), "getbaseobject" | "getactorbase" | "getleveledactorbase") => {
                Some(self.base_of(*form).map_or(Value::None, Value::Form))
            }
            (Value::None, _) => return stop(format!("{name} on None")),
            _ => None,
        })
    }

    fn entity_form(&self, entity: EntityId) -> Option<u32> {
        let player = self
            .world
            .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
            .map(|player| player.0);
        if Some(entity) == player {
            return Some(PLAYER_REF);
        }
        crate::condition::entity_global_form_id(self.world, entity)
    }

    fn entity_of(&self, form: u32) -> Option<EntityId> {
        crate::condition::resolve_entity_by_global_form_id(self.world, form)
    }

    fn linked_ref(&self, form: u32, keyword: u32) -> Option<u32> {
        let entity = self.entity_of(form)?;
        self.world
            .get::<crate::scene::SceneAliasCandidate>(entity)?
            .linked_refs
            .iter()
            .find(|(link, _)| *link == keyword || (keyword == 0 && *link == 0xFFFF_FFFF))
            .map(|(_, target)| *target)
    }

    fn base_of(&self, form: u32) -> Option<u32> {
        let entity = self.entity_of(form)?;
        crate::relationships::actor_base(self.world, entity)
    }

    /// Re-write `callee(args)` over bound values for the primitives.
    fn synthesize(
        &mut self,
        frame: &mut Frame,
        callee: &Expr,
        args: &[CallArg],
    ) -> Run<(Expr, NativeBindings, ScriptInstanceData)> {
        let mut synth = Synth::default();
        let mut bound_receiver = None;
        let callee = match callee {
            Expr::MemberAccess { object, member } => {
                let receiver = if static_class(&object.node).is_some() {
                    (**object).clone()
                } else {
                    let value = self.eval(frame, &object.node)?;
                    if matches!(value, Value::None) {
                        return stop(format!("{} on None", member.node.0));
                    }
                    let receiver = synth.bind(self, &value)?;
                    bound_receiver = Some(receiver.clone());
                    receiver
                };
                Expr::MemberAccess {
                    object: Box::new(receiver),
                    member: member.clone(),
                }
            }
            // A bare global native (`GetCurrentGameTime()` style).
            other => other.clone(),
        };
        let mut bound = Vec::with_capacity(args.len());
        for arg in args {
            // An engine constant passes as the named member it was written
            // as (`Self.Motion_Keyframed`): its number differs per game, the
            // name does not.
            let constant = constant_name(&arg.value.node).filter(|name| self.engine_constant(name).is_some());
            let value = match (constant, &bound_receiver) {
                (Some(name), Some(receiver)) => spanned(Expr::MemberAccess {
                    object: Box::new(receiver.clone()),
                    member: spanned(Identifier(name.to_owned())),
                }),
                _ => {
                    let value = self.eval(frame, &arg.value.node)?;
                    synth.bind(self, &value)?
                }
            };
            bound.push(CallArg { name: None, value });
        }
        let call = Expr::Call {
            callee: Box::new(spanned(callee)),
            args: bound,
        };
        let (bindings, vmad) = synth.vmad();
        Ok((call, bindings, vmad))
    }

    fn call_value(&mut self, frame: &mut Frame, callee: &Expr, args: &[CallArg]) -> Run<Value> {
        if let Some(value) = self.array_call(frame, callee, args)? {
            return Ok(value);
        }
        if let Some(value) = self.object_read(frame, callee, args)? {
            return Ok(value);
        }
        if let Some(value) = self.script_call(frame, callee, args)? {
            return Ok(value);
        }
        if self_call_named(callee, "getstate") {
            let script = Arc::clone(&frame.script);
            return Ok(Value::Str(self.script_state(frame.owner, &script)));
        }
        let (call, bindings, vmad) = self.synthesize(frame, callee, args)?;
        if let Some(value) = classify_native_value(&call, &bindings) {
            let number = evaluate_value(&value, self.context, Some(&vmad), self.world, self.stages, self.deferred);
            return match number {
                Some(number) => Ok(Value::Float(number)),
                None => stop("a numeric read could not be resolved"),
            };
        }
        if let Some(guards) = classify_native_guard(&call, &bindings) {
            let mut all = true;
            for guard in &guards {
                match evaluate_guard(guard, self.context, Some(&vmad), self.world, self.stages, self.objectives, self.deferred) {
                    Some(pass) => all &= pass,
                    None => return stop("a Bool read could not be resolved"),
                }
            }
            return Ok(Value::Bool(all));
        }
        stop(format!("unsupported call {}", callee_name(callee)))
    }

    fn call_statement(&mut self, frame: &mut Frame, expression: &Expr) -> Run<Flow> {
        let Expr::Call { callee, args } = expression else {
            // A bare expression statement has no effect.
            self.eval(frame, expression)?;
            return Ok(Flow::Normal);
        };
        // `Utility.Wait(seconds)` suspends the run, every frame of it.
        if let Expr::MemberAccess { object, member } = &callee.node {
            if static_class(&object.node) == Some("utility") && member.node.0.eq_ignore_ascii_case("wait") {
                let seconds = match args.first() {
                    Some(arg) => self.eval(frame, &arg.value.node)?.number().unwrap_or(0.0),
                    None => 0.0,
                };
                // #5681 — the seconds are a run-time value here, so a literal
                // the lowerer already declined can still reach the VM. A wait
                // past the shared cap would park an immortal continuation in
                // `FragmentExecutionQueue`; stopping the run is the
                // decline-safe outcome, same choice the provider lowerer
                // makes for the same literal.
                if seconds > MAX_PROVIDER_WAIT_SECONDS {
                    return stop("Utility.Wait beyond MAX_PROVIDER_WAIT_SECONDS");
                }
                return Ok(Flow::Suspend(seconds.max(0.0), Vec::new(), Vec::new()));
            }
        }
        if self.array_call(frame, &callee.node, args)?.is_some() {
            return Ok(Flow::Normal);
        }
        if self.object_read(frame, &callee.node, args)?.is_some() {
            return Ok(Flow::Normal);
        }
        // A script function called as a statement may wait.
        if let Some((callee_frame, code)) = self.script_call_target(frame, &callee.node)? {
            let name = callee_name(&callee.node);
            let values = self.eval_args(frame, args)?;
            let mut callee_frame = self.enter(callee_frame, &code, &name, values)?;
            // The caller stays on the stack while the callee runs: if it holds
            // a local a suspension cannot store, the callee must not suspend.
            let blocked = !locals_storable(frame);
            self.blocked_callers += usize::from(blocked);
            let flow = self.run_frame(&mut callee_frame, &code, &name, None, &[]);
            self.blocked_callers -= usize::from(blocked);
            return self.callee_flow(flow?, &callee_frame, &code, &name);
        }
        // `GotoState`'s `OnEndState` / `OnBeginState` the script does not
        // handle: the empty events every script inherits.
        if is_state_event(&callee.node) {
            return Ok(Flow::Normal);
        }
        if let Some((object, name)) = quest_update_call(&callee.node) {
            if let Value::Form(quest) = self.eval(frame, object)? {
                if self.is_quest(quest) {
                    return self.quest_update(frame, quest, &name, args).map(|_| Flow::Normal);
                }
            }
        }
        if let Some(name) = update_call(&callee.node) {
            let mut numbers = Vec::with_capacity(args.len());
            for arg in args {
                numbers.push(self.eval(frame, &arg.value.node)?.number());
            }
            let Some(request) = super::UpdateRequest::of_call(&name, &numbers) else {
                return stop(format!("{name} without its numbers"));
            };
            let script = frame.script.name.node.0.clone();
            self.deferred.script_updates.push((frame.owner.0, script, request));
            return Ok(Flow::Normal);
        }
        if self_call_named(&callee.node, "gotostate") {
            let state = match args.first() {
                Some(arg) => self.eval(frame, &arg.value.node)?.text(),
                None => String::new(),
            };
            self.goto_state(frame, state)?;
            return Ok(Flow::Normal);
        }
        if is_trace(&callee.node) {
            return Ok(Flow::Normal);
        }
        let (call, bindings, vmad) = self.synthesize(frame, &callee.node, args)?;
        let Some(effect) = classify_native_effect(&call, &bindings) else {
            return stop(format!("unsupported statement {}", callee_name(&callee.node)));
        };
        if crate::translate::effects::is_latent(&effect) {
            return stop("a latent call");
        }
        let advances = apply_effects(
            std::slice::from_ref(&effect),
            self.context,
            Some(&vmad),
            self.world,
            self.stages,
            self.objectives,
            self.deferred,
        );
        self.advances.extend(advances);
        Ok(Flow::Normal)
    }
}

/// `Global.Value = n` as the native it is: `Global.SetValue(n)`.
fn set_value_call(global: &Spanned<Expr>, value: &Spanned<Expr>) -> (Box<Spanned<Expr>>, Vec<CallArg>) {
    let callee = spanned(Expr::MemberAccess {
        object: Box::new(global.clone()),
        member: spanned(Identifier("SetValue".into())),
    });
    let args = vec![CallArg {
        name: None,
        value: value.clone(),
    }];
    (Box::new(callee), args)
}

/// An update (un)registration on another object: the receiver and the
/// lowercase name.
fn quest_update_call(callee: &Expr) -> Option<(&Expr, String)> {
    let Expr::MemberAccess { object, member } = callee else {
        return None;
    };
    let name = member.node.0.to_ascii_lowercase();
    let on_self = matches!(&object.node, Expr::Ident(id) if id.0.eq_ignore_ascii_case("self"));
    (super::UpdateRequest::handler(&name).is_some() && !on_self && static_class(&object.node).is_none())
        .then_some((&object.node, name))
}

/// An update (un)registration on the script itself, by lowercase name.
fn update_call(callee: &Expr) -> Option<String> {
    let name = callee_name(callee).to_ascii_lowercase();
    (super::UpdateRequest::handler(&name).is_some() && self_call_named(callee, &name)).then_some(name)
}

/// A call of `name` (lowercase) on the script itself.
fn self_call_named(callee: &Expr, name: &str) -> bool {
    let (receiver, called) = match callee {
        Expr::Ident(called) => (None, called),
        Expr::MemberAccess { object, member } => (Some(&object.node), &member.node),
        _ => return false,
    };
    receiver.is_none_or(|r| matches!(r, Expr::Ident(id) if id.0.eq_ignore_ascii_case("self")))
        && called.0.eq_ignore_ascii_case(name)
}

/// `OnBeginState()` / `OnEndState()` on the script itself.
fn is_state_event(callee: &Expr) -> bool {
    let (receiver, name) = match callee {
        Expr::Ident(name) => (None, name),
        Expr::MemberAccess { object, member } => (Some(&object.node), &member.node),
        _ => return false,
    };
    let on_self = receiver.is_none_or(|r| matches!(r, Expr::Ident(id) if id.0.eq_ignore_ascii_case("self")));
    on_self && (name.0.eq_ignore_ascii_case("onbeginstate") || name.0.eq_ignore_ascii_case("onendstate"))
}

fn is_trace(callee: &Expr) -> bool {
    let Expr::MemberAccess { object, member } = callee else {
        return false;
    };
    static_class(&object.node) == Some("debug") && member.node.0.to_ascii_lowercase().starts_with("trace")
}

fn callee_name(callee: &Expr) -> String {
    match callee {
        Expr::MemberAccess { member, .. } => member.node.0.clone(),
        Expr::Ident(name) => name.0.clone(),
        _ => "<call>".into(),
    }
}

fn cast(value: Value, ty: &Type) -> Value {
    match ty {
        Type::Bool => Value::Bool(value.truthy()),
        Type::Int => match value {
            Value::Str(s) => Value::Int(s.trim().parse::<f32>().unwrap_or(0.0) as i32),
            other => Value::Int(other.number().unwrap_or(0.0) as i32),
        },
        Type::Float => match value {
            Value::Str(s) => Value::Float(s.trim().parse::<f32>().unwrap_or(0.0)),
            other => Value::Float(other.number().unwrap_or(0.0)),
        },
        Type::String => Value::Str(value.text()),
        // Object casts keep the object; Papyrus yields None on a mismatch,
        // which needs the type of a form this VM does not track.
        _ => value,
    }
}

fn arithmetic(op: BinaryOp, l: &Value, r: &Value) -> Run<Value> {
    if op == BinaryOp::Add && (matches!(l, Value::Str(_)) || matches!(r, Value::Str(_))) {
        return Ok(Value::Str(format!("{}{}", l.text(), r.text())));
    }
    let (Some(a), Some(b)) = (l.number(), r.number()) else {
        return stop(format!("arithmetic on {} and {}", l.text(), r.text()));
    };
    if let (Some(a), Some(b)) = (l.int(), r.int()) {
        // Papyrus `Int` arithmetic is 32-bit and exact.
        return Ok(Value::Int(match op {
            BinaryOp::Add => a.wrapping_add(b),
            BinaryOp::Sub => a.wrapping_sub(b),
            BinaryOp::Mul => a.wrapping_mul(b),
            BinaryOp::Div if b == 0 => return stop("integer division by zero"),
            BinaryOp::Div => a.wrapping_div(b),
            BinaryOp::Mod if b == 0 => return stop("modulo by zero"),
            BinaryOp::Mod => a.wrapping_rem(b),
            _ => return stop("not an arithmetic operator"),
        }));
    }
    Ok(match op {
        BinaryOp::Add => Value::Float(a + b),
        BinaryOp::Sub => Value::Float(a - b),
        BinaryOp::Mul => Value::Float(a * b),
        BinaryOp::Div => Value::Float(a / b),
        BinaryOp::Mod => {
            if b as i32 == 0 {
                return stop("modulo by zero");
            }
            Value::Int((a as i32) % (b as i32))
        }
        _ => return stop("not an arithmetic operator"),
    })
}

fn binary(op: BinaryOp, l: &Value, r: &Value) -> Run<Value> {
    Ok(match op {
        BinaryOp::Eq => Value::Bool(l.equals(r)),
        BinaryOp::Ne => Value::Bool(!l.equals(r)),
        BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
            let ordering = match (l, r) {
                (Value::Str(a), Value::Str(b)) => a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()),
                _ if l.int().is_some() && r.int().is_some() => l.int().cmp(&r.int()),
                _ => {
                    let (Some(a), Some(b)) = (l.number(), r.number()) else {
                        return stop(format!("comparing {} and {}", l.text(), r.text()));
                    };
                    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
                }
            };
            Value::Bool(match op {
                BinaryOp::Lt => ordering.is_lt(),
                BinaryOp::Le => ordering.is_le(),
                BinaryOp::Gt => ordering.is_gt(),
                _ => ordering.is_ge(),
            })
        }
        BinaryOp::StrCat => Value::Str(format!("{}{}", l.text(), r.text())),
        op => arithmetic(op, l, r)?,
    })
}

// ── Ahead-of-time check ──────────────────────────────────────────────

/// What the static check knows of an expression's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Num,
    Bool,
    Str,
    Object,
    Quest,
    Player,
    Alias,
    Array,
    /// A Fallout 4 struct.
    Struct,
    None,
}

/// Where the check resolves scripts: the library populate supplies.
pub use crate::translate::effects::ScriptLibrary as CheckLibrary;

struct Checker<'l> {
    library: Option<&'l dyn CheckLibrary>,
    /// Whether the function being checked waits (itself or in a function
    /// it calls as a statement).
    waits: bool,
    /// The functions that wait, by lowercased (script, function).
    waiting: HashSet<(String, String)>,
    /// The first reason the check failed, for diagnostics.
    reason: Option<String>,
    /// Scripts the run will need, by lowercased name.
    scripts: HashMap<String, Arc<Script>>,
    visiting: HashSet<(String, String)>,
}

struct CheckFrame {
    script: Arc<Script>,
    /// What `Self` is: a quest, or a reference for a reference script.
    self_kind: Kind,
    locals: HashMap<String, Kind>,
    /// Locals' and parameters' declared types (a quest local's script type).
    types: HashMap<String, Type>,
}

/// Whether `function` of `script` (a fragment of a quest) runs whole on this
/// VM; `Some` carries every script the run needs.
pub fn check_function(
    script: Arc<Script>,
    function: &str,
    library: Option<&dyn CheckLibrary>,
) -> Option<Vec<Arc<Script>>> {
    check_function_reason(script, function, library).ok()
}

/// [`check_function`] with the first reason it fails.
pub fn check_function_reason(
    script: Arc<Script>,
    function: &str,
    library: Option<&dyn CheckLibrary>,
) -> Result<Vec<Arc<Script>>, String> {
    check_reason(script, function, library, Kind::Quest)
}

/// Whether a reference script runs on this VM: it handles at least one of
/// `events`, and every handler of them — in any state, down the `extends`
/// chain — runs whole. `Ok` carries the events it handles and every script
/// the runs need; `Err` the first reason it does not run.
pub fn check_reference_script(
    script: Arc<Script>,
    events: &[&str],
    library: Option<&dyn CheckLibrary>,
) -> Result<(Vec<String>, Vec<Arc<Script>>), String> {
    let mut checker = Checker::new(library, &script);
    let mut handled = Vec::new();
    for event in events {
        if checker.versions(&script, event).is_empty() {
            continue;
        }
        if checker.check_call_into(&script, event, true, Kind::Object).is_none() {
            let reason = checker.reason.take().unwrap_or_else(|| "unsupported".into());
            return Err(format!("{event}: {reason}"));
        }
        handled.push((*event).to_owned());
    }
    if handled.is_empty() {
        return Err("handles no delivered event".into());
    }
    Ok((handled, checker.scripts.into_values().collect()))
}

/// The installed scripts as a [`CheckLibrary`].
struct Installed<'a>(&'a HashMap<String, Arc<Script>>);

impl crate::translate::effects::ScriptLibrary for Installed<'_> {
    fn script(&self, name: &str) -> Option<Arc<Script>> {
        self.0.get(&name.to_ascii_lowercase()).cloned()
    }
}

/// Whether `handler` of the installed `script` runs whole on this VM,
/// against the installed scripts — on a quest (`on_quest`) or a reference.
pub(super) fn handler_runs(scripts: &HashMap<String, Arc<Script>>, script: &str, handler: &str, on_quest: bool) -> bool {
    let Some(script) = scripts.get(&script.to_ascii_lowercase()) else {
        return false;
    };
    let kind = if on_quest { Kind::Quest } else { Kind::Object };
    let library = Installed(scripts);
    let mut checker = Checker::new(Some(&library), script);
    checker.versions(script, handler).is_empty() || checker.check_call_into(script, handler, true, kind).is_some()
}

fn check_reason(
    script: Arc<Script>,
    function: &str,
    library: Option<&dyn CheckLibrary>,
    self_kind: Kind,
) -> Result<Vec<Arc<Script>>, String> {
    let mut checker = Checker::new(library, &script);
    match checker.check_call_into(&script, function, true, self_kind) {
        Some(_) => Ok(checker.scripts.into_values().collect()),
        None => Err(checker.reason.unwrap_or_else(|| "unsupported".into())),
    }
}

fn kind_of_type(ty: &Type, library: Option<&dyn CheckLibrary>) -> Kind {
    match ty {
        Type::Int | Type::Float => Kind::Num,
        Type::Bool => Kind::Bool,
        Type::String => Kind::Str,
        Type::Array(_) => Kind::Array,
        Type::Var => Kind::None,
        Type::Object(name) => {
            let lower = name.0.to_ascii_lowercase();
            if lower == "quest" || library.is_some_and(|library| extends_quest(library, &lower, 0)) {
                Kind::Quest
            } else if lower.contains("alias") {
                Kind::Alias
            } else {
                Kind::Object
            }
        }
    }
}

fn extends_quest(library: &dyn CheckLibrary, name: &str, depth: usize) -> bool {
    if depth > 16 {
        return false;
    }
    let Some(script) = library.script(name) else {
        return false;
    };
    match &script.parent {
        Some(parent) if parent.node.0.eq_ignore_ascii_case("quest") => true,
        Some(parent) => extends_quest(library, &parent.node.0.to_ascii_lowercase(), depth + 1),
        None => false,
    }
}

impl<'l> Checker<'l> {
    fn new(library: Option<&'l dyn CheckLibrary>, script: &Arc<Script>) -> Self {
        let mut checker = Checker {
            library,
            reason: None,
            waits: false,
            waiting: HashSet::new(),
            scripts: HashMap::new(),
            visiting: HashSet::new(),
        };
        checker
            .scripts
            .insert(script.name.node.0.to_ascii_lowercase(), Arc::clone(script));
        checker
    }

    /// The type of `member` of the script an Object receiver is declared as
    /// (not an engine class).
    fn reference_member_type(&mut self, frame: &CheckFrame, owner: Kind, receiver: &Expr, member: &str) -> Option<Type> {
        if owner != Kind::Object {
            return None;
        }
        let script = self.declared_script(frame, receiver)?;
        if is_engine_class(&script) {
            return None;
        }
        match self.member_type(&script, member)? {
            Type::Array(_) => None,
            ty => Some(ty),
        }
    }

    /// The check's kind of the declared type `ty` in `script`: a struct type
    /// is a struct, the rest as [`kind_of_type`].
    fn kind_of_declared(&mut self, script: &Arc<Script>, ty: &Type) -> Kind {
        if let Type::Object(name) = ty {
            if self.struct_declaration(script, &name.0).is_some() {
                return Kind::Struct;
            }
        }
        kind_of_type(ty, self.library)
    }

    /// The declaration of the struct type `name` from `script`'s chain or,
    /// for `Script#Struct`, that script — registering it for the run.
    fn struct_declaration(&mut self, script: &Arc<Script>, name: &str) -> Option<byroredux_papyrus::ast::Struct> {
        let owners = match name.split_once('#') {
            Some((owner, _)) => {
                let owner = self.library?.script(owner)?;
                self.scripts
                    .insert(owner.name.node.0.to_ascii_lowercase(), Arc::clone(&owner));
                vec![owner]
            }
            None => self.chain(script),
        };
        let mut known: HashMap<String, Arc<Script>> = HashMap::new();
        for owner in &owners {
            known.insert(owner.name.node.0.to_ascii_lowercase(), Arc::clone(owner));
        }
        struct_declaration(&owners[0], name, &known)
    }

    /// The declared type of `e`, where the check can know it: a typed local
    /// or member, an array's element, a struct's member, a cast.
    fn static_type(&mut self, frame: &CheckFrame, e: &Expr) -> Option<Type> {
        match e {
            Expr::Ident(name) => match frame.types.get(&name.0.to_ascii_lowercase()) {
                Some(ty) => Some(ty.clone()),
                None => {
                    let script = Arc::clone(&frame.script);
                    self.member_type(&script, &name.0)
                }
            },
            Expr::Cast { target_type, .. } => Some(target_type.node.clone()),
            Expr::Index { object, .. } => match self.static_type(frame, &object.node)? {
                Type::Array(element) => Some(*element),
                _ => None,
            },
            Expr::MemberAccess { object, member } => {
                let owner = self.static_type(frame, &object.node)?;
                let Type::Object(owner) = owner else {
                    return None;
                };
                let script = Arc::clone(&frame.script);
                if let Some(declaration) = self.struct_declaration(&script, &owner.0) {
                    return declaration
                        .members
                        .iter()
                        .find(|m| m.name.node.0.eq_ignore_ascii_case(&member.node.0))
                        .map(|m| m.ty.node.clone());
                }
                let declared = self.library?.script(&owner.0)?;
                self.member_type(&declared, &member.node.0)
            }
            _ => None,
        }
    }

    /// An array method's result kind (CK *Arrays (Papyrus)*).
    fn check_array_call(&mut self, name: &str, args: &[Arg], statement: bool) -> Option<Kind> {
        let num = |arg: &Arg| matches!(arg, Arg::Literal(Expr::IntLit(_)) | Arg::Kind(Kind::Num));
        let ok = match name {
            "find" | "rfind" => (1..=2).contains(&args.len()) && args.get(1).is_none_or(num),
            "findstruct" | "rfindstruct" => {
                (2..=3).contains(&args.len())
                    && matches!(args[0], Arg::Literal(Expr::StringLit(_)))
                    && args.get(2).is_none_or(num)
            }
            "add" => statement && (1..=2).contains(&args.len()) && args.get(1).is_none_or(num),
            "insert" => statement && args.len() == 2 && num(&args[1]),
            "remove" => statement && (1..=2).contains(&args.len()) && args.iter().all(num),
            "removelast" | "clear" => statement && args.is_empty(),
            _ => false,
        };
        if !ok {
            return self.fail(|| format!("array {name}/{}", args.len()));
        }
        Some(if name.contains("find") { Kind::Num } else { Kind::None })
    }

    /// An engine class's constant `name`'s type, registering the class.
    fn engine_constant_type(&mut self, name: &str) -> Option<Type> {
        CONSTANT_CLASSES.iter().find_map(|class| {
            let script = self.library?.script(class)?;
            let ty = constant_property(&script, name)?.ty.node.clone();
            self.scripts.insert((*class).to_owned(), Arc::clone(&script));
            Some(ty)
        })
    }

    /// `name`'s declared type in `script` or its parents (registering the
    /// parents the run will need).
    fn member_type(&mut self, script: &Script, name: &str) -> Option<Type> {
        if let Some(ty) = script_member_type(script, name) {
            return Some(ty);
        }
        let mut parent = script.parent.as_ref().map(|p| p.node.0.clone());
        for _ in 0..16 {
            let name_of_parent = parent?;
            let lower = name_of_parent.to_ascii_lowercase();
            if matches!(lower.as_str(), "quest" | "topicinfo" | "scene" | "objectreference" | "actor") {
                // An engine class lends only its constants.
                let class = self.library?.script(&name_of_parent)?;
                let ty = constant_property(&class, name)?.ty.node.clone();
                self.scripts.insert(lower, Arc::clone(&class));
                return Some(ty);
            }
            let script = self.library?.script(&name_of_parent)?;
            self.scripts.insert(lower, Arc::clone(&script));
            if let Some(ty) = script_member_type(&script, name) {
                return Some(ty);
            }
            parent = script.parent.as_ref().map(|p| p.node.0.clone());
        }
        None
    }

    fn fail<T>(&mut self, reason: impl FnOnce() -> String) -> Option<T> {
        if self.reason.is_none() {
            self.reason = Some(reason());
        }
        None
    }

    /// `script` and the scripts it extends, registering each for the run.
    fn chain(&mut self, script: &Arc<Script>) -> Vec<Arc<Script>> {
        let mut chain = vec![Arc::clone(script)];
        while chain.len() < 16 {
            let Some(parent) = chain.last().and_then(|last| last.parent.as_ref()).map(|p| p.node.0.clone()) else {
                break;
            };
            let Some(parent) = self.library.and_then(|library| library.script(&parent)) else {
                break;
            };
            self.scripts
                .insert(parent.name.node.0.to_ascii_lowercase(), Arc::clone(&parent));
            chain.push(parent);
        }
        chain
    }

    /// Every body a call of `name` on an instance of `script` may run — in
    /// any state, down the `extends` chain to the engine classes — as
    /// (script, state).
    fn versions(&mut self, script: &Arc<Script>, name: &str) -> Vec<(Arc<Script>, Option<String>)> {
        let mut versions = Vec::new();
        for script in self.chain(script).into_iter().take_while(|script| !is_engine_class(script)) {
            let states = std::iter::once(None).chain(state_names(&script).map(|state| Some(state.to_owned())));
            for state in states.collect::<Vec<_>>() {
                if callable_in(&script, state.as_deref(), name).is_some() {
                    versions.push((Arc::clone(&script), state));
                }
            }
        }
        versions
    }

    /// Check a call of `function` on an instance of `script` whose `Self`
    /// is `self_kind`: every version of it. `statement`: whether the call is
    /// a statement — a function that waits may not be called for its value.
    fn check_call_into(&mut self, script: &Arc<Script>, function: &str, statement: bool, self_kind: Kind) -> Option<Kind> {
        let key = (script.name.node.0.to_ascii_lowercase(), function.to_ascii_lowercase());
        let versions = self.versions(script, function);
        let returns = versions
            .iter()
            .find_map(|(defined_in, state)| callable_in(defined_in, state.as_deref(), function)?.returns.cloned())
            .map_or(Kind::None, |ty| kind_of_type(&ty.node, self.library));
        if versions.is_empty() {
            return self.fail(|| format!("no function {function}"));
        }
        if !self.visiting.insert(key.clone()) {
            // Checked already, or recursion: assume the declared return.
            return self.call_waits(self.waiting.contains(&key), statement).map(|_| returns);
        }
        let outer = std::mem::replace(&mut self.waits, false);
        for (defined_in, state) in &versions {
            let callable = callable_in(defined_in, state.as_deref(), function)?;
            let mut frame = CheckFrame {
                script: Arc::clone(script),
                self_kind,
                locals: HashMap::new(),
                types: HashMap::new(),
            };
            for param in callable.params {
                let key = param.name.node.0.to_ascii_lowercase();
                let kind = self.kind_of_declared(script, &param.ty.node);
                frame.locals.insert(key.clone(), kind);
                frame.types.insert(key, param.ty.node.clone());
            }
            callable
                .body
                .iter()
                .try_for_each(|statement| self.check_stmt(&mut frame, &statement.node))?;
        }
        let waits = std::mem::replace(&mut self.waits, outer);
        if waits {
            self.waiting.insert(key);
        }
        self.call_waits(waits, statement).map(|_| returns)
    }

    /// Check an event handler a registration delivers to later, installing
    /// the scripts it needs when it runs. A handler the VM cannot run does
    /// not fail the registering code — the delivery checks it again and
    /// skips it — so its failed check is rolled back. It may wait; the
    /// registering call does not.
    fn check_handler(&mut self, script: &Arc<Script>, handler: &str, self_kind: Kind) -> Option<()> {
        let saved = (
            self.waits,
            self.reason.clone(),
            self.visiting.clone(),
            self.waiting.clone(),
            self.scripts.clone(),
        );
        if self.check_call_into(script, handler, true, self_kind).is_none() {
            (self.reason, self.visiting, self.waiting, self.scripts) = (saved.1, saved.2, saved.3, saved.4);
        }
        self.waits = saved.0;
        Some(())
    }

    /// A call to a function that `waits` makes its caller wait too.
    fn call_waits(&mut self, waits: bool, statement: bool) -> Option<()> {
        if waits {
            if !statement {
                return self.fail(|| "a wait in a function called for its value".into());
            }
            self.waits = true;
        }
        Some(())
    }

    fn check_block(&mut self, frame: &mut CheckFrame, body: &[Spanned<Stmt>]) -> Option<()> {
        for statement in body {
            self.check_stmt(frame, &statement.node)?;
        }
        Some(())
    }

    fn check_stmt(&mut self, frame: &mut CheckFrame, statement: &Stmt) -> Option<()> {
        match statement {
            Stmt::VarDecl(variable) => {
                if let Some(init) = &variable.initial_value {
                    self.kind(frame, &init.node)?;
                }
                let key = variable.name.node.0.to_ascii_lowercase();
                let kind = self.kind_of_declared(&frame.script, &variable.ty.node);
                frame.locals.insert(key.clone(), kind);
                frame.types.insert(key, variable.ty.node.clone());
            }
            Stmt::Assign { target, value, .. } => {
                let value_kind = self.kind(frame, &value.node)?;
                match &target.node {
                    Expr::Ident(name) => {
                        let key = name.0.to_ascii_lowercase();
                        if !frame.locals.contains_key(&key) && value_kind == Kind::Array {
                            return self.fail(|| format!("{value_kind:?} write to a script variable"));
                        }
                    }
                    Expr::MemberAccess { object, member } => {
                        let owner = self.kind(frame, &object.node)?;
                        if owner == Kind::Struct {
                            if self.static_type(frame, &target.node).is_none() {
                                let member = member.node.0.clone();
                                return self.fail(|| format!("assignment to .{member} of an untyped struct"));
                            }
                        } else if owner == Kind::Object && member.node.0.eq_ignore_ascii_case("value") && value_kind == Kind::Num {
                            // `Global.Value = n`.
                            let (callee, args) = set_value_call(object, value);
                            self.check_call(frame, &callee.node, &args, true)?;
                        } else if value_kind != Kind::Array
                            && self.reference_member_type(frame, owner, &object.node, &member.node.0).is_some()
                        {
                            // A variable of a script on another reference.
                        } else if owner != Kind::Quest || matches!(value_kind, Kind::Str | Kind::Array) {
                            return self.fail(|| format!("{value_kind:?} assignment to a member of {owner:?}"));
                        }
                    }
                    Expr::Index { object, index } => {
                        self.kind(frame, &object.node)?;
                        self.kind(frame, &index.node)?;
                    }
                    _ => return None,
                }
            }
            Stmt::Return(value) => {
                if let Some(value) = value {
                    self.kind(frame, &value.node)?;
                }
            }
            Stmt::If {
                condition,
                body,
                elseif_clauses,
                else_body,
            } => {
                self.kind(frame, &condition.node)?;
                self.check_block(frame, body)?;
                for (condition, body) in elseif_clauses {
                    self.kind(frame, &condition.node)?;
                    self.check_block(frame, body)?;
                }
                if let Some(body) = else_body {
                    self.check_block(frame, body)?;
                }
            }
            Stmt::While { condition, body } => {
                self.kind(frame, &condition.node)?;
                self.check_block(frame, body)?;
            }
            Stmt::ExprStmt(expression) => match &expression.node {
                Expr::Call { callee, args } => {
                    self.check_call(frame, &callee.node, args, true)?;
                }
                other => {
                    self.kind(frame, other)?;
                }
            },
        }
        Some(())
    }

    fn kind(&mut self, frame: &mut CheckFrame, e: &Expr) -> Option<Kind> {
        Some(match e {
            Expr::IntLit(_) | Expr::FloatLit(_) => Kind::Num,
            Expr::BoolLit(_) => Kind::Bool,
            Expr::StringLit(_) => Kind::Str,
            Expr::NoneLit => Kind::None,
            Expr::Ident(name) => {
                let key = name.0.to_ascii_lowercase();
                if let Some(kind) = frame.locals.get(&key) {
                    *kind
                } else if key == "self" {
                    frame.self_kind
                } else if is_state_variable(&name.0) {
                    Kind::Str
                } else {
                    let script = Arc::clone(&frame.script);
                    let Some(ty) = self.member_type(&script, &name.0) else {
                        let name = name.0.clone();
                        return self.fail(|| format!("unknown name {name}"));
                    };
                    self.kind_of_declared(&script, &ty)
                }
            }
            Expr::Cast { expr, target_type } => {
                let inner = self.kind(frame, &expr.node)?;
                match kind_of_type(&target_type.node, self.library) {
                    Kind::Object if inner == Kind::Quest => Kind::Quest,
                    Kind::Object if inner == Kind::Player => Kind::Player,
                    kind => kind,
                }
            }
            Expr::UnaryOp { op, operand } => {
                let inner = self.kind(frame, &operand.node)?;
                match op {
                    UnaryOp::Not => Kind::Bool,
                    UnaryOp::Neg => {
                        if inner != Kind::Num {
                            return None;
                        }
                        Kind::Num
                    }
                }
            }
            Expr::BinaryOp { left, op, right } => {
                let l = self.kind(frame, &left.node)?;
                let r = self.kind(frame, &right.node)?;
                match op {
                    BinaryOp::And | BinaryOp::Or => Kind::Bool,
                    BinaryOp::Eq | BinaryOp::Ne => Kind::Bool,
                    BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                        if !matches!(l, Kind::Num | Kind::Bool | Kind::Str) || !matches!(r, Kind::Num | Kind::Bool | Kind::Str) {
                            return None;
                        }
                        Kind::Bool
                    }
                    BinaryOp::StrCat => Kind::Str,
                    BinaryOp::Add if l == Kind::Str || r == Kind::Str => Kind::Str,
                    _ => {
                        if !matches!(l, Kind::Num | Kind::Bool) || !matches!(r, Kind::Num | Kind::Bool) {
                            return None;
                        }
                        Kind::Num
                    }
                }
            }
            Expr::Index { object, index } => {
                if self.kind(frame, &object.node)? != Kind::Array {
                    return None;
                }
                self.kind(frame, &index.node)?;
                // The element's declared type when the array's is known;
                // else an untracked value.
                match self.static_type(frame, e) {
                    Some(ty) => {
                        let script = Arc::clone(&frame.script);
                        self.kind_of_declared(&script, &ty)
                    }
                    None => Kind::Object,
                }
            }
            Expr::NewStruct { ty } => {
                let script = Arc::clone(&frame.script);
                match &ty.node {
                    Type::Object(name) if self.struct_declaration(&script, &name.0).is_some() => Kind::Struct,
                    _ => return self.fail(|| "new of an unknown struct".into()),
                }
            }
            Expr::New { size, .. } => {
                self.kind(frame, &size.node)?;
                Kind::Array
            }
            Expr::ArrayLit(items) => {
                for item in items {
                    self.kind(frame, &item.node)?;
                }
                Kind::Array
            }
            Expr::MemberAccess { object, member } => {
                let owner = self.kind(frame, &object.node)?;
                if member.node.0.eq_ignore_ascii_case("length") && owner == Kind::Array {
                    return Some(Kind::Num);
                }
                if owner == Kind::Struct {
                    let Some(ty) = self.static_type(frame, e) else {
                        let member = member.node.0.clone();
                        return self.fail(|| format!("member .{member} of an untyped struct"));
                    };
                    let script = Arc::clone(&frame.script);
                    return Some(self.kind_of_declared(&script, &ty));
                }
                if owner == Kind::Quest {
                    // Another quest script's member: its declared type when
                    // the quest's script is known (an `ActorValue Property`
                    // is no number), else a numeric variable, what the VM
                    // reads from an untyped quest.
                    let declared = self
                        .declared_script(frame, &object.node)
                        .and_then(|script| self.member_type(&script, &member.node.0));
                    match declared {
                        Some(ty) => {
                            let script = Arc::clone(&frame.script);
                            self.kind_of_declared(&script, &ty)
                        }
                        None => Kind::Num,
                    }
                } else if member.node.0.eq_ignore_ascii_case("value") {
                    Kind::Num
                } else if let Some(ty) = self.engine_constant_type(&member.node.0).filter(|_| owner != Kind::None) {
                    kind_of_type(&ty, self.library)
                } else if let Some(ty) = self.reference_member_type(frame, owner, &object.node, &member.node.0) {
                    kind_of_type(&ty, self.library)
                } else {
                    let member = member.node.0.clone();
                    return self.fail(|| format!("member .{member} of {owner:?}"));
                }
            }
            Expr::Call { callee, args } => self.check_call(frame, &callee.node, args, false)?,
            Expr::Is { .. } | Expr::ParentAccess => return self.fail(|| "Is / Parent".into()),
        })
    }

    /// A call's result kind, or `None` when the VM cannot run it.
    fn check_call(&mut self, frame: &mut CheckFrame, callee: &Expr, args: &[CallArg], statement: bool) -> Option<Kind> {
        let mut arg_kinds = Vec::with_capacity(args.len());
        for arg in args {
            let kind = self.kind(frame, &arg.value.node)?;
            // A literal argument is checked as written: the run sees it too;
            // so is an engine constant, which the run passes by name.
            let constant = constant_name(&arg.value.node)
                .filter(|name| !frame.locals.contains_key(&name.to_ascii_lowercase()))
                .filter(|name| self.engine_constant_type(name).is_some());
            arg_kinds.push(match (constant, literal_of(&arg.value.node)) {
                (Some(name), _) => Arg::Constant(name.to_owned()),
                (None, Some(literal)) => Arg::Literal(literal),
                (None, None) => Arg::Kind(kind),
            });
        }
        let (receiver, name) = match callee {
            Expr::Ident(name) => (None, name.0.to_ascii_lowercase()),
            Expr::MemberAccess { object, member } => (Some(&object.node), member.node.0.to_ascii_lowercase()),
            _ => return None,
        };
        // The frame's own script function.
        if receiver.is_none()
            || receiver.is_some_and(|r| matches!(r, Expr::Ident(id) if id.0.eq_ignore_ascii_case("self")))
        {
            let script = Arc::clone(&frame.script);
            if !self.versions(&script, &name).is_empty() {
                return self.check_call_into(&script, &name, statement, frame.self_kind);
            }
            if is_state_event(callee) {
                return Some(Kind::None);
            }
            if name == "getstate" {
                return Some(Kind::Str);
            }
            // An update registration: the handler it delivers to must run.
            if let Some(handler) = super::UpdateRequest::handler(&name) {
                let interval_ok = update_args_ok(&name, &arg_kinds);
                if !statement || !interval_ok {
                    return self.fail(|| format!("{name} as written"));
                }
                if !handler.is_empty() && !self.versions(&script, handler).is_empty() {
                    self.check_handler(&script, handler, frame.self_kind)?;
                }
                return Some(Kind::None);
            }
            if name == "gotostate" && statement {
                // The switch runs the state events, called for no value.
                for event in ["onendstate", "onbeginstate"] {
                    if !self.versions(&script, event).is_empty() {
                        self.check_call_into(&script, event, false, frame.self_kind)?;
                    }
                }
                return Some(Kind::None);
            }
        }
        let Some(receiver) = receiver else {
            return self.fail(|| format!("global {name}"));
        };
        // `ScriptName.Fn()`: a global function of a script the library has.
        if let Expr::Ident(script_name) = receiver {
            let key = script_name.0.to_ascii_lowercase();
            let is_value = key == "self"
                || frame.locals.contains_key(&key)
                || static_class(receiver).is_some()
                || {
                    let script = Arc::clone(&frame.script);
                    self.member_type(&script, &script_name.0).is_some()
                };
            if !is_value {
                if let Some(script) = self.library.and_then(|library| library.script(&script_name.0)) {
                    if !is_engine_class(&script) && !self.versions(&script, &name).is_empty() {
                        self.scripts.insert(key, Arc::clone(&script));
                        return self.check_call_into(&script, &name, statement, Kind::None);
                    }
                }
            }
        }
        if let Some(class) = static_class(receiver) {
            if class == "game" && name == "getplayer" {
                return Some(Kind::Player);
            }
            if class == "game"
                && name == "getformfromfile"
                && matches!(arg_kinds.as_slice(), [Arg::Literal(Expr::IntLit(_)) | Arg::Kind(Kind::Num), Arg::Literal(Expr::StringLit(_)) | Arg::Kind(Kind::Str)])
            {
                return Some(Kind::Object);
            }
            if class == "debug" && name.starts_with("trace") {
                return Some(Kind::None);
            }
            if class == "game"
                && name == "getform"
                && matches!(arg_kinds.as_slice(), [Arg::Literal(Expr::IntLit(_)) | Arg::Kind(Kind::Num)])
            {
                return Some(Kind::Object);
            }
            if class == "math" && MATH_NATIVES.contains(&name.as_str()) {
                let numeric = |arg: &Arg| matches!(arg, Arg::Literal(Expr::IntLit(_) | Expr::FloatLit(_)) | Arg::Kind(Kind::Num));
                let arity = if matches!(name.as_str(), "min" | "max" | "pow") { 2 } else { 1 };
                if arg_kinds.len() == arity && arg_kinds.iter().all(numeric) {
                    return Some(Kind::Num);
                }
                return self.fail(|| format!("Math.{name} as written"));
            }
            if class == "utility" && name == "getcurrentgametime" && arg_kinds.is_empty() {
                return Some(Kind::Num);
            }
            if class == "utility" && name == "wait" {
                if statement && matches!(arg_kinds.as_slice(), [Arg::Literal(_)] | [Arg::Kind(Kind::Num)]) {
                    self.waits = true;
                    return Some(Kind::None);
                }
                return self.fail(|| "Utility.Wait called for its value".into());
            }
            let result = self.check_native(receiver.clone(), Kind::None, &name, &arg_kinds, statement);
            if result.is_none() {
                return self.fail(|| format!("native {class}.{name}/{}", arg_kinds.len()));
            }
            return result;
        }
        let receiver_kind = self.kind(frame, receiver)?;
        if receiver_kind == Kind::Array {
            return self.check_array_call(&name, &arg_kinds, statement);
        }
        match (receiver_kind, name.as_str()) {
            (Kind::Alias, "getref" | "getreference" | "getactorref" | "getactorreference" | "getshipref" | "getshipreference") => {
                return Some(Kind::Object);
            }
            (_, "getowningquest") => return Some(Kind::Quest),
            (
                Kind::Object | Kind::Player,
                "getlinkedref" | "getbaseobject" | "getactorbase" | "getleveledactorbase" | "getcurrentlocation"
                | "getdialoguetarget",
            ) => {
                return Some(Kind::Object);
            }
            (Kind::Object | Kind::Player, "getnthlinkedref")
                if matches!(arg_kinds.as_slice(), [Arg::Literal(_)] | [Arg::Kind(Kind::Num)]) =>
            {
                return Some(Kind::Object);
            }
            (Kind::Alias, "getlocation") => return Some(Kind::Object),
            _ => {}
        }
        // Another quest's update registration: its handler must run.
        if receiver_kind == Kind::Quest {
            if let Some(handler) = super::UpdateRequest::handler(&name) {
                let interval_ok = update_args_ok(&name, &arg_kinds);
                if !statement || !interval_ok {
                    return self.fail(|| format!("{name} as written"));
                }
                if !handler.is_empty() {
                    let Some(script) = self.declared_script(frame, receiver) else {
                        return self.fail(|| format!("{name} on a quest of unknown script"));
                    };
                    if !self.versions(&script, handler).is_empty() {
                        self.check_handler(&script, handler, Kind::Quest)?;
                    }
                }
                return Some(Kind::None);
            }
        }
        // A function of a script on another reference, through a cast or a
        // typed variable: it runs as that reference's.
        if receiver_kind == Kind::Object {
            if let Some(script) = self.declared_script(frame, receiver) {
                if !is_engine_class(&script) && !self.versions(&script, &name).is_empty() {
                    return self.check_call_into(&script, &name, statement, Kind::Object);
                }
            }
        }
        // A quest script's function, through a cast or a typed property.
        if receiver_kind == Kind::Quest {
            if let Some(script) = self.quest_script_for(frame, receiver, &name) {
                return self.check_call_into(&script, &name, statement, Kind::Quest);
            }
        }
        let Some(placeholder) = spanned_placeholder(receiver_kind) else {
            return self.fail(|| format!("{name} on {receiver_kind:?}"));
        };
        let result = self.check_native(placeholder, receiver_kind, &name, &arg_kinds, statement);
        if result.is_none() {
            return self.fail(|| format!("native {name}/{} on {receiver_kind:?}", arg_kinds.len()));
        }
        result
    }

    /// The script a quest-typed receiver's `name` lives in: a cast's target
    /// type, or the declared type of a property / local.
    fn quest_script_for(&mut self, frame: &CheckFrame, receiver: &Expr, name: &str) -> Option<Arc<Script>> {
        let script = self.declared_script(frame, receiver)?;
        if self.versions(&script, name).is_empty() {
            return None;
        }
        Some(script)
    }

    /// The script a receiver is declared as — a cast's target type, or a
    /// property's / local's type — registered for the run.
    fn declared_script(&mut self, frame: &CheckFrame, receiver: &Expr) -> Option<Arc<Script>> {
        let type_name = match receiver {
            Expr::Cast { target_type, .. } => match &target_type.node {
                Type::Object(ty) => ty.0.clone(),
                _ => return None,
            },
            Expr::Ident(id) if id.0.eq_ignore_ascii_case("self") => frame.script.name.node.0.clone(),
            Expr::Ident(id) => match frame
                .types
                .get(&id.0.to_ascii_lowercase())
                .cloned()
                .or_else(|| self.member_type(&frame.script, &id.0))
            {
                Some(Type::Object(ty)) => ty.0.clone(),
                _ => return None,
            },
            _ => return None,
        };
        let script = self.library?.script(&type_name)?;
        self.scripts
            .insert(script.name.node.0.to_ascii_lowercase(), Arc::clone(&script));
        Some(script)
    }

    /// Whether the primitives take this native call, over placeholders.
    fn check_native(
        &mut self,
        receiver: Expr,
        _receiver_kind: Kind,
        name: &str,
        arg_kinds: &[Arg],
        statement: bool,
    ) -> Option<Kind> {
        let mut bindings = NativeBindings::default();
        let placeholder = |kind: Kind, index: usize, bindings: &mut NativeBindings| -> Option<Spanned<Expr>> {
            let name = format!("__vm{index}");
            Some(match kind {
                Kind::Num => spanned(Expr::IntLit(1)),
                Kind::Bool => spanned(Expr::BoolLit(false)),
                Kind::Str => spanned(Expr::StringLit("x".into())),
                Kind::None => spanned(Expr::NoneLit),
                Kind::Player => {
                    bindings.players.insert(name.clone());
                    ident(&name)
                }
                Kind::Quest => {
                    bindings.quests.insert(name.clone());
                    ident(&name)
                }
                Kind::Object | Kind::Alias => ident(&name),
                Kind::Array | Kind::Struct => return None,
            })
        };
        let receiver = match receiver {
            Expr::Ident(id) if id.0.starts_with("__vm_kind_") => {
                let kind = match id.0.as_str() {
                    "__vm_kind_quest" => Kind::Quest,
                    "__vm_kind_player" => Kind::Player,
                    _ => Kind::Object,
                };
                placeholder(kind, 0, &mut bindings)?
            }
            other => spanned(other),
        };
        let mut args = Vec::with_capacity(arg_kinds.len());
        for (index, arg) in arg_kinds.iter().enumerate() {
            args.push(CallArg {
                name: None,
                value: match arg {
                    Arg::Literal(literal) => spanned(literal.clone()),
                    Arg::Kind(kind) => placeholder(*kind, index + 1, &mut bindings)?,
                    Arg::Constant(name) => spanned(Expr::MemberAccess {
                        object: Box::new(receiver.clone()),
                        member: spanned(Identifier(name.clone())),
                    }),
                },
            });
        }
        let call = Expr::Call {
            callee: Box::new(spanned(Expr::MemberAccess {
                object: Box::new(receiver),
                member: spanned(Identifier(name.to_owned())),
            })),
            args,
        };
        if statement {
            if let Some(effect) = classify_native_effect(&call, &bindings) {
                return (!crate::translate::effects::is_latent(&effect)).then_some(Kind::None);
            }
        }
        if classify_native_value(&call, &bindings).is_some() {
            return Some(Kind::Num);
        }
        if classify_native_guard(&call, &bindings).is_some() {
            return Some(Kind::Bool);
        }
        None
    }
}

/// An update or timer call's arguments: numbers, at most as many as it
/// takes, and the interval when it starts one.
fn update_args_ok(name: &str, args: &[Arg]) -> bool {
    let numeric = |arg: &Arg| matches!(arg, Arg::Literal(Expr::IntLit(_) | Expr::FloatLit(_)) | Arg::Kind(Kind::Num));
    let starts = !super::UpdateRequest::handler(name).unwrap_or_default().is_empty();
    args.len() <= super::UpdateRequest::max_args(name) && (!starts || !args.is_empty()) && args.iter().all(numeric)
}

/// A native call's argument as the check sees it.
enum Arg {
    Literal(Expr),
    Kind(Kind),
    /// An engine constant, passed as the receiver's named member.
    Constant(String),
}

/// The name an argument reads an engine constant by, if it is written as
/// one: `X.Motion_Keyframed` or a bare `Motion_Keyframed`.
fn constant_name(e: &Expr) -> Option<&str> {
    match e {
        Expr::MemberAccess { member, .. } => Some(member.node.0.as_str()),
        Expr::Ident(name) if !name.0.eq_ignore_ascii_case("self") => Some(name.0.as_str()),
        _ => None,
    }
}

/// `e` as a literal (through casts and negation), if it is one.
fn literal_of(e: &Expr) -> Option<Expr> {
    match e {
        Expr::IntLit(_) | Expr::FloatLit(_) | Expr::BoolLit(_) | Expr::StringLit(_) | Expr::NoneLit => {
            Some(e.clone())
        }
        Expr::Cast { expr, target_type } => match (literal_of(&expr.node)?, &target_type.node) {
            (Expr::IntLit(i), Type::Float) => Some(Expr::FloatLit(i as f64)),
            (Expr::FloatLit(f), Type::Int) => Some(Expr::IntLit(f as i64)),
            (literal, Type::Int | Type::Float | Type::Bool | Type::String) => Some(literal),
            _ => None,
        },
        Expr::UnaryOp { op: UnaryOp::Neg, operand } => match literal_of(&operand.node)? {
            Expr::IntLit(i) => Some(Expr::IntLit(-i)),
            Expr::FloatLit(f) => Some(Expr::FloatLit(-f)),
            _ => None,
        },
        _ => None,
    }
}

/// A receiver placeholder the native check binds by kind.
fn spanned_placeholder(kind: Kind) -> Option<Expr> {
    let name = match kind {
        Kind::Quest => "__vm_kind_quest",
        Kind::Player => "__vm_kind_player",
        Kind::Object | Kind::Alias => "__vm_kind_object",
        _ => return None,
    };
    Some(Expr::Ident(Identifier(name.to_owned())))
}

#[cfg(test)]
mod tests;
