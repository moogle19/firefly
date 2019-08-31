use super::*;

use num_traits::Num;

#[test]
fn with_negative_without_big_integer_underflow_shifts_right_and_returns_big_integer() {
    with(|integer, process| {
        let shift = process.integer(-1).unwrap();

        let result = erlang::bsl_2(integer, shift, &process);

        assert!(result.is_ok());

        let shifted = result.unwrap();

        assert!(shifted.is_bigint());

        assert_eq!(
            shifted,
            process
                .integer(
                    <BigInt as Num>::from_str_radix(
                        "10110011100011110000111110000011111100000011111110000000111111110000000",
                        2
                    )
                    .unwrap()
                )
                .unwrap()
        );
    });
}

#[test]
fn with_negative_with_big_integer_underflow_without_small_integer_underflow_shifts_right_and_returns_small_integer(
) {
    with(|integer, process| {
        let shift = process.integer(-69).unwrap();

        let result = erlang::bsl_2(integer, shift, &process);

        assert!(result.is_ok());

        let shifted = result.unwrap();

        assert!(shifted.is_smallint());
        assert_eq!(shifted, process.integer(0b101).unwrap());
    });
}

#[test]
fn with_negative_with_underflow_returns_zero() {
    with(|integer, process| {
        let shift = process.integer(-74).unwrap();

        assert_eq!(
            erlang::bsl_2(integer, shift, &process),
            Ok(process.integer(0b0).unwrap())
        );
    });
}

#[test]
fn with_positive_returns_big_integer() {
    with(|integer, process| {
        let shift = process.integer(1).unwrap();

        let result = erlang::bsl_2(integer, shift, &process);

        assert!(result.is_ok());

        let shifted = result.unwrap();

        assert!(shifted.is_bigint());

        assert_eq!(
            shifted,
            process
                .integer(
                    <BigInt as Num>::from_str_radix(
                        "1011001110001111000011111000001111110000001111111000000011111111000000000",
                        2
                    )
                    .unwrap()
                )
                .unwrap()
        );
    });
}

fn with<F>(f: F)
where
    F: FnOnce(Term, &Process) -> (),
{
    with_process(|process| {
        let integer = process
            .integer(
                <BigInt as Num>::from_str_radix(
                    "101100111000111100001111100000111111000000111111100000001111111100000000",
                    2,
                )
                .unwrap(),
            )
            .unwrap();

        assert!(integer.is_bigint());

        f(integer, &process)
    })
}
