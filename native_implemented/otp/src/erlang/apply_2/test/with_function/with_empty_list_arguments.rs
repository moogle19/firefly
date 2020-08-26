use super::*;

#[test]
fn without_arity_errors_badarity() {
    run!(
        |arc_process| {
            (
                Just(arc_process.clone()),
                strategy::term::export_closure_non_zero_arity_range_inclusive(),
            )
        },
        |(arc_process, arity)| {
            let module = Atom::from_str("module");
            let function = Atom::from_str("function");
            let function =
                strategy::term::export_closure(&arc_process.clone(), module, function, arity);
            let result = result(&arc_process, function, Term::NIL);

            prop_assert_badarity!(
                result,
                &arc_process,
                function,
                Term::NIL,
                format!(
                    "arguments ([]) length (0) does not match arity ({}) of function ({})",
                    arity, function
                )
            );

            Ok(())
        },
    );
}

// `with_arity_returns_function_return` in integration tests
