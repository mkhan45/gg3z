use std::hash::Hash;

use super::arena::{Id, Arena, Interner};

pub type TermId = Id<Term>;
pub type PropId = Id<Prop>;
pub type VarId = Id<Var>;
pub type SymbolId = Id<String>;
pub type RelId = Id<RelInfo>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Var {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Var(VarId),
    Atom(SymbolId),
    Int(i32),
    Float(f32),
    App { sym: SymbolId, args: Vec<TermId> },
}

impl Term {
    pub fn to_z3_int(
        &self,
        var_cache: &mut std::collections::HashMap<VarId, z3::ast::Int>,
    ) -> Option<z3::ast::Int> {
        match self {
            Term::Int(i) => Some(z3::ast::Int::from_i64((*i).into())),
            Term::Var(v) => {
                let z3_var = var_cache
                    .entry(*v)
                    .or_insert_with(|| z3::ast::Int::new_const(format!("v{}", v.index())));
                Some(z3_var.clone())
            }
            _ => None,
        }
    }

    pub fn to_z3_real(
        &self,
        var_cache: &mut std::collections::HashMap<VarId, z3::ast::Real>,
    ) -> Option<z3::ast::Real> {
        match self {
            Term::Float(f) => {
                let (num, den) = float_to_rational(*f);
                Some(z3::ast::Real::from_rational(num, den))
            }
            Term::Int(i) => Some(z3::ast::Real::from_rational((*i).into(), 1)),
            Term::Var(v) => {
                let z3_var = var_cache
                    .entry(*v)
                    .or_insert_with(|| z3::ast::Real::new_const(format!("r{}", v.index())));
                Some(z3_var.clone())
            }
            _ => None,
        }
    }
}

fn float_to_rational(f: f32) -> (i64, i64) {
    const PRECISION: i64 = 1_000_000;
    let num = (f * PRECISION as f32).round() as i64;
    fn gcd(a: i64, b: i64) -> i64 {
        if b == 0 { a.abs() } else { gcd(b, a % b) }
    }
    let g = gcd(num, PRECISION);
    (num / g, PRECISION / g)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Prop {
    True,
    False,
    Eq(TermId, TermId),
    And(PropId, PropId),
    Or(PropId, PropId),
    Not(PropId),
    Cond(PropId, PropId, PropId),
    /// User-defined relation application (resolved via back-chaining)
    App { rel: RelId, args: Vec<TermId> },
    /// SMT constraint (deferred to Z3 solver)
    Constraint { kind: ConstraintKind, args: Vec<TermId> },
}

/// Constraint kinds for SMT solving. These represent arithmetic constraints
/// that are deferred to Z3 rather than resolved through back-chaining.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstraintKind {
    // Integer constraints
    IntEq,
    IntNeq,
    IntLt,
    IntLe,
    IntGt,
    IntGe,
    IntAdd,
    IntSub,
    IntMul,
    IntDiv,
    // Real constraints
    RealEq,
    RealNeq,
    RealLt,
    RealLe,
    RealGt,
    RealGe,
    RealAdd,
    RealSub,
    RealMul,
    RealDiv,
}

impl ConstraintKind {
    /// Returns the expected arity for this constraint kind.
    pub fn arity(self) -> usize {
        match self {
            // Binary comparisons
            ConstraintKind::IntEq | ConstraintKind::IntNeq |
            ConstraintKind::IntLt | ConstraintKind::IntLe |
            ConstraintKind::IntGt | ConstraintKind::IntGe |
            ConstraintKind::RealEq | ConstraintKind::RealNeq |
            ConstraintKind::RealLt | ConstraintKind::RealLe |
            ConstraintKind::RealGt | ConstraintKind::RealGe => 2,
            // Ternary arithmetic
            ConstraintKind::IntAdd | ConstraintKind::IntSub |
            ConstraintKind::IntMul | ConstraintKind::IntDiv |
            ConstraintKind::RealAdd | ConstraintKind::RealSub |
            ConstraintKind::RealMul | ConstraintKind::RealDiv => 3,
        }
    }

    /// Parse a constraint kind from a relation name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "int_eq" => Some(ConstraintKind::IntEq),
            "int_neq" => Some(ConstraintKind::IntNeq),
            "int_lt" => Some(ConstraintKind::IntLt),
            "int_le" => Some(ConstraintKind::IntLe),
            "int_gt" => Some(ConstraintKind::IntGt),
            "int_ge" => Some(ConstraintKind::IntGe),
            "int_add" => Some(ConstraintKind::IntAdd),
            "int_sub" => Some(ConstraintKind::IntSub),
            "int_mul" => Some(ConstraintKind::IntMul),
            "int_div" => Some(ConstraintKind::IntDiv),
            "real_eq" => Some(ConstraintKind::RealEq),
            "real_neq" => Some(ConstraintKind::RealNeq),
            "real_lt" => Some(ConstraintKind::RealLt),
            "real_le" => Some(ConstraintKind::RealLe),
            "real_gt" => Some(ConstraintKind::RealGt),
            "real_ge" => Some(ConstraintKind::RealGe),
            "real_add" => Some(ConstraintKind::RealAdd),
            "real_sub" => Some(ConstraintKind::RealSub),
            "real_mul" => Some(ConstraintKind::RealMul),
            "real_div" => Some(ConstraintKind::RealDiv),
            _ => None,
        }
    }

    /// Get the name of this constraint kind (for debugging/display).
    pub fn name(self) -> &'static str {
        match self {
            ConstraintKind::IntEq => "int_eq",
            ConstraintKind::IntNeq => "int_neq",
            ConstraintKind::IntLt => "int_lt",
            ConstraintKind::IntLe => "int_le",
            ConstraintKind::IntGt => "int_gt",
            ConstraintKind::IntGe => "int_ge",
            ConstraintKind::IntAdd => "int_add",
            ConstraintKind::IntSub => "int_sub",
            ConstraintKind::IntMul => "int_mul",
            ConstraintKind::IntDiv => "int_div",
            ConstraintKind::RealEq => "real_eq",
            ConstraintKind::RealNeq => "real_neq",
            ConstraintKind::RealLt => "real_lt",
            ConstraintKind::RealLe => "real_le",
            ConstraintKind::RealGt => "real_gt",
            ConstraintKind::RealGe => "real_ge",
            ConstraintKind::RealAdd => "real_add",
            ConstraintKind::RealSub => "real_sub",
            ConstraintKind::RealMul => "real_mul",
            ConstraintKind::RealDiv => "real_div",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelInfo {
    pub name: String,
    pub arity: usize,
}

#[derive(Debug, Clone)]
pub struct Clause {
    pub name: String,
    pub head_rel: RelId,
    pub head_args: Vec<TermId>,
    pub body: PropId,
}

#[derive(Debug, Clone)]
pub struct DrawDirective {
    pub condition: PropId,
    pub draws: Vec<TermId>,
}

#[derive(Debug, Clone)]
pub struct Stage {
    pub name: String,
    pub rules: Vec<Clause>,
    pub state_constraints: Vec<PropId>,
    pub next_var_map: std::collections::HashMap<String, TermId>,
    pub draw_directives: Vec<DrawDirective>,
}

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub terms: Arena<Term>,
    pub props: Arena<Prop>,
    pub vars: Arena<Var>,
    pub symbols: Interner<String>,
    pub rels: Arena<RelInfo>,
    pub state_vars: Vec<String>,
    pub state_var_term_ids: std::collections::HashMap<String, TermId>,
    pub facts: Vec<PropId>,
    pub global_rules: Vec<Clause>,
    pub stages: Vec<Stage>,
}
