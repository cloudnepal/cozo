use std::collections::BTreeSet;
use std::mem;

use anyhow::{ensure, Result};

use crate::data::program::{NormalFormAtom, NormalFormRule};

impl NormalFormRule {
    pub(crate) fn convert_to_well_ordered_rule(self) -> Result<Self> {
        let mut seen_variables: BTreeSet<_> = self.head.iter().cloned().collect();
        let mut round_1_collected = vec![];
        let mut pending = vec![];

        for atom in self.body {
            match atom {
                a @ NormalFormAtom::Unification(ref u) => {
                    if u.is_const() {
                        seen_variables.insert(u.binding.clone());
                        round_1_collected.push(a);
                    } else {
                        let unif_vars = u.bindings_in_expr();
                        if unif_vars.is_subset(&seen_variables) {
                            seen_variables.insert(u.binding.clone());
                            round_1_collected.push(a);
                        } else {
                            pending.push(a);
                        }
                    }
                }
                a @ NormalFormAtom::AttrTriple(ref t) => {
                    seen_variables.insert(t.value.clone());
                    seen_variables.insert(t.entity.clone());
                    round_1_collected.push(a);
                }
                a @ NormalFormAtom::Rule(ref r) => {
                    for arg in &r.args {
                        seen_variables.insert(arg.clone());
                    }
                    round_1_collected.push(a)
                }
                a @ (NormalFormAtom::NegatedAttrTriple(_)
                | NormalFormAtom::NegatedRule(_)
                | NormalFormAtom::Predicate(_)) => {
                    pending.push(a);
                }
            }
        }

        let mut collected = vec![];
        seen_variables = self.head.iter().cloned().collect();
        let mut last_pending = pending;
        let mut pending = vec![];
        for atom in round_1_collected {
            mem::swap(&mut last_pending, &mut pending);
            pending.clear();
            match atom {
                a @ NormalFormAtom::AttrTriple(ref t) => {
                    seen_variables.insert(t.value.clone());
                    seen_variables.insert(t.entity.clone());
                    collected.push(a)
                }
                a @ NormalFormAtom::Rule(ref r) => {
                    seen_variables.extend(r.args.iter().cloned());
                    collected.push(a)
                }
                a @ (NormalFormAtom::NegatedAttrTriple(_)
                | NormalFormAtom::NegatedRule(_)
                | NormalFormAtom::Predicate(_)) => {
                    unreachable!()
                }
                a @ NormalFormAtom::Unification(ref u) => {
                    seen_variables.insert(u.binding.clone());
                    collected.push(a);
                }
            }
            for atom in last_pending {
                match atom {
                    NormalFormAtom::AttrTriple(_) | NormalFormAtom::Rule(_) => unreachable!(),
                    a @ NormalFormAtom::NegatedAttrTriple(ref t) => {
                        if seen_variables.contains(&t.value) && seen_variables.contains(&t.entity) {
                            collected.push(a);
                        } else {
                            pending.push(a);
                        }
                    }
                    a @ NormalFormAtom::NegatedRule(ref r) => {
                        if r.args.iter().map(|a| seen_variables.contains(a)).all() {
                            collected.push(a);
                        } else {
                            pending.push(a);
                        }
                    }
                    a @ NormalFormAtom::Predicate(ref p) => {
                        if p.bindings().is_subset(&seen_variables) {
                            collected.push(a);
                        } else {
                            pending.push(a);
                        }
                    }
                    a @ NormalFormAtom::Unification(ref u) => {
                        if u.bindings_in_expr().is_subset(&seen_variables) {
                            collected.push(a);
                        } else {
                            pending.push(a);
                        }
                    }
                }
            }
        }

        ensure!(
            pending.is_empty(),
            "found unsafe atoms in rule: {:?}",
            pending
        );

        Ok(NormalFormRule {
            head: self.head,
            aggr: self.aggr,
            body: collected,
            vld: self.vld,
        })
    }
}

// fn reorder_rule_body_for_negations(clauses: Vec<Atom>) -> Result<Vec<Atom>> {
//     let (negations, others): (Vec<_>, _) = clauses.into_iter().partition(|a| a.is_negation());
//     let mut seen_bindings = BTreeSet::new();
//     for a in &others {
//         a.collect_bindings(&mut seen_bindings);
//     }
//     let mut negations_with_meta = negations
//         .into_iter()
//         .map(|p| {
//             let p = p.into_negated().unwrap();
//             let mut bindings = Default::default();
//             p.collect_bindings(&mut bindings);
//             let valid_bindings: BTreeSet<_> =
//                 bindings.intersection(&seen_bindings).cloned().collect();
//             (Some(p), valid_bindings)
//         })
//         .collect_vec();
//     let mut ret = vec![];
//     seen_bindings.clear();
//     for a in others {
//         a.collect_bindings(&mut seen_bindings);
//         ret.push(a);
//         for (negated, pred_bindings) in negations_with_meta.iter_mut() {
//             if negated.is_none() {
//                 continue;
//             }
//             if seen_bindings.is_superset(pred_bindings) {
//                 let negated = negated.take().unwrap();
//                 ret.push(Atom::Negation(Box::new(negated)));
//             }
//         }
//     }
//     Ok(ret)
// }
//
// fn reorder_rule_body_for_predicates(clauses: Vec<Atom>) -> Result<Vec<Atom>> {
//     let (predicates, others): (Vec<_>, _) = clauses.into_iter().partition(|a| a.is_predicate());
//     let mut predicates_with_meta = predicates
//         .into_iter()
//         .map(|p| {
//             let p = p.into_predicate().unwrap();
//             let bindings = p.bindings();
//             (Some(p), bindings)
//         })
//         .collect_vec();
//     let mut seen_bindings = BTreeSet::new();
//     let mut ret = vec![];
//     for a in others {
//         a.collect_bindings(&mut seen_bindings);
//         ret.push(a);
//         for (pred, pred_bindings) in predicates_with_meta.iter_mut() {
//             if pred.is_none() {
//                 continue;
//             }
//             if seen_bindings.is_superset(pred_bindings) {
//                 let pred = pred.take().unwrap();
//                 ret.push(Atom::Predicate(pred));
//             }
//         }
//     }
//     for (p, bindings) in predicates_with_meta {
//         ensure!(
//                 p.is_none(),
//                 "unsafe bindings {:?} found in predicate {:?}",
//                 bindings
//                     .difference(&seen_bindings)
//                     .cloned()
//                     .collect::<BTreeSet<_>>(),
//                 p.unwrap()
//             );
//     }
//     Ok(ret)
// }
