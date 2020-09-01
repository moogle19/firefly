test_stdout!(
    with_valid_arguments_when_run_exits_normal_and_parent_does_not_exit,
    "{child, sum, 3}\n{parent, alive, true}\n"
);
test_stdout_substrings!(
    without_valid_arguments_when_run_exits_and_parent_exits,
    vec![
        "exited with reason: function_clause",
        "{parent, exited, function_clause}"
    ]
);