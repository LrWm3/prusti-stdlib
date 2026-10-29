use viper::*;

lazy_static::lazy_static! {
    static ref VIPER: Viper = Viper::new_for_tests();
}

#[test]
fn opaque_functions_require_reveal_and_still_check_their_bodies() {
    let context = VIPER.attach_current_thread();
    let ast = context.new_ast_factory();
    let pos = ast.no_position();
    let hidden = ast.function_with_opacity(
        "hidden",
        &[],
        ast.int_type(),
        &[],
        &[],
        pos,
        Some(ast.int_lit(1)),
        true,
    );
    let prove_value = |reveal| {
        ast.method(
            "prove_value",
            &[],
            &[],
            &[],
            &[],
            Some(ast.seqn(
                &[ast.assert(
                    ast.eq_cmp(
                        ast.func_app_with_reveal("hidden", &[], ast.int_type(), pos, reveal),
                        ast.int_lit(1),
                    ),
                    pos,
                )],
                &[],
            )),
        )
    };
    for reveal in [false, true] {
        let mut verifier = context.new_verifier_with_default_smt(VerificationBackend::Silicon);
        let result = verifier.verify(
            ast.program(&[], &[], &[hidden], &[], &[prove_value(reveal)], &[]),
            None,
        );
        if reveal {
            assert!(result.is_success(), "{result:?}");
        } else {
            assert!(
                matches!(result, VerificationResultKind::Failure(ref errors)
                if errors.len() == 1 && errors[0].full_id == "assert.failed:assertion.false"),
                "{result:?}"
            );
        }
    }
    let invalid = ast.function_with_opacity(
        "invalid",
        &[],
        ast.int_type(),
        &[],
        &[ast.eq_cmp(ast.result_with_pos(ast.int_type(), pos), ast.int_lit(2))],
        pos,
        Some(ast.int_lit(1)),
        true,
    );
    let mut verifier = context.new_verifier_with_default_smt(VerificationBackend::Silicon);
    let result = verifier.verify(ast.program(&[], &[], &[invalid], &[], &[], &[]), None);
    assert!(
        matches!(result, VerificationResultKind::Failure(ref errors)
        if errors.len() == 1 && errors[0].full_id == "postcondition.violated:assertion.false"),
        "{result:?}"
    );
}
