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
//! [`check_function`] decides ahead of time, against typed placeholders,
//! whether every statement and call of a fragment (and of the script
//! functions it reaches) is one this VM runs, so populate installs
//! `Effect::Interpret` only for fragments that run whole. A run that still
//! meets something it cannot do — a native that declines its actual values —
//! stops there and logs it.
//!
//! Not modelled: latent calls (`Utility.Wait` declines the fragment), object
//! script variables that outlive one run, structs, and calls into
//! non-quest scripts.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use byroredux_core::ecs::storage::EntityId;
use byroredux_core::ecs::world::World;
use byroredux_papyrus::ast::{
    AssignOp, BinaryOp, CallArg, Expr, Function, Identifier, Script, ScriptItem, Stmt, Type,
    UnaryOp,
};
use byroredux_papyrus::span::{Span, Spanned};
use byroredux_plugin::esm::records::script_instance::{
    PropertyValue, ScriptInstance, ScriptInstanceData, ScriptProperty,
};

use super::effects::{apply_effects, evaluate_guard, evaluate_value, DeferredFragmentEffects};
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
            Value::Alias { .. } | Value::Array(_) => true,
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
            (a, b) => match (a.number(), b.number()) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            },
        }
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
    /// The quest the script is attached to (`Self`).
    quest: QuestFormId,
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

/// `frame`'s state at `path` for a [`VmResume`].
fn frame_state(frame: &Frame, function: &str, path: Vec<VmStep>) -> Run<VmFrame> {
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
        quest: frame.quest.0,
        path,
        locals,
    })
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
        Value::Array(_) => return None,
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
}

/// Run `script`'s `function` as a fragment of the quest `context`, its
/// properties bound by `vmad` — from the start, or from where `resume` says a
/// `Utility.Wait` suspended it. Returns the quest-stage advances its natives
/// produced, and the wait and resume point when it suspends again.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_fragment(
    script: &str,
    function: &str,
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
        steps: 0,
        depth: 0,
        advances: Vec::new(),
    };
    let (top, inner) = match resume.and_then(|state| state.frames.split_first()) {
        Some((top, inner)) => (Some(top), inner),
        None => (None, &[][..]),
    };
    let mut frame = Frame {
        script: Arc::clone(&script),
        quest: context,
        vmad: vmad.cloned(),
        locals: top.map(restore_locals).unwrap_or_default(),
    };
    let run = vm
        .run_frame(&mut frame, function, top.map(|top| top.path.as_slice()), inner)
        .and_then(|flow| match flow {
            Flow::Suspend(seconds, path, inner) => {
                let mut frames = vec![frame_state(&frame, function, path)?];
                frames.extend(inner);
                Ok(Some((seconds, VmResume { frames })))
            }
            _ => Ok(None),
        });
    let suspended = match run {
        Ok(suspended) => suspended,
        Err(Stop(why)) => {
            log::warn!(
                "Papyrus VM: {}.{function} for quest {:08X} stopped: {why}",
                script.name.node.0,
                context.0
            );
            None
        }
    };
    (vm.advances, suspended)
}

fn find_function<'s>(script: &'s Script, name: &str) -> Option<&'s Function> {
    script.body.iter().find_map(|item| match &item.node {
        ScriptItem::Function(f) if f.name.node.0.eq_ignore_ascii_case(name) => Some(f),
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
        if self.steps > MAX_STEPS {
            return stop("step limit");
        }
        Ok(())
    }

    /// Run `function` in `frame` — from the start, or along `path`, with
    /// `inner` the suspended frames of the call the path ends in.
    fn run_frame(&mut self, frame: &mut Frame, function: &str, path: Option<&[VmStep]>, inner: &[VmFrame]) -> Run<Flow> {
        if self.depth >= MAX_DEPTH {
            return stop("call depth");
        }
        let script = Arc::clone(&frame.script);
        let Some(function) = find_function(&script, function) else {
            return stop(format!("no function {function}"));
        };
        self.depth += 1;
        let flow = self.exec_block(frame, &function.body, path, inner);
        self.depth -= 1;
        flow
    }

    /// `frame` with `name`'s parameters bound to `args`.
    fn enter(&mut self, mut frame: Frame, name: &str, args: Vec<Value>) -> Run<Frame> {
        let script = Arc::clone(&frame.script);
        let Some(function) = find_function(&script, name) else {
            return stop(format!("no function {name}"));
        };
        let mut args = args.into_iter();
        for param in &function.params {
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
    fn call_function(&mut self, frame: Frame, name: &str, args: Vec<Value>) -> Run<Value> {
        let mut frame = self.enter(frame, name, args)?;
        Ok(match self.run_frame(&mut frame, name, None, &[])? {
            Flow::Return(value) => value,
            Flow::Normal => Value::None,
            Flow::Suspend(..) => return stop("a wait in a function called for its value"),
        })
    }

    /// A called function's flow as its call statement's: a suspension adds
    /// the callee's frame to the chain.
    fn callee_flow(&mut self, flow: Flow, frame: &Frame, function: &str) -> Run<Flow> {
        Ok(match flow {
            Flow::Suspend(seconds, path, deeper) => {
                let mut frames = vec![frame_state(frame, function, path)?];
                frames.extend(deeper);
                Flow::Suspend(seconds, Vec::new(), frames)
            }
            _ => Flow::Normal,
        })
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
        let quest = QuestFormId(callee.quest);
        // The properties the call bound (`script_call_target`): the caller's
        // own for a call into its own script, else the quest's.
        let vmad = if quest == caller.quest && Arc::ptr_eq(&script, &caller.script) {
            caller.vmad.clone()
        } else {
            self.deferred.quest_scripts.get(&quest).cloned()
        };
        let mut frame = Frame {
            script,
            quest,
            vmad,
            locals: restore_locals(callee),
        };
        let flow = self.run_frame(&mut frame, &callee.function, Some(&callee.path), deeper)?;
        self.callee_flow(flow, &frame, &callee.function)
    }

    /// Run `body`, or — `resume` — continue it from the position a wait
    /// suspended it at.
    fn exec_block(
        &mut self,
        frame: &mut Frame,
        body: &[Spanned<Stmt>],
        resume: Option<&[VmStep]>,
        inner: &[VmFrame],
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
                Flow::Normal => {}
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
                let flow = self.exec_block(frame, block, Some(rest), inner)?;
                Ok(suspend_at(VmStep::Branch(*branch), flow))
            }
            Stmt::While { condition, body } => {
                match self.exec_block(frame, body, Some(rest), inner)? {
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
            match self.exec_block(frame, body, None, &[])? {
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
                    let flow = self.exec_block(frame, body, None, &[])?;
                    return Ok(suspend_at(VmStep::Branch(0), flow));
                }
                for (k, (condition, body)) in elseif_clauses.iter().enumerate() {
                    if self.eval(frame, &condition.node)?.truthy() {
                        let flow = self.exec_block(frame, body, None, &[])?;
                        return Ok(suspend_at(VmStep::Branch(k as u32 + 1), flow));
                    }
                }
                if let Some(body) = else_body {
                    let flow = self.exec_block(frame, body, None, &[])?;
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
                // A script variable or property of the frame's quest script.
                self.write_quest_variable(frame.quest, &name.0, &value)
            }
            Expr::MemberAccess { object, member } => {
                let owner = self.eval(frame, &object.node)?;
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
        let object = match value {
            Value::None => crate::ScriptObject::None,
            Value::Form(form) => crate::ScriptObject::Form(*form),
            Value::Alias { quest, alias } => crate::ScriptObject::Alias {
                quest: *quest,
                alias: *alias,
            },
            other => return stop(format!("a {} write to script variable {name}", other.text())),
        };
        self.deferred.quest_objects.push((quest, key, object));
        Ok(())
    }

    fn read_quest_object(&self, quest: QuestFormId, name: &str) -> Value {
        self.written_quest_object(quest, name).unwrap_or(Value::None)
    }

    /// An object a script wrote to `quest`'s variable `name`, if any.
    fn written_quest_object(&self, quest: QuestFormId, name: &str) -> Option<Value> {
        let key = crate::QuestScriptVariables::property_key(name);
        let pending = self
            .deferred
            .quest_objects
            .iter()
            .rev()
            .find(|(q, k, _)| *q == quest && *k == key)
            .map(|(_, _, value)| *value);
        let stored = pending.or_else(|| {
            self.world
                .try_resource::<crate::QuestScriptObjects>()
                .and_then(|objects| objects.get(quest, key))
        });
        Some(match stored? {
            crate::ScriptObject::Form(form) => Value::Form(form),
            crate::ScriptObject::Alias { quest, alias } => Value::Alias { quest, alias },
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
            return Ok(Value::Form(frame.quest.0));
        }
        if let Some(value) = Self::property(frame.vmad.as_ref(), name) {
            // A numeric property a script wrote reads the written value.
            if value.number().is_some() {
                if let Some(written) = self.read_quest_variable(frame.quest, name) {
                    return Ok(if value.is_float() {
                        Value::Float(written)
                    } else {
                        Value::Int(written as i32)
                    });
                }
            }
            // An object property a script re-pointed reads the new object.
            if let Some(written) = self.written_quest_object(frame.quest, name) {
                return Ok(written);
            }
            return Ok(value);
        }
        match member_type_in(&frame.script, name, &self.deferred.vm_scripts) {
            Some(ty @ (Type::Int | Type::Float | Type::Bool)) => {
                let value = self.read_quest_variable(frame.quest, name).unwrap_or(0.0);
                Ok(match ty {
                    Type::Int => Value::Int(value as i32),
                    Type::Bool => Value::Bool(value != 0.0),
                    _ => Value::Float(value),
                })
            }
            Some(Type::Object(_) | Type::Var) => Ok(self.read_quest_object(frame.quest, name)),
            Some(_) => stop(format!("script variable {name} of an unsupported type")),
            None => stop(format!("unknown name {name}")),
        }
    }

    /// `quest.Variable` — another quest script's property or variable.
    fn read_member(&mut self, owner: &Value, name: &str) -> Run<Value> {
        let Value::Form(form) = owner else {
            return stop(format!("member {name} of {}", owner.text()));
        };
        let quest = QuestFormId(*form);
        let vmad = self.deferred.quest_scripts.get(&quest).cloned();
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

    /// The quest script that defines `function` for the quest `quest`, if any.
    fn quest_script_with(&self, quest: QuestFormId, function: &str) -> Option<Arc<Script>> {
        let vmad = self.deferred.quest_scripts.get(&quest)?;
        vmad.scripts.iter().find_map(|attached| {
            let script = self.deferred.vm_scripts.get(&attached.name.to_ascii_lowercase())?;
            find_function(script, function).map(|_| Arc::clone(script))
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
        let Some(callee_frame) = self.script_call_target(frame, callee)? else {
            return Ok(None);
        };
        let values = self.eval_args(frame, args)?;
        self.call_function(callee_frame, &callee_name(callee), values).map(Some)
    }

    /// The fresh frame a call to a script function runs in, when `callee`
    /// names one.
    fn script_call_target(&mut self, frame: &mut Frame, callee: &Expr) -> Run<Option<Frame>> {
        let (receiver, name) = match callee {
            Expr::Ident(name) => (None, name.0.as_str()),
            Expr::MemberAccess { object, member } => (Some(&object.node), member.node.0.as_str()),
            _ => return Ok(None),
        };
        let target = match receiver {
            None => Some((Arc::clone(&frame.script), frame.quest, frame.vmad.clone())),
            Some(object) if static_class(object).is_some() => None,
            Some(object) => match self.eval(frame, object)? {
                Value::Form(form) if form == frame.quest.0 && find_function(&frame.script, name).is_some() => {
                    Some((Arc::clone(&frame.script), frame.quest, frame.vmad.clone()))
                }
                Value::Form(form) if self.is_quest(form) => self
                    .quest_script_with(QuestFormId(form), name)
                    .map(|script| {
                        let vmad = self.deferred.quest_scripts.get(&QuestFormId(form)).cloned();
                        (script, QuestFormId(form), vmad)
                    }),
                _ => None,
            },
        };
        let Some((script, quest, vmad)) = target else {
            return Ok(None);
        };
        if find_function(&script, name).is_none() {
            return Ok(None);
        }
        Ok(Some(Frame {
            script,
            quest,
            vmad,
            locals: HashMap::new(),
        }))
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
            (Value::Form(_), "getowningquest") => Some(Value::Form(frame.quest.0)),
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
        let callee = match callee {
            Expr::MemberAccess { object, member } => {
                let receiver = if static_class(&object.node).is_some() {
                    (**object).clone()
                } else {
                    let value = self.eval(frame, &object.node)?;
                    if matches!(value, Value::None) {
                        return stop(format!("{} on None", member.node.0));
                    }
                    synth.bind(self, &value)?
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
            let value = self.eval(frame, &arg.value.node)?;
            bound.push(CallArg {
                name: None,
                value: synth.bind(self, &value)?,
            });
        }
        let call = Expr::Call {
            callee: Box::new(spanned(callee)),
            args: bound,
        };
        let (bindings, vmad) = synth.vmad();
        Ok((call, bindings, vmad))
    }

    fn call_value(&mut self, frame: &mut Frame, callee: &Expr, args: &[CallArg]) -> Run<Value> {
        if let Some(value) = self.object_read(frame, callee, args)? {
            return Ok(value);
        }
        if let Some(value) = self.script_call(frame, callee, args)? {
            return Ok(value);
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
                return Ok(Flow::Suspend(seconds.max(0.0), Vec::new(), Vec::new()));
            }
        }
        if self.object_read(frame, &callee.node, args)?.is_some() {
            return Ok(Flow::Normal);
        }
        // A script function called as a statement may wait.
        if let Some(callee_frame) = self.script_call_target(frame, &callee.node)? {
            let name = callee_name(&callee.node);
            let values = self.eval_args(frame, args)?;
            let mut callee_frame = self.enter(callee_frame, &name, values)?;
            let flow = self.run_frame(&mut callee_frame, &name, None, &[])?;
            return self.callee_flow(flow, &callee_frame, &name);
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
    let float = l.is_float() || r.is_float();
    Ok(match (op, float) {
        (BinaryOp::Add, true) => Value::Float(a + b),
        (BinaryOp::Sub, true) => Value::Float(a - b),
        (BinaryOp::Mul, true) => Value::Float(a * b),
        (BinaryOp::Div, true) => Value::Float(a / b),
        (BinaryOp::Add, false) => Value::Int((a as i32).wrapping_add(b as i32)),
        (BinaryOp::Sub, false) => Value::Int((a as i32).wrapping_sub(b as i32)),
        (BinaryOp::Mul, false) => Value::Int((a as i32).wrapping_mul(b as i32)),
        (BinaryOp::Div, false) => {
            if b as i32 == 0 {
                return stop("integer division by zero");
            }
            Value::Int((a as i32) / (b as i32))
        }
        (BinaryOp::Mod, _) => {
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
        .insert(script.name.node.0.to_ascii_lowercase(), Arc::clone(&script));
    match checker.check_call_into(&script, function, true) {
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

impl Checker<'_> {
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
                return None;
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

    /// `statement`: whether the call is a statement — a function that waits
    /// may not be called for its value.
    fn check_call_into(&mut self, script: &Arc<Script>, function: &str, statement: bool) -> Option<Kind> {
        let key = (script.name.node.0.to_ascii_lowercase(), function.to_ascii_lowercase());
        let function_item = find_function(script, function)?;
        let returns = function_item
            .return_type
            .as_ref()
            .map_or(Kind::None, |ty| kind_of_type(&ty.node, self.library));
        if !self.visiting.insert(key.clone()) {
            // Checked already, or recursion: assume the declared return.
            return self.call_waits(self.waiting.contains(&key), statement).map(|_| returns);
        }
        let mut frame = CheckFrame {
            script: Arc::clone(script),
            locals: HashMap::new(),
            types: HashMap::new(),
        };
        for param in &function_item.params {
            let key = param.name.node.0.to_ascii_lowercase();
            frame.locals.insert(key.clone(), kind_of_type(&param.ty.node, self.library));
            frame.types.insert(key, param.ty.node.clone());
        }
        let outer = std::mem::replace(&mut self.waits, false);
        function_item
            .body
            .iter()
            .try_for_each(|statement| self.check_stmt(&mut frame, &statement.node))?;
        let waits = std::mem::replace(&mut self.waits, outer);
        if waits {
            self.waiting.insert(key);
        }
        self.call_waits(waits, statement).map(|_| returns)
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
                frame
                    .locals
                    .insert(key.clone(), kind_of_type(&variable.ty.node, self.library));
                frame.types.insert(key, variable.ty.node.clone());
            }
            Stmt::Assign { target, value, .. } => {
                let value_kind = self.kind(frame, &value.node)?;
                match &target.node {
                    Expr::Ident(name) => {
                        let key = name.0.to_ascii_lowercase();
                        if !frame.locals.contains_key(&key)
                            && matches!(value_kind, Kind::Str | Kind::Array)
                        {
                            return self.fail(|| format!("{value_kind:?} write to a script variable"));
                        }
                    }
                    Expr::MemberAccess { object, member } => {
                        let owner = self.kind(frame, &object.node)?;
                        if owner == Kind::Object && member.node.0.eq_ignore_ascii_case("value") && value_kind == Kind::Num {
                            // `Global.Value = n`.
                            let (callee, args) = set_value_call(object, value);
                            self.check_call(frame, &callee.node, &args, true)?;
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
                    Kind::Quest
                } else {
                    let script = Arc::clone(&frame.script);
                    let Some(ty) = self.member_type(&script, &name.0) else {
                        let name = name.0.clone();
                        return self.fail(|| format!("unknown name {name}"));
                    };
                    kind_of_type(&ty, self.library)
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
                // Element kinds are not tracked; reads of them stay values.
                Kind::Object
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
                if owner == Kind::Quest {
                    // Another quest script's numeric variable; its type is
                    // not known here, numbers are what the VM reads.
                    Kind::Num
                } else if member.node.0.eq_ignore_ascii_case("value") {
                    Kind::Num
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
            // A literal argument is checked as written: the run sees it too.
            arg_kinds.push(match literal_of(&arg.value.node) {
                Some(literal) => Arg::Literal(literal),
                None => Arg::Kind(kind),
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
            if find_function(&frame.script, &name).is_some() {
                let script = Arc::clone(&frame.script);
                return self.check_call_into(&script, &name, statement);
            }
        }
        let Some(receiver) = receiver else {
            return self.fail(|| format!("global {name}"));
        };
        if let Some(class) = static_class(receiver) {
            if class == "game" && name == "getplayer" {
                return Some(Kind::Player);
            }
            if class == "debug" && name.starts_with("trace") {
                return Some(Kind::None);
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
            (Kind::Alias, "getlocation") => return Some(Kind::Object),
            _ => {}
        }
        // A quest script's function, through a cast or a typed property.
        if receiver_kind == Kind::Quest {
            if let Some(script) = self.quest_script_for(frame, receiver, &name) {
                return self.check_call_into(&script, &name, statement);
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
        find_function(&script, name)?;
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
                Kind::Array => return None,
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

/// A native call's argument as the check sees it.
enum Arg {
    Literal(Expr),
    Kind(Kind),
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
