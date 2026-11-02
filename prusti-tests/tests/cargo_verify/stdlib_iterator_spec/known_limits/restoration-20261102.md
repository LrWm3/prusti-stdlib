# Unqualified restoration controls (2026-11-02)

The optional coercion controls isolate current-owner restoration from the
unmodeled array `IndexMut<RangeFull>` functional contract. Build 758 still
fails the caller read after clearing or rotating; the original RangeFull
controls remain failures too. Neither group is qualified.

The attempted conditional array contract in candidate 764 is not selected by
the existing call encoder. The bounds-negative is incorrectly accepted, so
this contract must not be adopted. Its custom-index test initially lacked
`refine_trait_spec`; this checkpoint corrects that test annotation but has
not rerun it. No corrected-test qualification is claimed.

The main plan remains `STDLIB_EXTERN_SPEC.md`. Finish existing restoration and
collection/ownership stabilization, qualify the combined result, then pause
before later features or automation scheduling. Ferrum is the routine backend;
Silicon is reserved for major sign-off and final combined qualification.


Candidate 782 passes 96 audited checks (optional coercion 7, existing optional 9,
owner histories 80), but the additional local repeated/independent controls in
785 expose incomplete entry-sequence selection. Both positives fail permission
checks; both false-value controls match their expected diagnostics.

Candidate 788 replaces that selection with the physical owner address preserved
by array-to-slice unsizing and composes retained histories newest first. Its own
build, three unit tests, six encodings and six full AST checks pass. The repeated
and independent positives pass natively; the complete native regression gate is
still running. The encoder snapshot remains a recovery checkpoint, not a main
integration or full-matrix qualification. The unselected array conditional
contract from 764 has been removed from the current candidate; its commit and
failing fixtures remain preserved.
