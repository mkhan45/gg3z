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
    App { rel: RelId, args: Vec<TermId> },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RelKind {
    User,
    SMTInt,
    SMTReal,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelInfo {
    pub name: String,
    pub arity: usize,
    pub kind: RelKind,
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
