use typing::{Type, TypeEnv};
use typing::type_env::TypeId;
use span::CSpan;
use errors::{Emitter, Diag };
use std::rc::Rc;
use std::cell::RefCell;
use std::process::exit;
use typing::typed_term::TyTerm;

use typing::constraint::{Constraints, Equals};
use typing::substitution::Substitution;

pub struct Unifier {
    pub emitter: Rc<RefCell<Emitter>>,
    pub tenv: Rc<RefCell<TypeEnv>>,
    pub unresolved: Rc<RefCell<Vec<TyTerm>>>,
}

impl Unifier {

    pub fn new(emitter: Rc<RefCell<Emitter>>, tenv: Rc<RefCell<TypeEnv>>, unresolved: Rc<RefCell<Vec<TyTerm>>>) -> Unifier {
        Unifier {
            emitter,
            tenv,
            unresolved,
        }
    }

    pub fn unify(&mut self, cs: Constraints) -> Substitution {
        if cs.is_empty() {
            Substitution::empty()
        } else {
            let emitter = cs.emitter.clone();
            let tenv = cs.tenv.clone();
            let unresolved = cs.unresolved.clone();
            let mut it = cs.set.into_iter();
            let mut subst = self.unify_one(it.next().unwrap());
            let subst_tail = subst.apply(&Constraints {set: it.collect(), emitter, tenv, unresolved});
            let subst_tail: Substitution = self.unify(subst_tail);
            subst.compose(subst_tail)
        }
    }

    fn unify_one(&mut self, eq: Equals) -> Substitution {
        use self::Type::*;
        // println!("{:?}", eq);
        let emitter = Rc::clone(&self.emitter);
        let tenv = Rc::clone(&self.tenv);
        let unresolved = Rc::clone(&self.unresolved);
        match eq {
            Equals(Unit(_), Unit(_)) => Substitution::empty(),
            Equals(INT(_), INT(_)) => Substitution::empty(),
            Equals(FLOAT(_), FLOAT(_)) => Substitution::empty(),
            Equals(BOOL(_), BOOL(_)) => Substitution::empty(),

            Equals(INT(_), ResolvedDim(_, _)) => Substitution::empty(),
            Equals(ResolvedDim(_, _), INT(_)) => Substitution::empty(),

            Equals(a @ ResolvedDim(_, _), b @ ResolvedDim(_, _)) => {
                if a.as_num() == b.as_num() {
                    Substitution::empty()
                } else {
                    self.emitter.borrow_mut().add(Diag::DimensionMismatch(a.clone(), b.clone()));
                    Substitution::empty()
                }
            }

            Equals(VAR(tvar, _), ty) => self.unify_var(tvar, ty),
            Equals(ty, VAR(tvar, _)) => self.unify_var(tvar, ty),

            Equals(DIM(tvar, _), ty) => self.unify_var(tvar, ty),
            Equals(ty, DIM(tvar, _)) => self.unify_var(tvar, ty),

            Equals(FnArgs(v1, _), FnArgs(v2, _)) => self.unify(
                Constraints {
                    set: v1.into_iter().zip(v2).map(|(i, j)| Equals(i, j)).collect(),
                    emitter,
                    tenv,
                    unresolved,
                },
            ),

            Equals(FnArg(Some(a), ty1, _), FnArg(Some(b), ty2, _)) => {
                if a == b {
                    self.unify(
                        Constraints {
                            set: btreeset!{ Equals(*ty1, *ty2)},
                            emitter,
                            tenv,
                            unresolved,
                        },
                        )
                } else {
                    panic!("supplied parameter is incorrect! {} != {}", a, b);
                }
            }

            Equals(FUN(m1,n1,p1, r1, _), FUN(m2,n2,p2, r2, _)) => {
                if n1 == n2 {
                    self.unify(
                        Constraints{
                            set: btreeset!{
                                Equals(*p1, *p2),
                                Equals(*r1, *r2),
                            },
                            emitter,
                            tenv,
                            unresolved,
                        },
                    )
                } else {
                    println!("{} {} {} {}", m1, m2, n1, n2);
                    panic!()
                }
            },

            Equals(Tuple(vs1, _), Tuple(vs2, _)) => self.unify(
                Constraints {
                    set: vs1.into_iter().zip(vs2).map(|(i,j)| Equals(i,j)).collect(),
                    emitter,
                    tenv,
                    unresolved,
                },
            ),

            Equals(ts1 @ TSR(_, _), ts2 @ TSR(_, _)) => {
                if ts1.as_rank() == ts2.as_rank() {
                    if let (TSR(dims1, s1), TSR(dims2, s2)) = (ts1.clone(), ts2.clone()) {
                        let cons = Constraints {
                            set: dims1
                                .into_iter()
                                .zip(dims2)
                                .filter_map(|(i, j)| {
                                    if let (Type::ResolvedDim(a,_), Type::ResolvedDim(b,_)) = (i.clone(),j.clone()) {
                                        if a != b { self.emitter.borrow_mut().add(Diag::TypeError(ts1.clone(),ts2.clone())) }
                                        None
                                    } else {
                                        Some(Equals(i.with_span(&s1), j.with_span(&s2)))
                                    }
                                })
                                .collect(),
                            emitter,
                            tenv,
                            unresolved,
                        };
                        self.unify(cons)
                    } else {
                        unimplemented!();
                    }
                } else {
                    self.emitter.borrow_mut().add(Diag::RankMismatch(ts1, ts2));
                    Substitution::empty()
                }
            }

            Equals(Module(n1, Some(box ty1), _), Module(n2, Some(box ty2), _)) => self.unify(
                Constraints {
                    set: btreeset!{
                        if n1 == n2 {
                            Equals(ty1, ty2)
                        } else {
                            panic!();
                        }
                    },
                    emitter,
                    tenv,
                    unresolved,
                },
            ),

            Equals(u @ UnresolvedModuleFun(_, _, _, _), ty) => {
                Substitution(btreemap!(
                    u => ty,
                ))
            }

            _ => {
                let Equals(a, b) = eq;
                let mut em = self.emitter.borrow_mut();
                em.add(Diag::TypeError(a, b));
                em.print_errs();
                exit(-1);
            }
        }
    }

    fn unify_var(&mut self, tvar: TypeId, ty: Type) -> Substitution {
        use self::Type::*;

        let span = CSpan::fresh_span();
        match ty.clone() {
            VAR(tvar2, _) => {
                if tvar == tvar2 {
                    Substitution::empty()
                } else {
                    Substitution(btreemap!{ VAR(tvar, span) => ty })
                }
            }
            DIM(tvar2, _) => {
                if tvar == tvar2 {
                    Substitution::empty()
                } else {
                    Substitution(btreemap!{ VAR(tvar, span) => ty })
                }
            }
            _ => if occurs(tvar, &ty) {
                panic!("circular type")
            } else {
                Substitution(btreemap!{ VAR(tvar, span) => ty })
            },
        }
    }
}

fn occurs(tvar: TypeId, ty: &Type) -> bool {
    use self::Type::*;
    match ty {
        FUN(_,_, ref p, ref r, _) => occurs(tvar, &p) | occurs(tvar, &r),
        VAR(ref tvar2, _) => tvar == *tvar2,
        _ => false,
    }
}
