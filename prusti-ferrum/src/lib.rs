#![feature(rustc_private)]

//! Direct VIR to native Silver AST conversion.
//!
//! Incomplete cases return errors, never a weakened verification program.
use ferrum_silver::ast::{
    position::{IdentifierPosition, LineColumn, LineColumnPosition, Position},
    r#type::{Type, TypeVar},
};
use std::sync::Arc;
use vir::CompType;

#[derive(Debug)]
pub enum ConversionError {
    UnsupportedType(String),
    DomainTypeRequiresDeclarations(String),
    UnsupportedExpression,
    UnsupportedStatement,
    UnsupportedProgramMember(&'static str),
}

/// Preserve the exact identifier position used by the existing JNI adapter.
pub fn position(span: Option<&vir::VirSpan<'_>>) -> Position {
    match span {
        None => Position::NoPosition,
        Some(span) => Position::LineColumn(LineColumn::Identifier(Box::new(IdentifierPosition {
            file: None,
            start: Box::new(LineColumnPosition::new(0, 0).into()),
            end: None,
            id: span.id.to_string(),
        }))),
    }
}

/// Convert context-independent VIR types. Domain and ADT instances require the
/// program declaration map to preserve generic substitutions and interpretation.
pub fn convert_type<T: CompType>(ty: &vir::Type<'_, T>) -> Result<Type, ConversionError> {
    Ok(match ty.kind() {
        vir::TypeKind::Int => Type::Int,
        vir::TypeKind::Bool => Type::Bool,
        vir::TypeKind::Ref => Type::Ref,
        vir::TypeKind::Perm => Type::Perm,
        vir::TypeKind::DomainTypeParam(p) => Type::Var(TypeVar::new(p.name)),
        vir::TypeKind::Set(t) => Type::Set(Arc::new(convert_type(t)?)),
        vir::TypeKind::Multiset(t) => Type::Multiset(Arc::new(convert_type(t)?)),
        vir::TypeKind::Seq(t) => Type::Seq(Arc::new(convert_type(t)?)),
        vir::TypeKind::Map(k, v) => Type::map(convert_type(k)?, convert_type(v)?),
        vir::TypeKind::Domain(name, _) => {
            return Err(ConversionError::DomainTypeRequiresDeclarations(
                (*name).into(),
            ))
        }
        vir::TypeKind::Unsupported(t) => {
            return Err(ConversionError::UnsupportedType(t.name.into()))
        }
        vir::TypeKind::Err => return Err(ConversionError::UnsupportedType("error type".into())),
    })
}

/// Program-wide domain and ADT declarations required by type conversion.
pub struct TypeContext<'vir> {
    domains: std::collections::HashMap<&'vir str, vir::Domain<'vir>>,
    adts: std::collections::HashMap<&'vir str, vir::Adt<'vir>>,
}
impl<'vir> TypeContext<'vir> {
    pub fn new(program: vir::Program<'vir>) -> Self {
        Self {
            domains: program
                .domains
                .iter()
                .map(|domain| (domain.name, *domain))
                .collect(),
            adts: program.adts.iter().map(|adt| (adt.name, *adt)).collect(),
        }
    }
    pub fn convert<T: CompType>(&self, ty: &vir::Type<'vir, T>) -> Result<Type, ConversionError> {
        use ferrum_silver::ast::r#type::{BackendType, DomainType};
        Ok(match ty.kind() {
            vir::TypeKind::Int => Type::Int,
            vir::TypeKind::Bool => Type::Bool,
            vir::TypeKind::Ref => Type::Ref,
            vir::TypeKind::Perm => Type::Perm,
            vir::TypeKind::DomainTypeParam(p) => Type::Var(TypeVar::new(p.name)),
            vir::TypeKind::Set(t) => Type::Set(Arc::new(self.convert(t)?)),
            vir::TypeKind::Multiset(t) => Type::Multiset(Arc::new(self.convert(t)?)),
            vir::TypeKind::Seq(t) => Type::Seq(Arc::new(self.convert(t)?)),
            vir::TypeKind::Map(k, v) => Type::map(self.convert(k)?, self.convert(v)?),
            vir::TypeKind::Domain(name, args) => {
                let (parameters, interpretation, is_adt) =
                    match (self.domains.get(name), self.adts.get(name)) {
                        (Some(domain), None) => (domain.typarams, domain.interpretation, false),
                        (None, Some(adt)) => (adt.typarams, None, true),
                        _ => {
                            return Err(ConversionError::DomainTypeRequiresDeclarations(
                                (*name).into(),
                            ))
                        }
                    };
                if let Some(backend) = interpretation {
                    Type::Backend(BackendType::new(
                        (*name).into(),
                        backend
                            .interpretation
                            .iter()
                            .map(|pair| (pair.key.into(), pair.value.into()))
                            .collect(),
                    ))
                } else {
                    let vars = parameters
                        .iter()
                        .map(|p| TypeVar::new(p.name))
                        .collect::<Vec<_>>();
                    let substitutions = vars
                        .iter()
                        .zip(args.iter())
                        .map(|(var, arg)| Ok((var.clone(), self.convert(arg)?)))
                        .collect::<Result<_, ConversionError>>()?;
                    if is_adt {
                        Type::Extension(Arc::new(
                            ferrum_silver::plugin::standard::adt::adt_ast_extension::AdtType::new(
                                *name,
                                substitutions,
                                vars,
                            ),
                        ))
                    } else {
                        Type::Domain(DomainType::new(*name, substitutions, vars))
                    }
                }
            }
            vir::TypeKind::Unsupported(t) => {
                return Err(ConversionError::UnsupportedType(t.name.into()))
            }
            vir::TypeKind::Err => {
                return Err(ConversionError::UnsupportedType("error type".into()))
            }
        })
    }
}

/// Construct the validated Silver AST frontend. The caller owns this session on
/// its current thread, as required by the native solver interface.
pub fn native_frontend(
    args: &[String],
    reporter: Arc<dyn ferrum_silver::reporter::reporter::Reporter>,
) -> Result<ferrum::NativeFrontendApi, ferrum::FerrumError> {
    ferrum::NativeFrontendApi::new(args, reporter, false)
}

impl<'vir> TypeContext<'vir> {
    /// Convert an expression without routing through textual Silver syntax.
    pub fn expression<T: CompType>(
        &self,
        expression: &vir::Expr<'vir, T>,
    ) -> Result<ferrum_silver::ast::expression::Exp, ConversionError> {
        use ferrum_silver::ast::{
            ast::Metadata,
            expression::{Exp, ExpKind, LocalVar},
        };
        let meta = if matches!(
            expression.kind,
            vir::ExprKindData::Const(
                vir::ConstData::Wildcard | vir::ConstData::NoPerm | vir::ConstData::FullPerm
            )
        ) {
            Metadata::default()
        } else {
            Metadata {
                position: position(expression.span),
                ..Default::default()
            }
        };
        let kind = match expression.kind {
            vir::ExprKindData::Const(value) => match value {
                vir::ConstData::Bool(true) => ExpKind::TrueLit,
                vir::ConstData::Bool(false) => ExpKind::FalseLit,
                vir::ConstData::Int(value) => ExpKind::IntLit(num_bigint::BigInt::from(*value)),
                vir::ConstData::Wildcard => ExpKind::WildcardPerm,
                vir::ConstData::NoPerm => ExpKind::NoPerm,
                vir::ConstData::FullPerm => ExpKind::FullPerm,
                vir::ConstData::Null => ExpKind::NullLit,
            },
            vir::ExprKindData::Local(local) => ExpKind::LocalVar(LocalVar::new(
                local.name.into(),
                self.convert(&local.ty)?,
                meta.clone(),
            )),
            vir::ExprKindData::Result(ty) => ExpKind::Result(self.convert(ty)?),
            vir::ExprKindData::AccField(acc) => {
                let field = ferrum_silver::ast::program::Field::new(
                    ferrum_silver::ast::program::FieldData {
                        name: acc.field.name.into(),
                        typ: self.convert(&acc.field.ty)?,
                        meta: Metadata::default(),
                    },
                );
                let access = ferrum_silver::ast::expression::FieldAccess::new(
                    self.expression(&acc.recv)?,
                    field,
                    meta.clone(),
                );
                ExpKind::FieldAccessPredicate(
                    access,
                    acc.perm.map(|p| self.expression(&p)).transpose()?,
                )
            }
            vir::ExprKindData::PredicateApp(pred) => {
                let args = pred
                    .args
                    .iter()
                    .map(|arg| self.expression(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                let location = ferrum_silver::ast::expression::PredicateAccess::new(
                    args,
                    pred.target.into(),
                    meta.clone(),
                );
                ExpKind::PredicateAccessPredicate(
                    ferrum_silver::ast::statement::PredicateAccessPredicate::new(
                        location,
                        pred.perm.map(|p| self.expression(&p)).transpose()?,
                        meta.clone(),
                    ),
                )
            }
            vir::ExprKindData::PredicatePerm(pred) => {
                let args = pred
                    .args
                    .iter()
                    .map(|arg| self.expression(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                let location = ferrum_silver::ast::expression::PredicateAccess::new(
                    args,
                    pred.target.into(),
                    meta.clone(),
                );
                ExpKind::CurrentPerm(Exp::new(ExpKind::PredicateAccess(location), meta.clone()))
            }
            vir::ExprKindData::Wand(wand) => {
                ExpKind::MagicWand(self.expression(&wand.lhs)?, self.expression(&wand.rhs)?)
            }
            vir::ExprKindData::InhaleExhale(pair) => ExpKind::InhaleExhaleExp(
                self.expression(&pair.inhale)?,
                self.expression(&pair.exhale)?,
            ),
            vir::ExprKindData::Unfolding(unfolding) => {
                let pred = unfolding.target;
                let args = pred
                    .args
                    .iter()
                    .map(|arg| self.expression(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                let m = Metadata::default();
                let location = ferrum_silver::ast::expression::PredicateAccess::new(
                    args,
                    pred.target.into(),
                    m.clone(),
                );
                let access = ferrum_silver::ast::statement::PredicateAccessPredicate::new(
                    location,
                    pred.perm.map(|p| self.expression(&p)).transpose()?,
                    m.clone(),
                );
                ExpKind::Unfolding(
                    Exp::new(ExpKind::PredicateAccessPredicate(access), m),
                    self.expression(&unfolding.expr)?,
                )
            }
            vir::ExprKindData::Ternary(t) => ExpKind::CondExp(
                self.expression(&t.cond)?,
                self.expression(&t.then)?,
                self.expression(&t.else_)?,
            ),
            vir::ExprKindData::Field(receiver, field) => {
                let field = ferrum_silver::ast::program::Field::new(
                    ferrum_silver::ast::program::FieldData {
                        name: field.name.into(),
                        typ: self.convert(&field.ty)?,
                        meta: Metadata::default(),
                    },
                );
                ExpKind::FieldAccess(ferrum_silver::ast::expression::FieldAccess::new(
                    self.expression(&receiver)?,
                    field,
                    meta.clone(),
                ))
            }
            vir::ExprKindData::Old(old) => {
                let inner = self.expression(&old.expr)?;
                match old.label {
                    vir::OldLabel::None => ExpKind::Old(inner),
                    vir::OldLabel::Lhs => ExpKind::LabelledOld(inner, "lhs".into()),
                    vir::OldLabel::Block(block) => ExpKind::LabelledOld(inner, block.name()),
                    vir::OldLabel::Label(label) => ExpKind::LabelledOld(inner, label.into()),
                }
            }
            vir::ExprKindData::UnOp(op) => {
                let inner = self.expression(&op.expr)?;
                match op.kind {
                    vir::UnOpKind::Neg => ExpKind::Minus(inner),
                    vir::UnOpKind::PermNeg => ExpKind::PermMinus(inner),
                    vir::UnOpKind::Not => ExpKind::Not(inner),
                }
            }
            vir::ExprKindData::BinOp(op) => {
                let lhs = self.expression(&op.lhs)?;
                let rhs = self.expression(&op.rhs)?;
                match op.kind {
                    vir::BinOpKind::CmpEq => ExpKind::EqCmp(lhs, rhs),
                    vir::BinOpKind::CmpNe => ExpKind::NeCmp(lhs, rhs),
                    vir::BinOpKind::CmpGt => ExpKind::GtCmp(lhs, rhs),
                    vir::BinOpKind::CmpLt => ExpKind::LtCmp(lhs, rhs),
                    vir::BinOpKind::CmpGe => ExpKind::GeCmp(lhs, rhs),
                    vir::BinOpKind::CmpLe => ExpKind::LeCmp(lhs, rhs),
                    vir::BinOpKind::And => ExpKind::And(lhs, rhs),
                    vir::BinOpKind::Or => ExpKind::Or(lhs, rhs),
                    vir::BinOpKind::Add => ExpKind::Add(lhs, rhs),
                    vir::BinOpKind::Sub => ExpKind::Sub(lhs, rhs),
                    vir::BinOpKind::Mul => ExpKind::Mul(lhs, rhs),
                    vir::BinOpKind::Div => ExpKind::Div(lhs, rhs),
                    vir::BinOpKind::Mod => ExpKind::Mod(lhs, rhs),
                    vir::BinOpKind::Implies => ExpKind::Implies(lhs, rhs),
                    vir::BinOpKind::PermAdd => ExpKind::PermAdd(lhs, rhs),
                    vir::BinOpKind::PermSub => ExpKind::PermSub(lhs, rhs),
                    vir::BinOpKind::PermMul => ExpKind::PermMul(lhs, rhs),
                    vir::BinOpKind::PermPermDiv => ExpKind::PermPermDiv(lhs, rhs),
                }
            }
            _ => return Err(ConversionError::UnsupportedExpression),
        };
        Ok(Exp::new(kind, meta))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrum_silver::{
        ast::{
            ast::Metadata,
            expression::{Exp, ExpKind},
            program::{Method, MethodData, Program},
            statement::{Stmt, StmtKind},
        },
        reporter::reporter::NoopReporter,
        verifier::verification_result::VerificationResult,
    };

    #[test]
    fn vir_program_reaches_validated_native_verifier() {
        let Some(z3) = std::env::var_os("FERRUM_TEST_Z3") else {
            return;
        };
        std::thread::Builder::new()
            .stack_size(ferrum::VERIFICATION_STACK_SIZE)
            .spawn(move || {
                vir::init_vcx(vir::VirCtxt::new_without_tcx());
                let programs = vir::with_vcx(|vcx| {
                    let mut programs = Vec::new();
                    for value in [true, false] {
                        let label = vcx.alloc(vir::CfgBlockLabelData::End);
                        let assertion = vcx.mk_assert_stmt(if value {
                            vcx.mk_bool::<true>()
                        } else {
                            vcx.mk_bool::<false>()
                        });
                        let terminator = vcx.alloc(vir::TerminatorStmtData::Exit);
                        let block =
                            vcx.mk_cfg_block(label, &[], vcx.alloc_slice(&[assertion]), terminator);
                        let method = vcx.mk_method_unchecked(
                            "test",
                            &[],
                            &[],
                            &[],
                            &[],
                            Some(vcx.alloc_slice(&[block])),
                        );
                        let program =
                            vcx.mk_program(&[], &[], &[], &[], &[], vcx.alloc_slice(&[method]));
                        programs.push(program_to_silver(program).unwrap());
                    }
                    programs
                });
                let args = vec!["--z3Exe".into(), z3.to_string_lossy().into_owned()];
                let mut frontend = native_frontend(&args, Arc::new(NoopReporter)).unwrap();
                assert!(matches!(
                    frontend.verify(&programs[0]).unwrap(),
                    VerificationResult::Success
                ));
                assert!(matches!(
                    frontend.verify(&programs[1]).unwrap(),
                    VerificationResult::Failure(_)
                ));
                frontend.stop().unwrap();
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn constructed_ast_uses_native_frontend_validation_and_verifier() {
        let Some(z3) = std::env::var_os("FERRUM_TEST_Z3") else {
            return;
        };
        std::thread::Builder::new()
            .stack_size(ferrum::VERIFICATION_STACK_SIZE)
            .spawn(move || {
                let args = vec!["--z3Exe".into(), z3.to_string_lossy().into_owned()];
                let mut frontend = native_frontend(&args, Arc::new(NoopReporter)).unwrap();
                let make = |value| {
                    let assertion = Stmt::new(
                        StmtKind::Assert(Exp::new(
                            if value {
                                ExpKind::TrueLit
                            } else {
                                ExpKind::FalseLit
                            },
                            Metadata::default(),
                        )),
                        Metadata::default(),
                    );
                    let method = Method::new(MethodData {
                        name: "test".into(),
                        formal_args: vec![],
                        formal_returns: vec![],
                        pres: vec![],
                        posts: vec![],
                        body: Some(assertion),
                        meta: Metadata::default(),
                    });
                    Program::new(
                        vec![],
                        vec![],
                        vec![],
                        vec![],
                        vec![method],
                        vec![],
                        Metadata::default(),
                    )
                };
                assert!(matches!(
                    frontend.verify(&make(true)).unwrap(),
                    VerificationResult::Success
                ));
                assert!(matches!(
                    frontend.verify(&make(false)).unwrap(),
                    VerificationResult::Failure(_)
                ));
                frontend.stop().unwrap();
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

impl<'vir> TypeContext<'vir> {
    fn local_declaration(
        &self,
        local: vir::LocalDeclDyn<'vir>,
    ) -> Result<ferrum_silver::ast::program::LocalVarDecl, ConversionError> {
        Ok(ferrum_silver::ast::program::LocalVarDecl::new(
            local.name.into(),
            self.convert(&local.ty)?,
            Default::default(),
        ))
    }
    /// Convert a statement and collect its hoisted declarations as the JVM adapter does.
    pub fn statement(
        &self,
        statement: &vir::Stmt<'vir>,
        declarations: &mut Vec<ferrum_silver::ast::program::Declaration>,
    ) -> Result<ferrum_silver::ast::statement::Stmt, ConversionError> {
        use ferrum_silver::ast::{
            ast::{Info, Metadata},
            expression::{ExpKind, LocalVar},
            program::Declaration,
            statement::{Stmt, StmtKind},
        };
        let meta = Metadata {
            position: position(statement.span),
            ..Default::default()
        };
        let empty = |info| {
            Stmt::new(
                StmtKind::Seqn {
                    statements: vec![],
                    declarations: vec![],
                },
                Metadata {
                    info,
                    ..Default::default()
                },
            )
        };
        Ok(match statement.kind {
            vir::StmtKindData::Comment(text) => empty(Info::Simple(vec![(*text).into()])),
            vir::StmtKindData::LocalDecl(decl, initializer) => {
                let silver_decl = self.local_declaration(decl)?;
                declarations.push(Declaration::Local(silver_decl));
                if let Some(rhs) = initializer {
                    let lhs =
                        LocalVar::new(decl.name.into(), self.convert(&decl.ty)?, meta.clone());
                    Stmt::new(
                        StmtKind::LocalVarAssign {
                            lhs,
                            rhs: self.expression(&rhs)?,
                        },
                        Metadata::default(),
                    )
                } else {
                    empty(Info::Simple(vec![format!("var {}", decl.name)]))
                }
            }
            vir::StmtKindData::Assert(exp) => {
                Stmt::new(StmtKind::Assert(self.expression(&exp)?), meta)
            }
            vir::StmtKindData::Inhale(exp) => {
                Stmt::new(StmtKind::Inhale(self.expression(&exp)?), meta)
            }
            vir::StmtKindData::Exhale(exp) => {
                Stmt::new(StmtKind::Exhale(self.expression(&exp)?), meta)
            }
            vir::StmtKindData::PureAssign(assign) => {
                let lhs = self.expression(&assign.lhs)?;
                let rhs = self.expression(&assign.rhs)?;
                match lhs.kind() {
                    ExpKind::LocalVar(local) => Stmt::new(
                        StmtKind::LocalVarAssign {
                            lhs: local.clone(),
                            rhs,
                        },
                        Metadata::default(),
                    ),
                    _ => return Err(ConversionError::UnsupportedStatement),
                }
            }
            vir::StmtKindData::MethodCall(call) => {
                let args = call
                    .args
                    .iter()
                    .map(|arg| self.expression(arg))
                    .collect::<Result<_, _>>()?;
                let targets = call
                    .targets
                    .iter()
                    .map(|target| {
                        Ok(LocalVar::new(
                            target.name.into(),
                            self.convert(&target.ty)?,
                            Default::default(),
                        ))
                    })
                    .collect::<Result<_, ConversionError>>()?;
                Stmt::new(
                    StmtKind::MethodCall {
                        method_name: call.method.into(),
                        args,
                        targets,
                    },
                    meta,
                )
            }
            vir::StmtKindData::If(cond, yes, no) => {
                let make_branch = |this: &Self,
                                   stmts: &[vir::Stmt<'vir>],
                                   decls: &mut Vec<Declaration>|
                 -> Result<Stmt, ConversionError> {
                    Ok(Stmt::new(
                        StmtKind::Seqn {
                            statements: stmts
                                .iter()
                                .map(|stmt| this.statement(stmt, decls))
                                .collect::<Result<_, _>>()?,
                            declarations: vec![],
                        },
                        Metadata::default(),
                    ))
                };
                let yes = make_branch(self, yes, declarations)?;
                let no = make_branch(self, no, declarations)?;
                Stmt::new(
                    StmtKind::If {
                        condition: self.expression(&cond)?,
                        then_branch: yes,
                        else_branch: no,
                    },
                    Metadata::default(),
                )
            }
            _ => return Err(ConversionError::UnsupportedStatement),
        })
    }
}

impl<'vir> TypeContext<'vir> {
    fn comment(&self, text: String) -> ferrum_silver::ast::statement::Stmt {
        use ferrum_silver::ast::{
            ast::{Info, Metadata},
            statement::{Stmt, StmtKind},
        };
        Stmt::new(
            StmtKind::Seqn {
                statements: vec![],
                declarations: vec![],
            },
            Metadata {
                info: Info::Simple(vec![text]),
                ..Default::default()
            },
        )
    }
    fn label(
        &self,
        name: String,
        invariants: Vec<ferrum_silver::ast::expression::Exp>,
    ) -> ferrum_silver::ast::program::LabelDeclaration {
        ferrum_silver::ast::program::LabelDeclaration::new(
            ferrum_silver::ast::program::LabelDeclarationData {
                name,
                invariants,
                meta: Default::default(),
            },
        )
    }
    fn terminator(
        &self,
        terminator: &vir::TerminatorStmt<'vir>,
        declarations: &mut Vec<ferrum_silver::ast::program::Declaration>,
    ) -> Result<Vec<ferrum_silver::ast::statement::Stmt>, ConversionError> {
        use ferrum_silver::ast::{
            ast::Metadata,
            expression::{Exp, ExpKind},
            statement::{Stmt, StmtKind},
        };
        let meta = Metadata::default();
        let goto = |name: String| Stmt::new(StmtKind::Goto(name), meta.clone());
        let seq = |statements| {
            Stmt::new(
                StmtKind::Seqn {
                    statements,
                    declarations: vec![],
                },
                meta.clone(),
            )
        };
        Ok(match terminator {
            vir::TerminatorStmtData::Exit => vec![self.comment("return".into())],
            vir::TerminatorStmtData::Goto(label) => vec![goto(label.name())],
            vir::TerminatorStmtData::AssumeFalse => vec![
                Stmt::new(
                    StmtKind::Inhale(Exp::new(ExpKind::FalseLit, meta.clone())),
                    meta.clone(),
                ),
                goto(vir::CfgBlockLabelData::End.name()),
            ],
            vir::TerminatorStmtData::GotoIf(branch) => {
                let mut target_branches = Vec::with_capacity(branch.targets.len());
                for target in branch.targets {
                    let mut then = target
                        .statements
                        .iter()
                        .map(|s| self.statement(s, declarations))
                        .collect::<Result<Vec<_>, _>>()?;
                    then.push(goto(target.label.name()));
                    target_branches.push((target.value, then));
                }
                let mut otherwise = branch
                    .otherwise_statements
                    .iter()
                    .map(|s| self.statement(s, declarations))
                    .collect::<Result<Vec<_>, _>>()?;
                otherwise.push(goto(branch.otherwise.name()));
                if target_branches.is_empty() {
                    otherwise
                } else {
                    let value = self.expression(&branch.value)?;
                    let mut tail = seq(otherwise);
                    for (target_value, then) in target_branches.into_iter().rev() {
                        let condition = Exp::new(
                            ExpKind::EqCmp(value.clone(), self.expression(&target_value)?),
                            meta.clone(),
                        );
                        tail = seq(vec![Stmt::new(
                            StmtKind::If {
                                condition,
                                then_branch: seq(then),
                                else_branch: tail,
                            },
                            meta.clone(),
                        )]);
                    }
                    vec![tail]
                }
            }
            vir::TerminatorStmtData::Dummy(_) => return Err(ConversionError::UnsupportedStatement),
        })
    }
    pub fn method(
        &self,
        method: &vir::Method<'vir>,
    ) -> Result<ferrum_silver::ast::program::Method, ConversionError> {
        use ferrum_silver::ast::{
            ast::Metadata,
            program::{Declaration, Method, MethodData},
            statement::{Stmt, StmtKind},
        };
        let args = method
            .args
            .iter()
            .map(|a| self.local_declaration(a))
            .collect::<Result<_, _>>()?;
        let rets = method
            .rets
            .iter()
            .map(|a| self.local_declaration(a))
            .collect::<Result<_, _>>()?;
        let pres = method
            .pres
            .iter()
            .map(|e| self.expression(e))
            .collect::<Result<_, _>>()?;
        let posts = method
            .posts
            .iter()
            .map(|e| self.expression(e))
            .collect::<Result<_, _>>()?;
        let body = if let Some(body) = method.body {
            let mut stmts = Vec::new();
            let mut declarations = Vec::new();
            for block in body.blocks {
                let name = block.label.label.name();
                declarations.push(Declaration::Label(self.label(name.clone(), vec![])));
                let invariants = block
                    .label
                    .invariants
                    .iter()
                    .map(|e| self.expression(e))
                    .collect::<Result<_, _>>()?;
                stmts.push(Stmt::new(
                    StmtKind::Label(self.label(name, invariants)),
                    Metadata::default(),
                ));
                for stmt in block.stmts {
                    stmts.push(self.statement(stmt, &mut declarations)?);
                }
                stmts.extend(self.terminator(&block.terminator, &mut declarations)?);
            }
            Some(Stmt::new(
                StmtKind::Seqn {
                    statements: stmts,
                    declarations,
                },
                Metadata::default(),
            ))
        } else {
            None
        };
        Ok(Method::new(MethodData {
            name: method.name.into(),
            formal_args: args,
            formal_returns: rets,
            pres,
            posts,
            body,
            meta: Metadata::default(),
        }))
    }
}

/// Convert a supported VIR program directly into Silver nodes. Untranslated
/// declaration families fail closed rather than being dropped.
pub fn program_to_silver(
    program: vir::Program<'_>,
) -> Result<ferrum_silver::ast::program::Program, ConversionError> {
    use ferrum_silver::ast::{
        ast::Metadata,
        program::{Field, FieldData, Program},
    };
    if !program.domains.is_empty() {
        return Err(ConversionError::UnsupportedProgramMember("domains"));
    }
    if !program.adts.is_empty() {
        return Err(ConversionError::UnsupportedProgramMember("adts"));
    }
    let context = TypeContext::new(program);
    let fields = program
        .fields
        .iter()
        .map(|field| {
            Ok(Field::new(FieldData {
                name: field.name.into(),
                typ: context.convert(&field.ty)?,
                meta: Metadata::default(),
            }))
        })
        .collect::<Result<Vec<_>, ConversionError>>()?;
    let predicates = program
        .predicates
        .iter()
        .map(|predicate| {
            Ok(ferrum_silver::ast::program::Predicate::new(
                ferrum_silver::ast::program::PredicateData {
                    name: predicate.name.into(),
                    formal_args: predicate
                        .args
                        .iter()
                        .map(|a| context.local_declaration(a))
                        .collect::<Result<_, _>>()?,
                    body: predicate.expr.map(|e| context.expression(&e)).transpose()?,
                    meta: Metadata::default(),
                },
            ))
        })
        .collect::<Result<Vec<_>, ConversionError>>()?;
    let functions = program
        .functions
        .iter()
        .map(|function| {
            if !matches!(function.decreases, vir::DecreasesGenData::None) {
                return Err(ConversionError::UnsupportedProgramMember(
                    "function decreases",
                ));
            }
            Ok(ferrum_silver::ast::program::Function::new(
                ferrum_silver::ast::program::FunctionData {
                    name: function.name.into(),
                    formal_args: function
                        .args
                        .iter()
                        .map(|a| context.local_declaration(a))
                        .collect::<Result<_, _>>()?,
                    typ: context.convert(&function.ret)?,
                    pres: function
                        .pres
                        .iter()
                        .map(|e| context.expression(e))
                        .collect::<Result<_, _>>()?,
                    posts: function
                        .posts
                        .iter()
                        .map(|e| context.expression(e))
                        .collect::<Result<_, _>>()?,
                    body: function.expr.map(|e| context.expression(&e)).transpose()?,
                    meta: Metadata::default(),
                },
            ))
        })
        .collect::<Result<Vec<_>, ConversionError>>()?;
    let methods = program
        .methods
        .iter()
        .map(|method| context.method(method))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Program::new(
        vec![],
        fields,
        functions,
        predicates,
        methods,
        vec![],
        Metadata::default(),
    ))
}
