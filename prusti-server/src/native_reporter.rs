//! Translate native Silver reporter events to Prusti server messages.
use crate::ServerMessage;
use prusti_ferrum::silver::{
    ast::program::Member,
    reporter::{
        message::Message,
        reporter::{ReportError, Reporter},
    },
    verifier::verification_result::AbstractError,
};
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    sync::{mpsc, Mutex},
};
use viper::{VerificationError, VerificationResultKind};

/// Source polling reporter queues before forwarding to the optional CSV reporter.
pub(crate) struct DebugReporter {
    pub progress: std::sync::Arc<dyn Reporter>,
    pub csv: std::sync::Arc<dyn Reporter>,
}
impl Reporter for DebugReporter {
    fn name(&self) -> &str {
        "polling_reporter"
    }
    fn report(&self, message: Message) -> Result<(), ReportError> {
        self.progress.report(message.clone())?;
        self.csv.report(message)
    }
}

pub(crate) struct NativeReporter {
    pub procedures: HashSet<String>,
    pub sender: mpsc::Sender<ServerMessage>,
    pub errors: Mutex<HashSet<u64>>,
}
impl NativeReporter {
    pub fn error_hash(error: &VerificationError) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        error.full_id.hash(&mut h);
        error.pos_id.hash(&mut h);
        error.offending_pos_id.hash(&mut h);
        error.reason_pos_id.hash(&mut h);
        h.finish()
    }
    fn send(&self, message: ServerMessage) -> Result<(), ReportError> {
        self.sender.send(message).map_err(|e| {
            ReportError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                e.to_string(),
            ))
        })
    }
}
impl Reporter for NativeReporter {
    fn name(&self) -> &str {
        "prusti-native"
    }
    fn report(&self, message: Message) -> Result<(), ReportError> {
        let (concerning, time, result) = match message {
            Message::EntitySuccess {
                concerning, time, ..
            } => (concerning, time, VerificationResultKind::Success),
            Message::EntityFailure {
                concerning,
                time,
                errors,
                ..
            } => {
                if time < 0
                    || !matches!(concerning.as_ref(), Member::Method(m) if self.procedures.contains(&m.name))
                {
                    return Ok(());
                }
                let mut converted = Vec::new();
                for error in errors {
                    let AbstractError::Verification(error) = error else {
                        return Err(ReportError::Unsupported("non-verification entity error"));
                    };
                    let mapped = prusti_ferrum::verification_error(error)?;
                    let mut error = VerificationError::new(
                        mapped.full_id,
                        mapped.pos_id,
                        mapped.offending_pos_id,
                        mapped.reason_pos_id,
                        mapped.message,
                        None,
                    );
                    let is_new = self
                        .errors
                        .lock()
                        .map_err(|_| ReportError::Unsupported("poisoned reporter error set"))?
                        .insert(Self::error_hash(&error));
                    if is_new {
                        error.counterexample =
                            prusti_ferrum::counterexample::from_diagnostic(&mapped.diagnostic)
                                .map_err(ReportError::Plugin)?;
                        converted.push(error);
                    }
                }
                (concerning, time, VerificationResultKind::Failure(converted))
            }
            Message::QuantifierInstantiations {
                quantifier,
                instantiations,
                ..
            } => {
                if let Some(name) = quantifier.strip_prefix("prog.l") {
                    let name = name
                        .strip_suffix("-aux")
                        .or_else(|| name.strip_suffix("_precondition"))
                        .unwrap_or(name);
                    if let Ok(pos_id) = name.parse::<usize>() {
                        let insts = u64::try_from(instantiations)
                            .map_err(|_| ReportError::Unsupported("negative quantifier count"))?;
                        self.send(ServerMessage::QuantifierInstantiation {
                            q_name: quantifier,
                            insts,
                            pos_id,
                        })?;
                    }
                }
                return Ok(());
            }
            Message::QuantifierChosenTriggers {
                quantifier,
                triggers,
                ..
            } => {
                let position = quantifier.metadata().position.to_string();
                if let Some((_, position)) = position.rsplit_once('.') {
                    let pos_id = position
                        .parse::<usize>()
                        .map_err(|_| ReportError::Unsupported("invalid quantifier position"))?;
                    let viper_quant =
                        prusti_ferrum::silver::ast::pretty::pretty_printer::expression_to_string(
                            &quantifier,
                        )?;
                    let mut parts = Vec::new();
                    for trigger in triggers {
                        let mut exps = Vec::new();
                        for exp in &trigger.exps {
                            exps.push(prusti_ferrum::silver::ast::pretty::pretty_printer::expression_to_string(exp)?);
                        }
                        parts.push(format!("{{{}}}", exps.join(", ")));
                    }
                    self.send(ServerMessage::QuantifierChosenTriggers {
                        viper_quant,
                        triggers: format!("[{}]", parts.join(", ")),
                        pos_id,
                    })?;
                }
                return Ok(());
            }
            _ => return Ok(()),
        };
        if let Member::Method(method) = concerning.as_ref() {
            if time >= 0 && self.procedures.contains(&method.name) {
                self.send(ServerMessage::MethodTermination {
                    viper_method_name: method.name.clone(),
                    result,
                    verification_time: time as u128,
                })?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prusti_ferrum::silver::{
        ast::{
            ast::Metadata,
            expression::{Exp, ExpKind},
            program::{Method, MethodData, Program},
            statement::{Stmt, StmtKind},
        },
        reporter::reporter::PluginAwareReporter,
    };
    use std::sync::{Arc, RwLock};
    #[test]
    fn chosen_triggers_carry_identifier_and_source_syntax() {
        use prusti_ferrum::silver::ast::{
            expression::Trigger,
            position::{IdentifierPosition, LineColumn, LineColumnPosition, Position},
        };
        let (sender, receiver) = mpsc::channel();
        let reporter = NativeReporter {
            procedures: HashSet::new(),
            sender,
            errors: Mutex::new(HashSet::new()),
        };
        let meta = Metadata {
            position: Position::LineColumn(LineColumn::Identifier(Box::new(IdentifierPosition {
                file: None,
                start: Box::new(LineColumnPosition::new(0, 0).into()),
                end: None,
                id: "42".into(),
            }))),
            ..Default::default()
        };
        let quantifier = Exp::new(
            ExpKind::Forall(
                vec![],
                vec![],
                Exp::new(ExpKind::TrueLit, Metadata::default()),
            ),
            meta,
        );
        let triggers = vec![Trigger::new(
            vec![
                Exp::new(ExpKind::IntLit(1.into()), Metadata::default()),
                Exp::new(ExpKind::IntLit(2.into()), Metadata::default()),
            ],
            Metadata::default(),
        )];
        reporter
            .report(Message::QuantifierChosenTriggers {
                quantifier,
                triggers,
                old_triggers: vec![],
            })
            .unwrap();
        assert!(
            matches!(receiver.try_recv().unwrap(), ServerMessage::QuantifierChosenTriggers { triggers, pos_id: 0, .. } if triggers == "[{1, 2}]")
        );
    }

    #[test]
    fn long_trigger_expressions_use_flat_source_to_string() {
        use prusti_ferrum::silver::ast::{
            expression::Trigger,
            position::{IdentifierPosition, LineColumn, LineColumnPosition, Position},
            pretty::pretty_printer::{expression_to_string, pretty_expression},
        };
        let (sender, receiver) = mpsc::channel();
        let reporter = NativeReporter {
            procedures: HashSet::new(),
            sender,
            errors: Mutex::new(HashSet::new()),
        };
        let meta = Metadata {
            position: Position::LineColumn(LineColumn::Identifier(Box::new(IdentifierPosition {
                file: None,
                start: Box::new(LineColumnPosition::new(0, 0).into()),
                end: None,
                id: "42".into(),
            }))),
            ..Default::default()
        };
        let mut exp = Exp::new(ExpKind::TrueLit, Metadata::default());
        for _ in 0..40 {
            exp = Exp::new(
                ExpKind::And(exp, Exp::new(ExpKind::TrueLit, Metadata::default())),
                Metadata::default(),
            );
        }
        assert!(pretty_expression(&exp).unwrap().contains('\n'));
        let quantifier = Exp::new(ExpKind::Forall(vec![], vec![], exp.clone()), meta);
        let expected_quant = expression_to_string(&quantifier).unwrap();
        let expected_trigger = format!("[{{{}}}]", expression_to_string(&exp).unwrap());
        reporter
            .report(Message::QuantifierChosenTriggers {
                quantifier,
                triggers: vec![Trigger::new(vec![exp], Metadata::default())],
                old_triggers: vec![],
            })
            .unwrap();
        let ServerMessage::QuantifierChosenTriggers {
            viper_quant,
            triggers,
            ..
        } = receiver.try_recv().unwrap()
        else {
            panic!("wrong event")
        };
        assert_eq!(viper_quant, expected_quant);
        assert_eq!(triggers, expected_trigger);
        assert!(!viper_quant.contains('\n'));
        assert!(!triggers.contains('\n'));
    }

    #[test]
    fn quantifier_counts_follow_source_name_filter() {
        let (sender, receiver) = mpsc::channel();
        let reporter = NativeReporter {
            procedures: HashSet::new(),
            sender,
            errors: Mutex::new(HashSet::new()),
        };
        for quantifier in ["prog.l42-aux", "unrelated", "prog.lbad"] {
            reporter
                .report(Message::QuantifierInstantiations {
                    quantifier: quantifier.into(),
                    instantiations: 7,
                    max_gen: 0,
                    max_cost: 0,
                })
                .unwrap();
        }
        assert!(
            matches!(receiver.try_recv().unwrap(), ServerMessage::QuantifierInstantiation { q_name, insts: 7, pos_id: 42 } if q_name == "prog.l42-aux")
        );
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    #[ignore = "requires real Z3; set Z3_EXE"]
    fn actual_method_results_reach_server_channel() {
        let z3 = std::env::var("Z3_EXE").expect("set Z3_EXE");
        std::thread::Builder::new().stack_size(128*1024*1024).spawn(move || {
            let (tx,rx) = mpsc::channel();
            let target = Arc::new(NativeReporter { procedures: ["good".to_string(),"bad".to_string()].into_iter().collect(), sender: tx, errors: Mutex::new(HashSet::new()) });
            let reporter = Arc::new(PluginAwareReporter { name: "prusti".into(), target: target.clone(), mapper: RwLock::new(None) });
            let method = |name: &str, good| Method::new(MethodData { name:name.into(), formal_args:vec![], formal_returns:vec![], pres:vec![], posts:vec![], body:Some(Stmt::new(StmtKind::Assert(Exp::new(if good { ExpKind::TrueLit } else { ExpKind::FalseLit }, Metadata::default())), Metadata::default())), meta:Metadata::default() });
            let program=Program::new(vec![],vec![],vec![],vec![],vec![method("good",true),method("bad",false),method("unselected",true)],vec![],Metadata::default());
            let mut frontend = prusti_ferrum::native_frontend(&["--z3Exe".into(),z3],reporter).unwrap();
            frontend.verify(&program).unwrap(); frontend.stop().unwrap();
            let msgs:Vec<_>=rx.try_iter().collect();
            assert_eq!(msgs.len(),2,"{msgs:?}");
            assert!(msgs.iter().any(|m| matches!(m,ServerMessage::MethodTermination{viper_method_name,result:VerificationResultKind::Success,..} if viper_method_name=="good")));
            assert!(msgs.iter().any(|m| matches!(m,ServerMessage::MethodTermination{viper_method_name,result:VerificationResultKind::Failure(e),..} if viper_method_name=="bad" && e.len()==1)));
            assert_eq!(target.errors.lock().unwrap().len(),1);
        }).unwrap().join().unwrap();
    }
}
