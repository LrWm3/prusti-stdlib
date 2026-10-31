//! Metadata for an owned logical slice embedded in a sized collection.
use super::{
    RustTyDecomposition, RustTyNormalized, RustTySpecial,
    generics::{GArgCaster, GArgsCastEnc},
    use_pure::{TyUsePure, TyUsePureEnc, TyUsePureField},
};
use crate::encoders::Pure;
use task_encoder::{EncodeFullError, TaskEncoder, TaskEncoderDependencies};
use vir::CastType;

#[derive(Clone)]
pub(crate) struct OwnedSlice<'vir> {
    pub index: usize,
    length: TyUsePureField<'vir>,
    usize_ty: TyUsePure<'vir>,
    cast: GArgCaster<'vir, Pure>,
}

/// Recover element type tags from the collection's owned storage before
/// packing concrete referent permissions into generic element resources.
#[derive(Clone)]
pub(crate) struct OwnedSliceTyping<'vir> {
    physical: super::use_impure::TyUseImpure<'vir>,
    usize_physical: super::use_impure::TyUseImpure<'vir>,
    usize_pure: TyUsePure<'vir>,
    typing: super::array_typing::ArrayTyping<'vir>,
}

impl<'vir> OwnedSliceTyping<'vir> {
    pub fn new<E: TaskEncoder>(
        deps: &mut TaskEncoderDependencies<'vir, E>,
        owner: RustTyDecomposition<'vir>,
    ) -> Result<Option<Self>, EncodeFullError<'vir, E>> {
        if owner.ty.special != RustTySpecial::Vec {
            return Ok(None);
        }
        let super::TySpecifics::StructLike(data) = &owner.ty.specifics else {
            unreachable!()
        };
        let slice = data.fields[2].ty().decompose_normalize(owner.args);
        let usize = vir::with_vcx(|vcx| RustTyDecomposition::from_prim_ty(vcx.tcx().types.usize));
        Ok(Some(Self {
            physical: deps.require_dep::<super::use_impure::TyUseImpureEnc>(owner)?,
            usize_physical: deps.require_dep::<super::use_impure::TyUseImpureEnc>(usize)?,
            usize_pure: deps.require_dep::<TyUsePureEnc>(usize)?,
            typing: deps.require_dep::<super::array_typing::ArrayTypingEnc>(slice)?,
        }))
    }

    pub fn establish(&self, storage: vir::ExprRef<'vir>) -> Vec<vir::Stmt<'vir>> {
        let fields = &self.physical.expect_structlike().fields;
        let length = self
            .usize_physical
            .ref_to_snap(fields[1].field_ref(storage));
        let extent = self
            .usize_pure
            .expect_primitive()
            .snap_to_prim(length.downcast_ty())
            .downcast_ty();
        let mut statements = self.physical.unfold(None, storage, None, None, None);
        statements.push(
            self.typing
                .establish_extent(fields[2].field_ref(storage), extent),
        );
        statements.extend(self.physical.fold(None, storage, None, None, None));
        statements
    }
}

impl<'vir> OwnedSlice<'vir> {
    pub fn new<E: TaskEncoder>(
        deps: &mut TaskEncoderDependencies<'vir, E>,
        owner: RustTyDecomposition<'vir>,
    ) -> Result<Option<Self>, EncodeFullError<'vir, E>> {
        if owner.ty.special != RustTySpecial::Vec {
            return Ok(None);
        }
        let pure = deps.require_dep::<TyUsePureEnc>(owner)?;
        let fields = &pure.expect_structlike().fields;
        assert_eq!(fields.len(), 3);
        let usize = vir::with_vcx(|vcx| RustTyDecomposition::from_prim_ty(vcx.tcx().types.usize));
        Ok(Some(Self {
            index: 2,
            length: fields[1],
            usize_ty: deps.require_dep::<TyUsePureEnc>(usize)?,
            cast: deps.require_dep::<GArgsCastEnc<Pure>>(Some(RustTyNormalized {
                param: RustTyDecomposition::param(),
                concrete: usize,
            }))?,
        }))
    }

    pub fn metadata<Curr, Next>(
        &self,
        owner: vir::ExprGenSnap<'vir, Curr, Next>,
    ) -> vir::ExprGenPSnap<'vir, Curr, Next> {
        self.cast
            .cast_to_callee_ctx(self.length.read(owner.downcast_ty()))
            .downcast_ty()
    }

    pub fn extent<Curr, Next>(
        &self,
        owner: vir::ExprGenSnap<'vir, Curr, Next>,
    ) -> vir::ExprGenInt<'vir, Curr, Next> {
        self.usize_ty
            .expect_primitive()
            .snap_to_prim(self.length.read(owner.downcast_ty()).downcast_ty())
            .downcast_ty()
    }
}
