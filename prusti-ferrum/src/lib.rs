#![feature(rustc_private)]

//! Direct VIR to native Silver AST conversion.
//!
//! Incomplete cases return errors, never a weakened verification program.
pub use ferrum_silver as silver;
pub mod counterexample;
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
    UnsupportedStatement(String),
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
    domain_functions:
        std::collections::HashMap<&'vir str, (vir::Domain<'vir>, vir::DomainFunction<'vir>)>,
    domain_axioms: std::collections::HashMap<&'vir str, vir::Domain<'vir>>,
    adt_constructors:
        std::collections::HashMap<&'vir str, (vir::Adt<'vir>, vir::AdtConstructor<'vir>)>,
    adt_destructors: std::collections::HashMap<&'vir str, vir::Adt<'vir>>,
}
/// Retain startup warnings even when frontend construction fails.
pub fn native_frontend_with_warning_sink(
    args: &[String],
    reporter: Arc<dyn ferrum_silver::reporter::reporter::Reporter>,
    warnings: Arc<std::sync::Mutex<Vec<String>>>,
) -> Result<ferrum::NativeFrontendApi, ferrum::FerrumError> {
    ferrum::NativeFrontendApi::new_with_warning_sink(args, reporter, false, warnings)
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
            domain_functions: program
                .domains
                .iter()
                .flat_map(|domain| {
                    domain
                        .functions
                        .iter()
                        .map(move |f| (f.name.to_str(), (*domain, *f)))
                })
                .collect(),
            domain_axioms: program
                .domains
                .iter()
                .flat_map(|domain| domain.axioms.iter().map(move |axiom| (axiom.name, *domain)))
                .collect(),
            adt_constructors: program
                .adts
                .iter()
                .flat_map(|adt| adt.constructors.iter().map(move |c| (c.name, (*adt, *c))))
                .collect(),
            adt_destructors: program
                .adts
                .iter()
                .flat_map(|adt| {
                    adt.constructors
                        .iter()
                        .flat_map(move |c| c.args.iter().map(move |arg| (arg.name, *adt)))
                })
                .collect(),
        }
    }
    fn adt_type_map(
        &self,
        ty: vir::TypeDyn<'vir>,
    ) -> Result<ferrum_silver::ast::r#type::TypeSubstitution, ConversionError> {
        let vir::TypeKind::Domain(name, args) = ty.kind() else {
            return Err(ConversionError::UnsupportedExpression);
        };
        let adt = self
            .adts
            .get(name)
            .ok_or(ConversionError::UnsupportedExpression)?;
        if adt.typarams.len() != args.len() {
            return Err(ConversionError::UnsupportedExpression);
        }
        adt.typarams
            .iter()
            .zip(args.iter())
            .map(|(p, t)| Ok((TypeVar::new(p.name), self.convert(t)?)))
            .collect()
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
                if parameters.len() != args.len() {
                    return Err(ConversionError::DomainTypeRequiresDeclarations(format!(
                        "{name}: expected {} type arguments, got {}",
                        parameters.len(),
                        args.len()
                    )));
                }
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
        let mut extra = ferrum_silver::ast::expression::ExpExtra::default();
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
            vir::ExprKindData::FuncApp(app) => {
                let args = app
                    .args
                    .iter()
                    .map(|a| self.expression(a))
                    .collect::<Result<Vec<_>, _>>()?;
                extra.typ = Some(self.convert(&app.result_ty)?);
                if let Some((domain, function)) = self.domain_functions.get(app.target) {
                    if domain.typarams.len() != app.typ_var_map.len() {
                        return Err(ConversionError::UnsupportedExpression);
                    }
                    if let Some(interp) = &function.interpretation {
                        extra.interpretation = Some(interp.interpretation.into());
                        ExpKind::BackendFuncApp(app.target.into(), args)
                    } else {
                        extra.domain_name = Some(domain.name.into());
                        let substitutions = domain
                            .typarams
                            .iter()
                            .zip(app.typ_var_map)
                            .map(|(p, t)| Ok((TypeVar::new(p.name), self.convert(t)?)))
                            .collect::<Result<_, ConversionError>>()?;
                        ExpKind::DomainFuncApp(app.target.into(), args, substitutions)
                    }
                } else if let Some((adt, _)) = self.adt_constructors.get(app.target) {
                    if !app.typ_var_map.is_empty() {
                        return Err(ConversionError::UnsupportedExpression);
                    }
                    let type_vars = self.adt_type_map(app.result_ty)?;
                    let node = ferrum_silver::plugin::standard::adt::adt_ast_extension::AdtConstructorApp { name:app.target.into(), args, type_vars, typ:self.convert(&app.result_ty)?, adt_name:adt.name.into(), meta:meta.clone() };
                    return Ok(node.expression());
                } else if !app.typ_var_map.is_empty() {
                    return Err(ConversionError::UnsupportedExpression);
                } else {
                    ExpKind::FuncApp(app.target.into(), args)
                }
            }
            vir::ExprKindData::AdtDestructor(receiver, field) => {
                let adt = self
                    .adt_destructors
                    .get(field.name)
                    .ok_or(ConversionError::UnsupportedExpression)?;
                let node =
                    ferrum_silver::plugin::standard::adt::adt_ast_extension::AdtDestructorApp {
                        name: field.name.into(),
                        receiver: self.expression(&receiver)?,
                        type_vars: self.adt_type_map(receiver.ty())?,
                        typ: self.convert(&field.ty)?,
                        adt_name: adt.name.into(),
                        meta: meta.clone(),
                    };
                return Ok(node.expression());
            }
            vir::ExprKindData::AdtDiscriminator(receiver, constructor) => {
                let (adt, _) = self
                    .adt_constructors
                    .get(constructor)
                    .ok_or(ConversionError::UnsupportedExpression)?;
                let node =
                    ferrum_silver::plugin::standard::adt::adt_ast_extension::AdtDiscriminatorApp {
                        name: (*constructor).into(),
                        receiver: self.expression(&receiver)?,
                        type_vars: self.adt_type_map(receiver.ty())?,
                        adt_name: adt.name.into(),
                        meta: meta.clone(),
                    };
                return Ok(node.expression());
            }
            vir::ExprKindData::Let(binding) => {
                let declaration = ferrum_silver::ast::program::LocalVarDecl::new(
                    binding.name.into(),
                    self.convert(&binding.val.ty())?,
                    Metadata::default(),
                );
                ExpKind::Let(
                    declaration,
                    self.expression(&binding.val)?,
                    self.expression(&binding.expr)?,
                )
            }
            vir::ExprKindData::Forall(q) => {
                let vars = q
                    .qvars
                    .iter()
                    .map(|v| self.local_declaration(v))
                    .collect::<Result<_, _>>()?;
                let triggers = q
                    .triggers
                    .iter()
                    .map(|t| {
                        Ok(ferrum_silver::ast::expression::Trigger::new(
                            t.exprs
                                .iter()
                                .map(|e| self.expression(e))
                                .collect::<Result<_, _>>()?,
                            Metadata::default(),
                        ))
                    })
                    .collect::<Result<_, ConversionError>>()?;
                ExpKind::Forall(vars, triggers, self.expression(&q.body)?)
            }
            vir::ExprKindData::Exists(q) => {
                let vars = q
                    .qvars
                    .iter()
                    .map(|v| self.local_declaration(v))
                    .collect::<Result<_, _>>()?;
                let triggers = q
                    .triggers
                    .iter()
                    .map(|t| {
                        Ok(ferrum_silver::ast::expression::Trigger::new(
                            t.exprs
                                .iter()
                                .map(|e| self.expression(e))
                                .collect::<Result<_, _>>()?,
                            Metadata::default(),
                        ))
                    })
                    .collect::<Result<_, ConversionError>>()?;
                ExpKind::Exists(vars, triggers, self.expression(&q.body)?)
            }
            vir::ExprKindData::CollectionBinOp(op) => {
                let lhs = self.expression(&op.lhs)?;
                let rhs = self.expression(&op.rhs)?;
                match op.kind {
                    vir::CollectionBinOpKind::Contains => match op.rhs.ty().kind() {
                        vir::TypeKind::Set(_) | vir::TypeKind::Multiset(_) => {
                            ExpKind::AnySetContains(lhs, rhs)
                        }
                        vir::TypeKind::Seq(_) => ExpKind::SeqContains(lhs, rhs),
                        vir::TypeKind::Map(..) => ExpKind::MapContains(rhs, lhs),
                        _ => return Err(ConversionError::UnsupportedExpression),
                    },
                    vir::CollectionBinOpKind::Index => match op.lhs.ty().kind() {
                        vir::TypeKind::Seq(_) => ExpKind::SeqIndex(lhs, rhs),
                        vir::TypeKind::Map(..) => ExpKind::MapLookup(lhs, rhs),
                        _ => return Err(ConversionError::UnsupportedExpression),
                    },
                    vir::CollectionBinOpKind::Union => ExpKind::AnySetUnion(lhs, rhs),
                    vir::CollectionBinOpKind::Intersection => ExpKind::AnySetIntersection(lhs, rhs),
                    vir::CollectionBinOpKind::Difference => ExpKind::AnySetMinus(lhs, rhs),
                    vir::CollectionBinOpKind::Subset => ExpKind::AnySetSubset(lhs, rhs),
                    vir::CollectionBinOpKind::Concat => ExpKind::SeqAppend(lhs, rhs),
                    vir::CollectionBinOpKind::Take => ExpKind::SeqTake(lhs, rhs),
                    vir::CollectionBinOpKind::Drop => ExpKind::SeqDrop(lhs, rhs),
                }
            }
            vir::ExprKindData::CollectionLiteral(literal) => {
                let values = literal
                    .values
                    .iter()
                    .map(|value| self.expression(value))
                    .collect::<Result<Vec<_>, _>>()?;
                if values.is_empty() {
                    match literal.ty.kind() {
                        vir::TypeKind::Set(t) => ExpKind::EmptySet(self.convert(t)?),
                        vir::TypeKind::Multiset(t) => ExpKind::EmptyMultiset(self.convert(t)?),
                        vir::TypeKind::Seq(t) => ExpKind::EmptySeq(self.convert(t)?),
                        vir::TypeKind::Map(k, v) => {
                            ExpKind::EmptyMap(self.convert(k)?, self.convert(v)?)
                        }
                        _ => return Err(ConversionError::UnsupportedExpression),
                    }
                } else {
                    match literal.ty.kind() {
                        vir::TypeKind::Set(_) => ExpKind::ExplicitSet(values),
                        vir::TypeKind::Multiset(_) => ExpKind::ExplicitMultiset(values),
                        vir::TypeKind::Seq(_) => ExpKind::ExplicitSeq(values),
                        _ => return Err(ConversionError::UnsupportedExpression),
                    }
                }
            }
            vir::ExprKindData::CollectionUpdate(update) => {
                let target = self.expression(&update.target)?;
                let key = self.expression(&update.key)?;
                let value = self.expression(&update.val)?;
                match update.target.ty().kind() {
                    vir::TypeKind::Seq(_) => ExpKind::SeqUpdate(target, key, value),
                    vir::TypeKind::Map(..) => ExpKind::MapUpdate(target, key, value),
                    _ => return Err(ConversionError::UnsupportedExpression),
                }
            }
            vir::ExprKindData::CollectionLen(value) => match value.ty().kind() {
                vir::TypeKind::Set(_) | vir::TypeKind::Multiset(_) => {
                    ExpKind::AnySetCardinality(self.expression(&value)?)
                }
                vir::TypeKind::Seq(_) => ExpKind::SeqLength(self.expression(&value)?),
                vir::TypeKind::Map(..) => ExpKind::MapCardinality(self.expression(&value)?),
                _ => return Err(ConversionError::UnsupportedExpression),
            },
            vir::ExprKindData::MapDomain(value) => ExpKind::MapDomain(self.expression(&value)?),
            vir::ExprKindData::MapRange(value) => ExpKind::MapRange(self.expression(&value)?),
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
        Ok(Exp::with_extra(kind, meta, extra))
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
    #[ignore = "requires real Z3; set FERRUM_TEST_Z3"]
    fn vir_program_reaches_validated_native_verifier() {
        let z3 = std::env::var_os("FERRUM_TEST_Z3").expect("set FERRUM_TEST_Z3");
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
    #[ignore = "requires real Z3; set FERRUM_TEST_Z3"]
    fn vir_collection_membership_reaches_native_verifier() {
        let z3 = std::env::var_os("FERRUM_TEST_Z3").expect("set FERRUM_TEST_Z3");
        std::thread::Builder::new()
            .stack_size(ferrum::VERIFICATION_STACK_SIZE)
            .spawn(move || {
                vir::init_vcx(vir::VirCtxt::new_without_tcx());
                let program = vir::with_vcx(|vcx| {
                    let one = vcx.mk_int::<1>();
                    let set = vcx.mk_set_literal_expr(vcx.alloc_slice(&[one]), one.ty());
                    let expr = vcx.mk_set_in_expr(one, set);
                    let assertion = vcx.mk_assert_stmt(expr);
                    let label = vcx.alloc(vir::CfgBlockLabelData::End);
                    let end = vcx.alloc(vir::TerminatorStmtData::Exit);
                    let block = vcx.mk_cfg_block(label, &[], vcx.alloc_slice(&[assertion]), end);
                    let method = vcx.mk_method_unchecked(
                        "test",
                        &[],
                        &[],
                        &[],
                        &[],
                        Some(vcx.alloc_slice(&[block])),
                    );
                    program_to_silver(vcx.mk_program(
                        &[],
                        &[],
                        &[],
                        &[],
                        &[],
                        vcx.alloc_slice(&[method]),
                    ))
                    .unwrap()
                });
                let mut frontend = native_frontend(
                    &["--z3Exe".into(), z3.to_string_lossy().into_owned()],
                    Arc::new(NoopReporter),
                )
                .unwrap();
                assert!(matches!(
                    frontend.verify(&program).unwrap(),
                    VerificationResult::Success
                ));
                frontend.stop().unwrap();
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn function_decreases_is_retained_in_postconditions() {
        use vir::CastType;
        vir::init_vcx(vir::VirCtxt::new_without_tcx());
        vir::with_vcx(|vcx| {
            let value = vcx.mk_int::<1>();
            let decreases = vcx.alloc(vir::DecreasesGenData::Tuple(
                vcx.alloc_slice(&[value.as_dyn()]),
                None,
            ));
            let function = vcx.alloc(vir::FunctionData {
                name: "f",
                args: &[],
                ret: value.ty().as_dyn(),
                pres: &[],
                posts: &[],
                decreases,
                expr: None,
            });
            let program = program_to_silver(vcx.mk_program(
                &[],
                &[],
                &[],
                &[],
                vcx.alloc_slice(&[function]),
                &[],
            ))
            .unwrap();
            let ferrum_silver::ast::expression::ExpKind::Extension(extension) =
                program.functions[0].posts[0].kind()
            else {
                panic!("expected decreases clause");
            };
            let clause = extension.0.as_any().downcast_ref::<ferrum_silver::plugin::standard::termination::termination_ast_extension::DecreasesClause>().unwrap();
            assert!(
                matches!(&clause.kind, ferrum_silver::plugin::standard::termination::termination_ast_extension::DecreasesKind::Tuple { tuple,.. } if tuple.len()==1)
            );
        });
    }

    #[test]
    fn duplicate_domain_axiom_names_use_last_owner_like_jni_adapter() {
        vir::init_vcx(vir::VirCtxt::new_without_tcx());
        vir::with_vcx(|vcx| {
            let a = vcx.alloc(vir::DomainAxiomData {
                name: "same",
                expr: vcx.mk_bool::<true>(),
            });
            let b = vcx.alloc(vir::DomainAxiomData {
                name: "same",
                expr: vcx.mk_bool::<true>(),
            });
            let first = vcx.alloc(vir::DomainData {
                name: "First",
                typarams: &[],
                axioms: vcx.alloc_slice(&[a]),
                functions: &[],
                interpretation: None,
            });
            let second = vcx.alloc(vir::DomainData {
                name: "Second",
                typarams: &[],
                axioms: vcx.alloc_slice(&[b]),
                functions: &[],
                interpretation: None,
            });
            let program = program_to_silver(vcx.mk_program(
                &[],
                &[],
                vcx.alloc_slice(&[first, second]),
                &[],
                &[],
                &[],
            ))
            .unwrap();
            let ferrum_silver::ast::program::DomainAxiomKind::Named { domain_name, .. } =
                program.domains[0].axioms[0].kind()
            else {
                panic!("expected named axiom");
            };
            assert_eq!(domain_name, "Second");
        });
    }

    #[test]
    fn malformed_domain_arity_is_rejected() {
        vir::init_vcx(vir::VirCtxt::new_without_tcx());
        vir::with_vcx(|vcx| {
            let domain = vcx.alloc(vir::DomainData {
                name: "D",
                typarams: &[],
                axioms: &[],
                functions: &[],
                interpretation: None,
            });
            let program = vcx.mk_program(&[], &[], vcx.alloc_slice(&[domain]), &[], &[], &[]);
            let context = TypeContext::new(program);
            let arg = vcx.alloc(vir::TypeData::<vir::Dyn>::new(vir::TypeKind::Int));
            let bad = vcx.alloc(vir::TypeData::<vir::Dyn>::new(vir::TypeKind::Domain(
                "D",
                vcx.alloc_slice(&[arg]),
            )));
            assert!(matches!(
                context.convert(&bad),
                Err(ConversionError::DomainTypeRequiresDeclarations(_))
            ));
        });
    }

    #[test]
    #[ignore = "requires real Z3; set FERRUM_TEST_Z3"]
    fn vir_domain_axiom_reaches_native_verifier() {
        let z3 = std::env::var_os("FERRUM_TEST_Z3").expect("set FERRUM_TEST_Z3");
        std::thread::Builder::new()
            .stack_size(ferrum::VERIFICATION_STACK_SIZE)
            .spawn(move || {
                vir::init_vcx(vir::VirCtxt::new_without_tcx());
                let program = vir::with_vcx(|vcx| {
                    let axiom = vcx.alloc(vir::DomainAxiomData {
                        name: "trivial",
                        expr: vcx.mk_bool::<true>(),
                    });
                    let domain = vcx.alloc(vir::DomainData {
                        name: "D",
                        typarams: &[],
                        axioms: vcx.alloc_slice(&[axiom]),
                        functions: &[],
                        interpretation: None,
                    });
                    let assertion = vcx.mk_assert_stmt(vcx.mk_bool::<true>());
                    let label = vcx.alloc(vir::CfgBlockLabelData::End);
                    let end = vcx.alloc(vir::TerminatorStmtData::Exit);
                    let block = vcx.mk_cfg_block(label, &[], vcx.alloc_slice(&[assertion]), end);
                    let method = vcx.mk_method_unchecked(
                        "test",
                        &[],
                        &[],
                        &[],
                        &[],
                        Some(vcx.alloc_slice(&[block])),
                    );
                    program_to_silver(vcx.mk_program(
                        &[],
                        &[],
                        vcx.alloc_slice(&[domain]),
                        &[],
                        &[],
                        vcx.alloc_slice(&[method]),
                    ))
                    .unwrap()
                });
                let mut frontend = native_frontend(
                    &["--z3Exe".into(), z3.to_string_lossy().into_owned()],
                    Arc::new(NoopReporter),
                )
                .unwrap();
                assert!(matches!(
                    frontend.verify(&program).unwrap(),
                    VerificationResult::Success
                ));
                frontend.stop().unwrap();
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    #[ignore = "requires real Z3; set FERRUM_TEST_Z3"]
    fn vir_adt_reaches_native_frontend() {
        let z3 = std::env::var_os("FERRUM_TEST_Z3").expect("set FERRUM_TEST_Z3");
        std::thread::Builder::new()
            .stack_size(ferrum::VERIFICATION_STACK_SIZE)
            .spawn(move || {
                vir::init_vcx(vir::VirCtxt::new_without_tcx());
                let program = vir::with_vcx(|vcx| {
                    let constructor = vcx.alloc(vir::AdtConstructorData {
                        name: "Just",
                        args: &[],
                        axiom: None,
                    });
                    let adt = vcx.alloc(vir::AdtData {
                        name: "Box",
                        typarams: &[],
                        constructors: vcx.alloc_slice(&[constructor]),
                    });
                    let assertion = vcx.mk_assert_stmt(vcx.mk_bool::<true>());
                    let label = vcx.alloc(vir::CfgBlockLabelData::End);
                    let end = vcx.alloc(vir::TerminatorStmtData::Exit);
                    let block = vcx.mk_cfg_block(label, &[], vcx.alloc_slice(&[assertion]), end);
                    let method = vcx.mk_method_unchecked(
                        "test",
                        &[],
                        &[],
                        &[],
                        &[],
                        Some(vcx.alloc_slice(&[block])),
                    );
                    program_to_silver(vcx.mk_program(
                        &[],
                        vcx.alloc_slice(&[adt]),
                        &[],
                        &[],
                        &[],
                        vcx.alloc_slice(&[method]),
                    ))
                    .unwrap()
                });
                let mut frontend = native_frontend(
                    &["--z3Exe".into(), z3.to_string_lossy().into_owned()],
                    Arc::new(NoopReporter),
                )
                .unwrap();
                assert!(matches!(
                    frontend.verify(&program).unwrap(),
                    VerificationResult::Success
                ));
                frontend.stop().unwrap();
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    #[ignore = "requires real Z3; set FERRUM_TEST_Z3"]
    fn vir_wand_package_and_apply_reach_native_verifier() {
        let z3 = std::env::var_os("FERRUM_TEST_Z3").expect("set FERRUM_TEST_Z3");
        std::thread::Builder::new()
            .stack_size(ferrum::VERIFICATION_STACK_SIZE)
            .spawn(move || {
                vir::init_vcx(vir::VirCtxt::new_without_tcx());
                let program = vir::with_vcx(|vcx| {
                    let wand = vcx.mk_wand(vcx.mk_bool::<true>(), vcx.mk_bool::<true>());
                    let package = vcx.mk_package_stmt(wand, &[]);
                    let apply = vcx.mk_apply_stmt(wand);
                    let label = vcx.alloc(vir::CfgBlockLabelData::End);
                    let end = vcx.alloc(vir::TerminatorStmtData::Exit);
                    let block =
                        vcx.mk_cfg_block(label, &[], vcx.alloc_slice(&[package, apply]), end);
                    let method = vcx.mk_method_unchecked(
                        "test",
                        &[],
                        &[],
                        &[],
                        &[],
                        Some(vcx.alloc_slice(&[block])),
                    );
                    program_to_silver(vcx.mk_program(
                        &[],
                        &[],
                        &[],
                        &[],
                        &[],
                        vcx.alloc_slice(&[method]),
                    ))
                    .unwrap()
                });
                let mut frontend = native_frontend(
                    &["--z3Exe".into(), z3.to_string_lossy().into_owned()],
                    Arc::new(NoopReporter),
                )
                .unwrap();
                let result = frontend.verify(&program).unwrap();
                assert!(matches!(result, VerificationResult::Success), "{result:?}");
                frontend.stop().unwrap();
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    #[ignore = "requires real Z3; set FERRUM_TEST_Z3"]
    fn native_mapped_counterexample_reaches_prusti_model() {
        let z3 = std::env::var_os("FERRUM_TEST_Z3").expect("set FERRUM_TEST_Z3");
        std::thread::Builder::new()
            .stack_size(ferrum::VERIFICATION_STACK_SIZE)
            .spawn(move || {
                let variable = ferrum_silver::ast::program::LocalVarDecl::new(
                    "x".into(),
                    Type::Int,
                    Metadata::default(),
                );
                let x = Exp::new(ExpKind::LocalVar(variable.local_var()), Metadata::default());
                let zero = Exp::new(ExpKind::IntLit(0.into()), Metadata::default());
                let assertion = Stmt::new(
                    StmtKind::Assert(Exp::new(ExpKind::GtCmp(x, zero), Metadata::default())),
                    Metadata::default(),
                );
                let method = Method::new(MethodData {
                    name: "bad".into(),
                    formal_args: vec![variable],
                    formal_returns: vec![],
                    pres: vec![],
                    posts: vec![],
                    body: Some(assertion),
                    meta: Metadata::default(),
                });
                let program = Program::new(
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                    vec![method],
                    vec![],
                    Metadata::default(),
                );
                let mut frontend = native_frontend(
                    &[
                        "--z3Exe".into(),
                        z3.to_string_lossy().into_owned(),
                        "--counterexample".into(),
                        "mapped".into(),
                        "--enableMoreCompleteExhale".into(),
                    ],
                    Arc::new(NoopReporter),
                )
                .unwrap();
                let VerificationResult::Failure(errors) = frontend.verify(&program).unwrap() else {
                    panic!("expected failure");
                };
                let ferrum_silver::verifier::verification_result::AbstractError::Verification(
                    error,
                ) = &errors[0]
                else {
                    panic!("expected verification error");
                };
                let model = counterexample::from_diagnostic(error)
                    .unwrap()
                    .expect("mapped counterexample");
                assert!(
                    model.model.entries.contains_key("x"),
                    "{:?}",
                    model.model.entries
                );
                frontend.stop().unwrap();
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    #[ignore = "requires real Z3; set FERRUM_TEST_Z3"]
    fn native_mapped_counterexample_keeps_raw_domain_local() {
        use ferrum_silver::ast::{
            program::{Domain, DomainData, LocalVarDecl},
            r#type::DomainType,
        };
        let z3 = std::env::var_os("FERRUM_TEST_Z3").expect("set FERRUM_TEST_Z3");
        std::thread::Builder::new()
            .stack_size(ferrum::VERIFICATION_STACK_SIZE)
            .spawn(move || {
                let meta = Metadata::default();
                let domain = Domain::new(DomainData {
                    name: "D".into(),
                    functions: vec![],
                    axioms: vec![],
                    type_vars: vec![],
                    interpretations: None,
                    meta: meta.clone(),
                });
                let x = LocalVarDecl::new(
                    "x".into(),
                    Type::Domain(DomainType::new("D", Default::default(), vec![])),
                    meta.clone(),
                );
                let var = Exp::new(ExpKind::LocalVar(x.local_var()), meta.clone());
                let neq = Exp::new(ExpKind::NeCmp(var.clone(), var), meta.clone());
                let method = Method::new(MethodData {
                    name: "bad".into(),
                    formal_args: vec![x],
                    formal_returns: vec![],
                    pres: vec![],
                    posts: vec![],
                    body: Some(Stmt::new(StmtKind::Assert(neq), meta.clone())),
                    meta: meta.clone(),
                });
                let program = Program::new(
                    vec![domain],
                    vec![],
                    vec![],
                    vec![],
                    vec![method],
                    vec![],
                    meta,
                );
                let mut frontend = native_frontend(
                    &[
                        "--z3Exe".into(),
                        z3.to_string_lossy().into_owned(),
                        "--counterexample".into(),
                        "mapped".into(),
                        "--enableMoreCompleteExhale".into(),
                    ],
                    Arc::new(NoopReporter),
                )
                .unwrap();
                let VerificationResult::Failure(errors) = frontend.verify(&program).unwrap() else {
                    panic!("expected failure")
                };
                let ferrum_silver::verifier::verification_result::AbstractError::Verification(
                    error,
                ) = &errors[0]
                else {
                    panic!("expected verification error")
                };
                let model = counterexample::from_diagnostic(error).unwrap().unwrap();
                assert!(
                    matches!(
                        model.model.entries.get("x"),
                        Some(prusti_backend_types::ModelEntry::DomainValue(_, _))
                    ),
                    "{:?}",
                    model.model.entries
                );
                frontend.stop().unwrap();
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn duplicate_method_requires_silver_transitive_check() {
        use ferrum_silver::ast::{
            pretty::pretty_printer::{pretty_node, PrintError},
            utility::{consistency::check_transitively, nodes::Node},
        };
        let method = Method::new(MethodData {
            name: "duplicate".into(),
            formal_args: vec![],
            formal_returns: vec![],
            pres: vec![],
            posts: vec![],
            body: None,
            meta: Metadata::default(),
        });
        let program = Program::new(
            vec![],
            vec![],
            vec![],
            vec![],
            vec![method.clone(), method],
            vec![],
            Metadata::default(),
        );
        let backend = ferrum_silicon::utils::consistency::check(&program, &|_| {
            unreachable!("no unsupported permission")
        })
        .unwrap();
        assert!(backend.is_empty());
        let silver = check_transitively(&program, &pretty_node, &|_: &[Node]| -> Result<
            String,
            PrintError,
        > {
            Err(PrintError::Unsupported("sequence"))
        })
        .unwrap();
        assert!(
            !silver.is_empty(),
            "Silver must reject duplicate declarations"
        );
    }

    #[test]
    #[ignore = "requires real Z3; set FERRUM_TEST_Z3"]
    fn native_error_preserves_identifier_positions() {
        let z3 = std::env::var_os("FERRUM_TEST_Z3").expect("set FERRUM_TEST_Z3");
        std::thread::Builder::new()
            .stack_size(ferrum::VERIFICATION_STACK_SIZE)
            .spawn(move || {
                let id = |value: &str| {
                    Position::LineColumn(LineColumn::Identifier(Box::new(IdentifierPosition {
                        file: None,
                        start: Box::new(LineColumnPosition::new(0, 0).into()),
                        end: None,
                        id: value.into(),
                    })))
                };
                let exp = Exp::new(
                    ExpKind::FalseLit,
                    Metadata {
                        position: id("456"),
                        ..Default::default()
                    },
                );
                let statement = Stmt::new(
                    StmtKind::Assert(exp),
                    Metadata {
                        position: id("123"),
                        ..Default::default()
                    },
                );
                let method = Method::new(MethodData {
                    name: "bad".into(),
                    formal_args: vec![],
                    formal_returns: vec![],
                    pres: vec![],
                    posts: vec![],
                    body: Some(statement),
                    meta: Metadata::default(),
                });
                let program = Program::new(
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                    vec![method],
                    vec![],
                    Metadata::default(),
                );
                let mut frontend = native_frontend(
                    &["--z3Exe".into(), z3.to_string_lossy().into_owned()],
                    Arc::new(NoopReporter),
                )
                .unwrap();
                let VerificationResult::Failure(errors) = frontend.verify(&program).unwrap() else {
                    panic!("expected failure");
                };
                let ferrum_silver::verifier::verification_result::AbstractError::Verification(
                    error,
                ) = &errors[0]
                else {
                    panic!("expected verification error");
                };
                let error = verification_error(error.clone()).unwrap();
                assert_eq!(error.offending_pos_id.as_deref(), Some("123"));
                assert_eq!(error.reason_pos_id.as_deref(), Some("456"));
                assert!(error.full_id.starts_with("assert.failed"));
                frontend.stop().unwrap();
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn failed_frontend_start_retains_caller_owned_warnings() {
        use std::os::unix::fs::PermissionsExt;
        let path =
            std::env::temp_dir().join(format!("prusti-ferrum-z3-fail-{}", std::process::id()));
        std::fs::write(
            &path,
            "#!/bin/sh\nIFS= read -r first_command || exit 1\nprintf 'WARNING failing-startup-warning\\ninvalid-response\\n'\ncat >/dev/null\n",
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let warnings = Arc::new(std::sync::Mutex::new(Vec::new()));
        let result = native_frontend_with_warning_sink(
            &["--z3Exe".into(), path.to_string_lossy().into_owned()],
            Arc::new(NoopReporter),
            warnings.clone(),
        );
        std::fs::remove_file(path).unwrap();
        assert!(result.is_err());
        assert!(warnings
            .lock()
            .unwrap()
            .iter()
            .any(|s| s.contains("failing-startup-warning")));
    }

    #[test]
    #[ignore = "requires real Z3; set FERRUM_TEST_Z3"]
    fn constructed_ast_uses_native_frontend_validation_and_verifier() {
        let z3 = std::env::var_os("FERRUM_TEST_Z3").expect("set FERRUM_TEST_Z3");
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
            vir::StmtKindData::Label(label) => Stmt::new(
                StmtKind::Label(self.label((*label).into(), vec![])),
                Metadata::default(),
            ),
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
            vir::StmtKindData::Fold(pred) | vir::StmtKindData::Unfold(pred) => {
                let args = pred
                    .args
                    .iter()
                    .map(|arg| self.expression(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                let m = Metadata::default();
                let access = ferrum_silver::ast::expression::PredicateAccess::new(
                    args,
                    pred.target.into(),
                    m.clone(),
                );
                let predicate = ferrum_silver::ast::statement::PredicateAccessPredicate::new(
                    access,
                    pred.perm.map(|p| self.expression(&p)).transpose()?,
                    m,
                );
                let kind = if matches!(statement.kind, vir::StmtKindData::Fold(_)) {
                    StmtKind::Fold(predicate)
                } else {
                    StmtKind::Unfold(predicate)
                };
                Stmt::new(kind, meta)
            }
            vir::StmtKindData::Refute(exp) => {
                ferrum_silver::plugin::standard::refute::refute_ast_extension::Refute::new(
                    self.expression(&exp)?,
                    meta,
                )
                .statement()
            }
            vir::StmtKindData::Apply(wand) => {
                let wand = ferrum_silver::ast::expression::Exp::new(
                    ExpKind::MagicWand(self.expression(&wand.lhs)?, self.expression(&wand.rhs)?),
                    Metadata::default(),
                );
                Stmt::new(StmtKind::Apply(wand), meta)
            }
            vir::StmtKindData::Package(wand, proof) => {
                let wand = ferrum_silver::ast::expression::Exp::new(
                    ExpKind::MagicWand(self.expression(&wand.lhs)?, self.expression(&wand.rhs)?),
                    Metadata::default(),
                );
                let mut nested = Vec::new();
                let statements = proof
                    .iter()
                    .map(|stmt| self.statement(stmt, &mut nested))
                    .collect::<Result<Vec<_>, _>>()?;
                let proof_script = Stmt::new(
                    StmtKind::Seqn {
                        statements,
                        declarations: nested,
                    },
                    Metadata::default(),
                );
                Stmt::new(StmtKind::Package { wand, proof_script }, meta)
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
                    _ => {
                        return Err(ConversionError::UnsupportedStatement(format!(
                            "{:?}",
                            statement.kind
                        )))
                    }
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
            _ => {
                return Err(ConversionError::UnsupportedStatement(format!(
                    "{:?}",
                    statement.kind
                )))
            }
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
            vir::TerminatorStmtData::Dummy(text) => vec![
                seq(vec![
                    self.comment((*text).into()),
                    Stmt::new(
                        StmtKind::Assert(Exp::new(ExpKind::FalseLit, meta.clone())),
                        meta.clone(),
                    ),
                ]),
                goto(vir::CfgBlockLabelData::End.name()),
            ],
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
    let context = TypeContext::new(program);
    let extensions = program
        .adts
        .iter()
        .map(|adt| {
            use ferrum_silver::plugin::standard::adt::adt_ast_extension::{Adt, AdtConstructor};
            let type_vars = adt
                .typarams
                .iter()
                .map(|p| TypeVar::new(p.name))
                .collect::<Vec<_>>();
            let constructors = adt
                .constructors
                .iter()
                .map(|constructor| {
                    let args = constructor
                        .args
                        .iter()
                        .map(|arg| context.local_declaration(arg))
                        .collect::<Result<_, _>>()?;
                    Ok(Arc::new(AdtConstructor::new(
                        adt.name,
                        type_vars.clone(),
                        constructor.name,
                        args,
                        Metadata::default(),
                    )))
                })
                .collect::<Result<Vec<_>, ConversionError>>()?;
            Ok(ferrum_silver::ast::program::ExtensionMemberHandle(
                Arc::new(Adt {
                    name: adt.name.into(),
                    constructors,
                    type_vars,
                    deriving_info: Default::default(),
                    meta: Metadata::default(),
                }),
            ))
        })
        .collect::<Result<Vec<_>, ConversionError>>()?;
    let domains = program
        .domains
        .iter()
        .map(|domain| {
            use ferrum_silver::ast::program::{
                AnyLocalVarDecl, Domain, DomainAxiom, DomainAxiomKind, DomainData, DomainFunc,
                DomainFuncData, LocalVarDecl,
            };
            let functions = domain
                .functions
                .iter()
                .map(|function| {
                    let args = function
                        .args
                        .iter()
                        .enumerate()
                        .map(|(index, ty)| {
                            Ok(AnyLocalVarDecl::Named(LocalVarDecl::new(
                                format!("arg{index}"),
                                context.convert(ty)?,
                                Metadata::default(),
                            )))
                        })
                        .collect::<Result<_, ConversionError>>()?;
                    Ok(DomainFunc::new(DomainFuncData {
                        name: function.name.to_str().into(),
                        formal_args: args,
                        typ: context.convert(&function.ret)?,
                        unique: function.unique,
                        interpretation: function
                            .interpretation
                            .as_ref()
                            .map(|i| i.interpretation.into()),
                        domain_name: context
                            .domain_functions
                            .get(function.name.to_str())
                            .ok_or(ConversionError::UnsupportedProgramMember(
                                "domain function owner",
                            ))?
                            .0
                            .name
                            .into(),
                        meta: Metadata::default(),
                    }))
                })
                .collect::<Result<_, ConversionError>>()?;
            let axioms = domain
                .axioms
                .iter()
                .map(|axiom| {
                    Ok(DomainAxiom::new(DomainAxiomKind::Named {
                        name: axiom.name.into(),
                        exp: context.expression(&axiom.expr)?,
                        domain_name: context
                            .domain_axioms
                            .get(axiom.name)
                            .ok_or(ConversionError::UnsupportedProgramMember(
                                "domain axiom owner",
                            ))?
                            .name
                            .into(),
                        meta: Metadata::default(),
                    }))
                })
                .collect::<Result<_, ConversionError>>()?;
            Ok(Domain::new(DomainData {
                name: domain.name.into(),
                functions,
                axioms,
                type_vars: domain
                    .typarams
                    .iter()
                    .map(|p| TypeVar::new(p.name))
                    .collect(),
                interpretations: domain.interpretation.map(|i| {
                    i.interpretation
                        .iter()
                        .map(|p| (p.key.into(), p.value.into()))
                        .collect()
                }),
                meta: Metadata::default(),
            }))
        })
        .collect::<Result<Vec<_>, ConversionError>>()?;
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
            use ferrum_silver::plugin::standard::termination::termination_ast_extension::{
                DecreasesClause, DecreasesKind,
            };
            let mut posts = function
                .posts
                .iter()
                .map(|e| context.expression(e))
                .collect::<Result<Vec<_>, _>>()?;
            let decreases = match function.decreases {
                vir::DecreasesGenData::None => None,
                vir::DecreasesGenData::Tuple(values, condition) => Some(DecreasesKind::Tuple {
                    tuple: values
                        .iter()
                        .map(|e| context.expression(e))
                        .collect::<Result<_, _>>()?,
                    condition: condition.map(|e| context.expression(&e)).transpose()?,
                }),
                vir::DecreasesGenData::Wildcard(condition) => Some(DecreasesKind::Wildcard {
                    condition: condition.map(|e| context.expression(&e)).transpose()?,
                }),
                vir::DecreasesGenData::Star => Some(DecreasesKind::Star),
            };
            if let Some(kind) = decreases {
                posts.push(DecreasesClause::new(kind, Metadata::default()).expression());
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
                    posts,
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
        domains,
        fields,
        functions,
        predicates,
        methods,
        extensions,
        Metadata::default(),
    ))
}

/// Identifier payloads have the same meaning as the identifiers supplied to
/// the JVM adapter; ordinary source positions do not carry a Prusti span ID.
pub fn position_id(pos: &Position) -> Option<String> {
    match pos {
        Position::LineColumn(LineColumn::Identifier(p)) => Some(p.id.clone()),
        _ => None,
    }
}

/// Native diagnostic data required by Prusti's existing span backtranslation.
/// Retain the original diagnostic so later counterexample mapping has access to
/// its typed payload instead of silently dropping it.
#[derive(Clone, Debug)]
pub struct NativeVerificationError {
    pub full_id: String,
    pub pos_id: Option<String>,
    pub offending_pos_id: Option<String>,
    pub reason_pos_id: Option<String>,
    pub message: String,
    pub diagnostic: Arc<dyn ferrum_silver::verifier::verification_result::VerificationDiagnostic>,
}

pub fn verification_error(
    error: Arc<dyn ferrum_silver::verifier::verification_result::VerificationDiagnostic>,
) -> Result<NativeVerificationError, ferrum_silver::verifier::verification_result::DiagnosticError>
{
    use ferrum_silver::verifier::verification_error::node_position;
    let offending_pos = node_position(&error.offending_node()?)?;
    let reason_pos = error.reason()?.position()?;
    Ok(NativeVerificationError {
        full_id: error.full_id(),
        pos_id: position_id(&error.position()),
        offending_pos_id: position_id(&offending_pos),
        reason_pos_id: position_id(&reason_pos),
        message: error.readable_message()?,
        diagnostic: error,
    })
}
