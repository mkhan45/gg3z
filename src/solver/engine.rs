use std::collections::VecDeque;

#[cfg(feature = "profile")]
use std::cell::RefCell;

use im::{HashMap, Vector};

use crate::solver::ir::{Clause, ConstraintKind, Program, Prop, PropId, RelId, Term, TermId, Var, VarId};
use crate::solver::arena::Arena;

#[cfg(feature = "profile")]
thread_local! {
    static PROFILE_STATS: RefCell<ProfileStats> = RefCell::new(ProfileStats::new());
}

#[cfg(feature = "profile")]
#[derive(Debug, Default)]
struct ProfileStats {
    walk_calls: usize,
    walk_chains: usize,
    unify_calls: usize,
    unify_failures: usize,
}

#[cfg(feature = "profile")]
impl ProfileStats {
    fn new() -> Self {
        Self::default()
    }

    fn record_walk(&mut self, depth: usize) {
        self.walk_calls += 1;
        if depth > 1 {
            self.walk_chains += 1;
        }
    }

    fn record_unify(&mut self, success: bool) {
        self.unify_calls += 1;
        if !success {
            self.unify_failures += 1;
        }
    }
}

macro_rules! record_profile {
    ($stats:ident => $body:expr) => {
        #[cfg(feature = "profile")]
        {
            PROFILE_STATS.with(|_stats| {
                let mut $stats = _stats.borrow_mut();
                $body;
            });
        }
    };
}

#[derive(Clone, Default)]
pub struct Subst {
    map: HashMap<VarId, TermId>,
}

impl Subst {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn walk(&self, t: TermId, terms: &Arena<Term>) -> TermId {
        self.walk_impl(t, terms, 0)
    }

    fn walk_impl(&self, t: TermId, terms: &Arena<Term>, depth: usize) -> TermId {
        match terms.get(t) {
            Term::Var(v) => {
                if let Some(&t2) = self.map.get(v) {
                    let result = self.walk_impl(t2, terms, depth + 1);
                    record_profile!(stats => stats.record_walk(depth + 1));
                    result
                } else {
                    record_profile!(stats => stats.record_walk(depth));
                    t
                }
            }
            _ => {
                record_profile!(stats => stats.record_walk(depth));
                t
            }
        }
    }

    pub fn extend(&self, v: VarId, t: TermId) -> Self {
        Self {
            map: self.map.update(v, t),
        }
    }

    pub fn get(&self, v: VarId) -> Option<TermId> {
        self.map.get(&v).copied()
    }

    pub fn unify(&self, t1: TermId, t2: TermId, terms: &Arena<Term>) -> Option<Self> {
        let t1 = self.walk(t1, terms);
        let t2 = self.walk(t2, terms);

        if t1 == t2 {
            record_profile!(stats => stats.record_unify(true));
            return Some(self.clone());
        }

        let term1 = terms.get(t1);
        let term2 = terms.get(t2);

        let result = match (term1, term2) {
            (Term::Var(v1), _) => Some(self.extend(*v1, t2)),
            (_, Term::Var(v2)) => Some(self.extend(*v2, t1)),
            (Term::Atom(s1), Term::Atom(s2)) if s1 == s2 => Some(self.clone()),
            (Term::Int(i1), Term::Int(i2)) if i1 == i2 => Some(self.clone()),
            (Term::Float(f1), Term::Float(f2)) if f1 == f2 => Some(self.clone()),
            (Term::App { sym: s1, args: a1 }, Term::App { sym: s2, args: a2 }) if s1 == s2 => {
                self.unify_args(a1, a2, terms)
            }
            _ => None,
        };
        record_profile!(stats => stats.record_unify(result.is_some()));
        result
    }

    pub fn unify_args(
        &self,
        args1: &[TermId],
        args2: &[TermId],
        terms: &Arena<Term>,
    ) -> Option<Self> {
        if args1.len() != args2.len() {
            return None;
        }
        let mut subst = self.clone();
        for (&a1, &a2) in args1.iter().zip(args2.iter()) {
            subst = subst.unify(a1, a2, terms)?;
        }
        Some(subst)
    }
}

#[derive(Debug, Clone)]
pub struct Constraint {
    pub kind: ConstraintKind,
    pub args: Vec<TermId>,
}

#[derive(Clone, Default)]
pub struct ConstraintStore {
    constraints: Vector<Constraint>,
}

impl ConstraintStore {
    pub fn new() -> Self {
        Self {
            constraints: Vector::new(),
        }
    }

    pub fn add(&self, c: Constraint) -> Self {
        Self {
            constraints: self.constraints.clone() + Vector::unit(c),
        }
    }

    pub fn is_ground_constraint(c: &Constraint, subst: &Subst, program: &Program) -> bool {
        c.args.iter().all(|t| {
            let walked = subst.walk(*t, &program.terms);
            !matches!(program.terms.get(walked), Term::Var(_))
        })
    }

    /// Partition constraints into ground (fully determined) and non-ground (contains variables).
    /// Used during SLD resolution for eager constraint propagation.
    fn partition_ground_constraints(&self, subst: &Subst, program: &Program) -> (Vec<Constraint>, Vec<Constraint>) {
        self.iter()
            .cloned()
            .partition(|c| Self::is_ground_constraint(c, subst, program))
    }

    /// SLD resolution constraint propagation: solve ground constraints, defer non-ground.
    /// Returns refined substitution and remaining (non-ground) constraints.
    /// Used during search to eagerly prune infeasible branches.
    pub fn propagate_ground(&self, subst: &Subst, program: &mut Program, z3_solver: &z3::Solver) -> Option<(Subst, ConstraintStore)> {
        let (ground, non_ground) = self.partition_ground_constraints(subst, program);
        
        let ground_store = ConstraintStore {
            constraints: ground.into_iter().collect(),
        };
        
        let new_subst = if ground_store.is_empty() {
            subst.clone()
        } else {
            ground_store.solve_constraints(subst, program, z3_solver)?
        };
        
        let remaining = non_ground.into_iter()
            .fold(ConstraintStore::new(), |acc, c| acc.add(c));
        
        Some((new_subst, remaining))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Constraint> {
        self.constraints.iter()
    }

    pub fn len(&self) -> usize {
        self.constraints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }

    /// Final constraint validation: solve all constraints (ground and non-ground).
    /// Used after proof search completes to verify the solution satisfies all constraints.
    /// Critical for negation: ensures phantom proofs with unsatisfiable constraints are rejected.
    pub fn solve_constraints(&self, subst: &Subst, program: &mut Program, z3_solver: &z3::Solver) -> Option<Subst> {
        if self.is_empty() {
            return Some(subst.clone());
        }

        z3_solver.reset();

        let mut int_vars: std::collections::HashMap<VarId, z3::ast::Int> =
            std::collections::HashMap::new();
        let mut real_vars: std::collections::HashMap<VarId, z3::ast::Real> =
            std::collections::HashMap::new();

        for constraint in self.iter() {
            let assertion = Self::constraint_to_z3(
                constraint, subst, &program.terms, &mut int_vars, &mut real_vars
            )?;
            z3_solver.assert(&assertion);
        }

        match z3_solver.check() {
            z3::SatResult::Sat => {
                let model = z3_solver.get_model()?;
                Some(Self::extract_bindings(&model, &int_vars, &real_vars, subst, program))
            }
            _ => None,
        }
    }

    fn constraint_to_z3(
        constraint: &Constraint,
        subst: &Subst,
        terms: &Arena<Term>,
        int_vars: &mut std::collections::HashMap<VarId, z3::ast::Int>,
        real_vars: &mut std::collections::HashMap<VarId, z3::ast::Real>,
    ) -> Option<z3::ast::Bool> {
        let mut to_int = |t: TermId| -> Option<z3::ast::Int> {
            let walked = subst.walk(t, terms);
            terms.get(walked).to_z3_int(int_vars)
        };

        let mut to_real = |t: TermId| -> Option<z3::ast::Real> {
            let walked = subst.walk(t, terms);
            terms.get(walked).to_z3_real(real_vars)
        };

        let args = &constraint.args;
        match constraint.kind {
            ConstraintKind::IntAdd => {
                Some(z3::ast::Int::add(&[&to_int(args[0])?, &to_int(args[1])?]).eq(&to_int(args[2])?))
            }
            ConstraintKind::IntSub => {
                Some(z3::ast::Int::sub(&[&to_int(args[0])?, &to_int(args[1])?]).eq(&to_int(args[2])?))
            }
            ConstraintKind::IntMul => {
                Some(z3::ast::Int::mul(&[&to_int(args[0])?, &to_int(args[1])?]).eq(&to_int(args[2])?))
            }
            ConstraintKind::IntDiv => {
                Some(to_int(args[0])?.div(&to_int(args[1])?).eq(&to_int(args[2])?))
            }
            ConstraintKind::IntEq => Some(to_int(args[0])?.eq(&to_int(args[1])?)),
            ConstraintKind::IntNeq => Some(to_int(args[0])?.eq(&to_int(args[1])?).not()),
            ConstraintKind::IntLt => Some(to_int(args[0])?.lt(&to_int(args[1])?)),
            ConstraintKind::IntLe => Some(to_int(args[0])?.le(&to_int(args[1])?)),
            ConstraintKind::IntGt => Some(to_int(args[0])?.gt(&to_int(args[1])?)),
            ConstraintKind::IntGe => Some(to_int(args[0])?.ge(&to_int(args[1])?)),
            ConstraintKind::RealAdd => {
                Some(z3::ast::Real::add(&[&to_real(args[0])?, &to_real(args[1])?]).eq(&to_real(args[2])?))
            }
            ConstraintKind::RealSub => {
                Some(z3::ast::Real::sub(&[&to_real(args[0])?, &to_real(args[1])?]).eq(&to_real(args[2])?))
            }
            ConstraintKind::RealMul => {
                Some(z3::ast::Real::mul(&[&to_real(args[0])?, &to_real(args[1])?]).eq(&to_real(args[2])?))
            }
            ConstraintKind::RealDiv => {
                Some(to_real(args[0])?.div(&to_real(args[1])?).eq(&to_real(args[2])?))
            }
            ConstraintKind::RealEq => Some(to_real(args[0])?.eq(&to_real(args[1])?)),
            ConstraintKind::RealNeq => Some(to_real(args[0])?.eq(&to_real(args[1])?).not()),
            ConstraintKind::RealLt => Some(to_real(args[0])?.lt(&to_real(args[1])?)),
            ConstraintKind::RealLe => Some(to_real(args[0])?.le(&to_real(args[1])?)),
            ConstraintKind::RealGt => Some(to_real(args[0])?.gt(&to_real(args[1])?)),
            ConstraintKind::RealGe => Some(to_real(args[0])?.ge(&to_real(args[1])?)),
        }
    }

    fn extract_bindings(
        model: &z3::Model,
        int_vars: &std::collections::HashMap<VarId, z3::ast::Int>,
        real_vars: &std::collections::HashMap<VarId, z3::ast::Real>,
        subst: &Subst,
        program: &mut Program,
    ) -> Subst {
        let mut new_subst = subst.clone();

        for (var_id, z3_var) in int_vars {
            if let Some(val) = model.eval(z3_var, true)
                && let Some(i) = val.as_i64()
            {
                let term_id = program.terms.alloc(Term::Int(i as i32));
                new_subst = new_subst.extend(*var_id, term_id);
            }
        }

        for (var_id, z3_var) in real_vars {
            if let Some(val) = model.eval(z3_var, true)
                && let Some((num, den)) = val.as_rational()
            {
                let f = num as f32 / den as f32;
                let term_id = program.terms.alloc(Term::Float(f));
                new_subst = new_subst.extend(*var_id, term_id);
            }
        }

        new_subst
    }
}

#[derive(Clone)]
pub struct State {
    pub subst: Subst,
    pub constraints: ConstraintStore,
    pub goals: Vector<PropId>,
}

impl State {
    pub fn new(initial_goal: PropId) -> Self {
        Self {
            subst: Subst::new(),
            constraints: ConstraintStore::new(),
            goals: Vector::unit(initial_goal),
        }
    }

    pub fn empty() -> Self {
        Self {
            subst: Subst::new(),
            constraints: ConstraintStore::new(),
            goals: Vector::new(),
        }
    }

    pub fn with_subst(&self, subst: Subst) -> Self {
        Self {
            subst,
            constraints: self.constraints.clone(),
            goals: self.goals.clone(),
        }
    }

    pub fn with_constraint(&self, c: Constraint) -> Self {
        Self {
            subst: self.subst.clone(),
            constraints: self.constraints.add(c),
            goals: self.goals.clone(),
        }
    }

    pub fn with_goal(&self, goal: PropId) -> Self {
        Self {
            subst: self.subst.clone(),
            constraints: self.constraints.clone(),
            goals: self.goals.clone() + Vector::unit(goal),
        }
    }

    pub fn with_goals(&self, new_goals: impl IntoIterator<Item = PropId>) -> Self {
        let mut goals = self.goals.clone();
        for g in new_goals {
            goals.push_back(g);
        }
        Self {
            subst: self.subst.clone(),
            constraints: self.constraints.clone(),
            goals,
        }
    }

    pub fn pop_goal(&self) -> Option<(PropId, Self)> {
        if self.goals.is_empty() {
            None
        } else {
            let mut goals = self.goals.clone();
            let goal = goals.pop_front().unwrap();
            Some((
                goal,
                Self {
                    subst: self.subst.clone(),
                    constraints: self.constraints.clone(),
                    goals,
                },
            ))
        }
    }

    pub fn is_solved(&self) -> bool {
        self.goals.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchStrategy {
    #[default]
    BFS,
    DFS,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminationReason {
    LimitReached,
    SearchExhausted,
    MaxStepsReached,
}

#[derive(Clone)]
pub struct SolutionSet {
    pub solutions: Vec<State>,
    pub reason: TerminationReason,
}

impl SolutionSet {
    pub fn solutions(&self) -> &[State] {
        &self.solutions
    }
}

pub struct SearchQueue {
    pub queue: VecDeque<State>,
    pub strategy: SearchStrategy,
}

impl SearchQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            strategy: SearchStrategy::default(),
        }
    }

    pub fn with_strategy(strategy: SearchStrategy) -> Self {
        Self {
            queue: VecDeque::new(),
            strategy,
        }
    }

    pub fn push(&mut self, state: State) {
        self.queue.push_back(state);
    }

    pub fn pop(&mut self) -> Option<State> {
        match self.strategy {
            SearchStrategy::BFS => self.queue.pop_front(),
            SearchStrategy::DFS => self.queue.pop_back(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn strategy(&self) -> SearchStrategy {
        self.strategy
    }
}

impl Default for SearchQueue {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Solver<'p> {
    pub program: &'p mut Program,
    fresh_counter: u32,
    z3_solver: z3::Solver,
}

impl<'p> Solver<'p> {
    pub fn new(program: &'p mut Program) -> Self {
        Self {
            program,
            fresh_counter: 0,
            z3_solver: z3::Solver::new(),
        }
    }

    fn fresh_var(&mut self) -> (VarId, TermId) {
        let name = format!("_S{}", self.fresh_counter);
        self.fresh_counter += 1;
        let var = Var { name };
        let var_id = self.program.vars.alloc(var);
        let term_id = self.program.terms.alloc(Term::Var(var_id));
        (var_id, term_id)
    }

    fn instantiate_clause(&mut self, clause: &Clause) -> (Vec<TermId>, PropId) {
        let mut var_map: HashMap<VarId, TermId> = HashMap::new();

        let new_head_args: Vec<TermId> = clause
            .head_args
            .iter()
            .map(|&t| self.rename_term(t, &mut var_map))
            .collect();

        let new_body = self.rename_prop(clause.body, &mut var_map);

        (new_head_args, new_body)
    }

    fn rename_term(&mut self, term_id: TermId, var_map: &mut HashMap<VarId, TermId>) -> TermId {
        match self.program.terms.get(term_id).clone() {
            Term::Var(v) => {
                let var_name = &self.program.vars.get(v).name;
                if let Some(&original_state_var_term_id) = self.program.state_var_term_ids.get(var_name)
                    && original_state_var_term_id == term_id
                {
                    return term_id;
                }
                
                if let Some(&new_term) = var_map.get(&v) {
                    new_term
                } else {
                    let (_, new_term_id) = self.fresh_var();
                    var_map.insert(v, new_term_id);
                    new_term_id
                }
            }
            Term::App { sym, args } => {
                let new_args: Vec<TermId> = args
                    .iter()
                    .map(|&t| self.rename_term(t, var_map))
                    .collect();
                if new_args == args {
                    term_id
                } else {
                    self.program.terms.alloc(Term::App { sym, args: new_args })
                }
            }
            Term::Atom(_) | Term::Int(_) | Term::Float(_) => term_id,
        }
    }

    fn rename_prop(&mut self, prop_id: PropId, var_map: &mut HashMap<VarId, TermId>) -> PropId {
        let prop = self.program.props.get(prop_id).clone();
        match prop {
            Prop::True | Prop::False => prop_id,
            Prop::Eq(t1, t2) => {
                let new_t1 = self.rename_term(t1, var_map);
                let new_t2 = self.rename_term(t2, var_map);
                if new_t1 == t1 && new_t2 == t2 {
                    prop_id
                } else {
                    self.program.props.alloc(Prop::Eq(new_t1, new_t2))
                }
            }
            Prop::And(p1, p2) => {
                let new_p1 = self.rename_prop(p1, var_map);
                let new_p2 = self.rename_prop(p2, var_map);
                if new_p1 == p1 && new_p2 == p2 {
                    prop_id
                } else {
                    self.program.props.alloc(Prop::And(new_p1, new_p2))
                }
            }
            Prop::Or(p1, p2) => {
                let new_p1 = self.rename_prop(p1, var_map);
                let new_p2 = self.rename_prop(p2, var_map);
                if new_p1 == p1 && new_p2 == p2 {
                    prop_id
                } else {
                    self.program.props.alloc(Prop::Or(new_p1, new_p2))
                }
            }
            Prop::Cond(c, p1, p2) => {
                let new_c = self.rename_prop(c, var_map);
                let new_p1 = self.rename_prop(p1, var_map);
                let new_p2 = self.rename_prop(p2, var_map);
                if new_c == c && new_p1 == p1 && new_p2 == p2 {
                    prop_id
                } else {
                    self.program.props.alloc(Prop::Cond(new_c, new_p1, new_p2))
                }
            }
            Prop::Not(p) => {
                let new_p = self.rename_prop(p, var_map);
                if new_p == p {
                    prop_id
                } else {
                    self.program.props.alloc(Prop::Not(new_p))
                }
            }
            Prop::App { rel, ref args } => {
                let new_args: Vec<TermId> = args
                    .iter()
                    .map(|&t| self.rename_term(t, var_map))
                    .collect();
                if new_args == *args {
                    prop_id
                } else {
                    self.program.props.alloc(Prop::App {
                        rel,
                        args: new_args,
                    })
                }
            }
            Prop::Constraint { kind, ref args } => {
                let new_args: Vec<TermId> = args
                    .iter()
                    .map(|&t| self.rename_term(t, var_map))
                    .collect();
                if new_args == *args {
                    prop_id
                } else {
                    self.program.props.alloc(Prop::Constraint {
                        kind,
                        args: new_args,
                    })
                }
            }
        }
    }

    fn step_prop(&mut self, state: State, prop_id: PropId, queue: &mut SearchQueue) {
        let prop = self.program.props.get(prop_id).clone();
        match prop {
            Prop::True => {
                queue.push(state);
            }
            Prop::False => { /* abandon branch */ }
            Prop::Eq(t1, t2) => {
                if let Some(new_subst) = state.subst.unify(t1, t2, &self.program.terms) {
                    queue.push(state.with_subst(new_subst));
                }
            }
            Prop::And(p1, p2) => {
                let new_state = state.with_goals([p1, p2]);
                queue.push(new_state);
            }
            Prop::Or(p1, p2) => {
                queue.push(state.with_goal(p1));
                queue.push(state.with_goal(p2));
            }
            Prop::Cond(c, p1, p2) => {
                let t_prop = self.program.props.alloc(Prop::And(c, p1));
                let e_prop = {
                    let n_prop = self.program.props.alloc(Prop::Not(c));
                    self.program.props.alloc(Prop::And(n_prop, p2))
                };
                queue.push(state.with_goal(t_prop));
                queue.push(state.with_goal(e_prop));
            }
            Prop::Not(p) => {
                let mut neg_queue = SearchQueue::new();
                neg_queue.push(state.with_goal(p));
                
                let mut found_valid_solution = false;

                while let Some(neg_state) = neg_queue.pop() {
                    if let Some((goal, remaining)) = neg_state.pop_goal() {
                        self.step_prop(remaining, goal, &mut neg_queue);
                    } else {
                        // Found a complete proof state - verify constraints are satisfiable
                        if let Some(_solved_subst) = neg_state.constraints.solve_constraints(&neg_state.subst, self.program, &self.z3_solver) {
                             found_valid_solution = true;
                             break;
                         }
                    }
                }
                
                if !found_valid_solution {
                    queue.push(state);
                }
            }
            Prop::App { rel, args } => {
                self.step_user_rel(&state, rel, &args, queue);
            }
            Prop::Constraint { kind, args } => {
                let new_state = state.with_constraint(Constraint { kind, args });
                if let Some((solved_subst, remaining)) = new_state
                    .constraints
                    .propagate_ground(&new_state.subst, self.program, &self.z3_solver)
                {
                    queue.push(State {
                        subst: solved_subst,
                        constraints: remaining,
                        goals: new_state.goals,
                    });
                }
            }
        }
    }

    fn step_user_rel(
        &mut self,
        state: &State,
        rel: RelId,
        args: &[TermId],
        queue: &mut SearchQueue,
    ) {
        let matching_facts: Vec<Vec<TermId>> = self
            .program
            .facts
            .iter()
            .filter_map(|&prop_id| {
                match self.program.props.get(prop_id) {
                    Prop::App { rel: fact_rel, args: fact_args } if *fact_rel == rel => {
                        Some(fact_args.clone())
                    }
                    _ => None,
                }
            })
            .collect();

        for fact_args in matching_facts {
            if let Some(new_subst) = state.subst.unify_args(args, &fact_args, &self.program.terms) {
                queue.push(state.with_subst(new_subst));
            }
        }

        let clauses: Vec<Clause> = self
            .program
            .global_rules
            .iter()
            .filter(|c| c.head_rel == rel)
            .cloned()
            .collect();

        for clause in clauses {
            let (new_head_args, new_body) = self.instantiate_clause(&clause);

            if let Some(new_subst) =
                state.subst.unify_args(args, &new_head_args, &self.program.terms)
            {
                queue.push(state.with_subst(new_subst).with_goal(new_body));
            }
        }
    }


    pub fn step_until_solution(
        &mut self,
        mut queue: SearchQueue,
        max_steps: usize,
    ) -> (Option<State>, SearchQueue) {
        let mut steps = 0;

        while let Some(state) = queue.pop() {
            steps += 1;
            if steps > max_steps {
                queue.push(state);
                return (None, queue);
            }

            if let Some((goal, remaining)) = state.pop_goal() {
                self.step_prop(remaining, goal, &mut queue);
            } else if let Some(solved_subst) =
                state.constraints.solve_constraints(&state.subst, self.program, &self.z3_solver)
            {
                #[cfg(feature = "profile")]
                PROFILE_STATS.with(|stats| {
                    dbg!(&*stats.borrow());
                });
                return (
                    Some(State {
                        subst: solved_subst,
                        constraints: ConstraintStore::new(),
                        goals: Vector::new(),
                    }),
                    queue,
                );
            }
        }
        (None, queue)
    }

    pub fn init_query(&mut self, goal: PropId, strategy: SearchStrategy) -> SearchQueue {
        let mut state = State::new(goal);

        for &fact_prop in &self.program.facts {
            state = state.with_goal(fact_prop);
        }

        let mut queue = SearchQueue::with_strategy(strategy);
        queue.push(state);
        queue
    }

    /// Collect solutions from a query up to a given limit and step count.
    /// Unifies the batch query API into a single canonical path.
    pub fn collect_solutions(
        &mut self,
        goal: PropId,
        strategy: SearchStrategy,
        limit: usize,
        max_steps: usize,
    ) -> SolutionSet {
        let mut queue = self.init_query(goal, strategy);
        let mut solutions = Vec::new();

        loop {
            if solutions.len() >= limit {
                return SolutionSet {
                    solutions,
                    reason: TerminationReason::LimitReached,
                };
            }

            let (solution, remaining_queue) = self.step_until_solution(queue, max_steps);
            
            let hit_max_steps = solution.is_none() && !remaining_queue.is_empty();
            queue = remaining_queue;

            if let Some(state) = solution {
                solutions.push(state);
            } else if hit_max_steps {
                return SolutionSet {
                    solutions,
                    reason: TerminationReason::MaxStepsReached,
                };
            } else {
                return SolutionSet {
                    solutions,
                    reason: TerminationReason::SearchExhausted,
                };
            }
        }
    }
}



pub fn reify_term(term_id: TermId, subst: &Subst, program: &Program) -> String {
    let walked = subst.walk(term_id, &program.terms);
    match program.terms.get(walked) {
        Term::Var(v) => {
            let var = program.vars.get(*v);
            format!("?{}", var.name)
        }
        Term::Atom(s) => program.symbols.get(*s).clone(),
        Term::Int(i) => i.to_string(),
        Term::Float(f) => f.to_string(),
        Term::App { sym, args } => {
            let name = program.symbols.get(*sym).clone();
            let arg_strs: Vec<String> = args
                .iter()
                .map(|a| reify_term(*a, subst, program))
                .collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
    }
}

pub fn format_solution(
    query_vars: &[(String, TermId)],
    state: &State,
    program: &Program,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (name, term_id) in query_vars {
        if !name.starts_with('_') {
            let value = reify_term(*term_id, &state.subst, program);
            parts.push(format!("{} = {}", name, value));
        }
    }
    if !state.constraints.is_empty() {
        parts.push(format!("[{} constraints]", state.constraints.len()));
    }
    if parts.is_empty() {
        "yes".to_string()
    } else {
        parts.join(", ")
    }
}
