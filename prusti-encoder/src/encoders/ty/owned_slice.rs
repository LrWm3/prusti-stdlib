//! Metadata for an owned logical slice embedded in a sized collection.
use super::{
    RustTyDecomposition, RustTyNormalized, RustTySpecial,
    generics::{GArgCaster, GArgsCastEnc},
    use_pure::{TyUsePure, TyUsePureEnc, TyUsePureField},
};
use crate::encoders::Pure;
use task_encoder::{EncodeFullError, TaskEncoder, TaskEncoderDependencies};
use vir::CastType;

fn field_indices(owner: RustTyDecomposition<'_>) -> Option<(usize, usize)> {
    match owner.ty.special {
        RustTySpecial::Vec => Some((1, 2)),
        RustTySpecial::VecIntoIter => {
            let len = owner.ty.expect_structlike().fields.len();
            Some((len - 2, len - 1))
        }
        _ => None,
    }
}

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
    length_index: usize,
    elements_index: usize,
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
        let Some((length_index, elements_index)) = field_indices(owner) else {
            return Ok(None);
        };
        let super::TySpecifics::StructLike(data) = &owner.ty.specifics else {
            unreachable!()
        };
        let slice = data.fields[elements_index]
            .ty()
            .decompose_normalize(owner.args);
        let usize = vir::with_vcx(|vcx| RustTyDecomposition::from_prim_ty(vcx.tcx().types.usize));
        Ok(Some(Self {
            length_index,
            elements_index,
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
            .ref_to_snap(fields[self.length_index].field_ref(storage));
        let extent = self
            .usize_pure
            .expect_primitive()
            .snap_to_prim(length.downcast_ty())
            .downcast_ty();
        let mut statements = self.physical.unfold(None, storage, None, None, None);
        statements.push(
            self.typing
                .establish_extent(fields[self.elements_index].field_ref(storage), extent),
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
        let Some((length_index, elements_index)) = field_indices(owner) else {
            return Ok(None);
        };
        let pure = deps.require_dep::<TyUsePureEnc>(owner)?;
        let fields = &pure.expect_structlike().fields;
        let usize = vir::with_vcx(|vcx| RustTyDecomposition::from_prim_ty(vcx.tcx().types.usize));
        Ok(Some(Self {
            index: elements_index,
            length: fields[length_index],
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
